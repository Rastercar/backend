use crate::{
    config::app_config,
    queue::controller::dto::events::EmailSendingErrorEvent,
    queue::{self},
};
use aws_sdk_sesv2::{
    config::Region,
    error::SdkError,
    operation::send_email::{builders::SendEmailFluentBuilder, SendEmailError, SendEmailOutput},
    types::{Body, Content, Destination, EmailContent, Message, MessageTag},
    Client,
};
use governor::{
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota,
};
use handlebars::Handlebars;
use shared::dto::mailer::EmailRecipient;
use std::{num::NonZeroU32, sync::Arc, thread, time};
use tokio::task::JoinSet;
use tracing::{error, event, Instrument, Level};
use uuid::Uuid;

/// see: https://docs.aws.amazon.com/ses/latest/APIReference/API_SendEmail.html
static MAX_RECIPIENTS_PER_SEND_EMAIL_OP: usize = 50;

/// name of the tag containing the request uuid that will be published to the email
pub static MAIL_REQUEST_UUID_TAG_NAME: &str = "request_uuid";

static MAX_EMAIL_RETRY_ATTEMPT: u8 = 4;

static RETRY_ATTEMPTS_INTERVAL: u8 = 5;

#[derive(Debug)]
pub struct SendEmailOptions {
    pub to: Vec<EmailRecipient>,
    pub from: Option<String>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub reply_to_addresses: Option<Vec<String>>,

    /// Uuid of the email request, used to publish error/finished events when all the deliveries for the request finish
    pub uuid: Uuid,

    /// If the email to be sent should have tracking for (click, delivery, report, send and open events)
    /// this changes how the email is fired in the following ways:
    ///
    /// a call with the sendEmail op is sent to SES for every email address in the `to` field, so we can properly
    /// track events to the recipient level, this is slower and more expensive as this triggers SNS events.
    ///
    /// the configuration set used to fire the emails
    pub track_events: bool,
}

type RateLimiter =
    governor::RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

pub struct Mailer {
    pub mailer_rmq: Arc<queue::MailerRabbitmq>,
    pub aws_client: Client,
    pub rate_limiter: Arc<RateLimiter>,
    pub default_sender: String,
    pub aws_ses_tracking_config_set: String,
}

fn to_utf8_content(input: &str) -> Result<Content, aws_sdk_sesv2::error::BuildError> {
    Content::builder().data(input).charset("UTF-8").build()
}

#[tracing::instrument(skip(rate_limiter, send_email_op, server))]
async fn send_with_rate_limiter(
    rate_limiter: Arc<RateLimiter>,
    send_email_op: SendEmailFluentBuilder,
    request_uuid: uuid::Uuid,
    recipients: Vec<String>,
    server: Arc<queue::MailerRabbitmq>,
) -> Result<SendEmailOutput, SdkError<SendEmailError>> {
    rate_limiter.until_ready().await;

    let mut result = send_email_op.clone().send().await;
    let mut attempt = 1;

    while attempt < MAX_EMAIL_RETRY_ATTEMPT && result.is_err() {
        attempt += 1;

        thread::sleep(time::Duration::from_secs(RETRY_ATTEMPTS_INTERVAL.into()));

        error!("sendEmail SES error: {:#?}", result.unwrap());

        rate_limiter.until_ready().await;
        result = send_email_op.clone().send().await;
    }

    if let Err(ses_err) = result {
        let sending_err_event =
            EmailSendingErrorEvent::new(ses_err.to_string(), request_uuid, recipients);

        if let Err(publishing_err) = server.publish_event(sending_err_event).await {
            error!("failed to publish SES error to RMQ: {}", publishing_err);
        }

        return Err(ses_err);
    }

    result
}

impl Mailer {
    pub async fn new(mailer_rmq: Arc<queue::MailerRabbitmq>) -> Mailer {
        let cfg = app_config();

        let aws_cfg = aws_config::from_env()
            .region(Region::new(cfg.aws_region.to_owned()))
            .load()
            .await;

        let time_limit = NonZeroU32::new(cfg.aws_ses_max_emails_per_second).unwrap();
        let rate_limiter = governor::RateLimiter::direct(Quota::per_second(time_limit));

        let client = Client::new(&aws_cfg);

        // quick check to test if the SES client is valid
        client
            .get_account()
            .send()
            .await
            .expect("failed to get AWS SES account");

        println!("[SES] connection ok");

        Mailer {
            mailer_rmq,
            rate_limiter: Arc::new(rate_limiter),
            aws_client: client,
            default_sender: cfg.app_default_email_sender.to_owned(),
            aws_ses_tracking_config_set: cfg.aws_ses_tracking_config_set.to_owned(),
        }
    }

    /// Sends the emails for all the recipients in parallel, passing uuid to the email tags.
    ///
    /// Each recipient with non empty replacements have the `body_html` {{}} tags
    /// replaced by the recipients replacements. Emails are send individually for
    /// every recipient with replacements or for every recipient if `track_events` is true.
    ///
    /// this future resolves once all the emails have been sent
    #[tracing::instrument(
        skip_all,
        fields(
            subject = %options.subject,
            mail_uuid = %options.uuid,
            track_events = %options.track_events
        )
    )]
    pub async fn send_emails(&self, options: SendEmailOptions) -> Result<(), String> {
        let html = options.body_html.unwrap_or_default();
        let text = options.body_text.unwrap_or_default();
        let subject = to_utf8_content(&options.subject)
            .map_err(|_| String::from("failed to build subject"))?;

        let uuid_str = options.uuid.to_string();

        let from = options.from.unwrap_or(self.default_sender.clone());

        event!(Level::INFO, from);

        let config_set = if options.track_events {
            Some(self.aws_ses_tracking_config_set.clone())
        } else {
            None
        };

        let (recipients_with_replacements, recipients_without_replacements): (_, Vec<_>) = options
            .to
            .into_iter()
            .partition(|recipient| recipient.has_replacements());

        let mut send_email_tasks = JoinSet::new();

        let email_id_tag = MessageTag::builder()
            .name(MAIL_REQUEST_UUID_TAG_NAME)
            .value(uuid_str.clone())
            .build()
            .map_err(|_| String::from("failed to build email id tag"))?;

        if !recipients_with_replacements.is_empty() {
            let mut reg = Handlebars::new();

            let template_registered = reg.register_template_string(&uuid_str, &html).is_ok();

            // Replace the HTML template with the specific replacements of every recipient
            // and send the email, since emails here must be sent individually email tracing
            // will work fine.
            for recipient in recipients_with_replacements {
                let recipient_html = if template_registered {
                    reg.render(&uuid_str, &recipient.replacements)
                        .unwrap_or(html.clone())
                } else {
                    html.clone()
                };

                let body = Body::builder()
                    .html(
                        to_utf8_content(&recipient_html)
                            .map_err(|_| String::from("failed to build html"))?,
                    )
                    .text(to_utf8_content(&text).map_err(|_| String::from("failed to build html"))?)
                    .build();

                let msg = Message::builder()
                    .subject(subject.clone())
                    .body(body)
                    .build();

                let email_content = EmailContent::builder().simple(msg).build();

                let dest = Destination::builder()
                    .to_addresses(recipient.email.clone())
                    .build();

                send_email_tasks.spawn(
                    send_with_rate_limiter(
                        self.rate_limiter.clone(),
                        self.aws_client
                            .send_email()
                            .from_email_address(from.clone())
                            .destination(dest)
                            .email_tags(email_id_tag.clone())
                            .set_reply_to_addresses(options.reply_to_addresses.clone())
                            .set_configuration_set_name(config_set.clone())
                            .content(email_content.clone()),
                        options.uuid,
                        vec![recipient.email.clone()],
                        self.mailer_rmq.clone(),
                    )
                    .instrument(tracing::Span::current()),
                );
            }
        }

        if !recipients_without_replacements.is_empty() {
            // if were supposed to track events for the email, the chunk size must be `1` to send emails individually,
            // otherwise we cannot determine the specific recipient that triggered a email event (eg: `open`, `click`)
            let chunk_size = if options.track_events {
                1
            } else {
                MAX_RECIPIENTS_PER_SEND_EMAIL_OP
            };

            for recipient_chunk in recipients_without_replacements.chunks(chunk_size) {
                let chunk_emails: Vec<String> = recipient_chunk
                    .to_vec()
                    .iter()
                    .map(|e| e.email.to_owned())
                    .collect();

                let body = Body::builder()
                    .html(to_utf8_content(&html).map_err(|_| String::from("failed to build html"))?)
                    .text(to_utf8_content(&text).map_err(|_| String::from("failed to build text"))?)
                    .build();

                let msg = Message::builder()
                    .subject(subject.clone())
                    .body(body)
                    .build();

                let email_content = EmailContent::builder().simple(msg).build();

                let dest = Destination::builder()
                    .set_to_addresses(Some(chunk_emails.clone()))
                    .build();

                send_email_tasks.spawn(
                    send_with_rate_limiter(
                        self.rate_limiter.clone(),
                        self.aws_client
                            .send_email()
                            .from_email_address(from.clone())
                            .destination(dest)
                            .email_tags(email_id_tag.clone())
                            .set_configuration_set_name(config_set.clone())
                            .set_reply_to_addresses(options.reply_to_addresses.clone())
                            .content(email_content.clone()),
                        options.uuid,
                        chunk_emails.clone(),
                        self.mailer_rmq.clone(),
                    )
                    .instrument(tracing::Span::current()),
                );
            }
        }

        // Wait for all tasks to finish
        while send_email_tasks.join_next().await.is_some() {}

        Ok(())
    }
}
