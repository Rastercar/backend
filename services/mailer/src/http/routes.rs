use std::{collections::HashMap, env};

use super::server::AppState;
use crate::{
    mailer::MAIL_REQUEST_UUID_TAG_NAME,
    queue::controller::dto::{
        events::{Email, EmailEvent},
        ses::{SesEvent, SnsNotification},
    },
};
use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use convert_case::{Case, Casing};
use opentelemetry::trace::Status;
use tracing::error;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tracing::instrument(skip_all)]
fn get_email_event_from_json_str(body: &str) -> Result<EmailEvent, String> {
    let sns_notification = serde_json::from_str::<SnsNotification>(body)
        .map_err(|e| format!("failed to parse request body to SnsNotification: {}", e))?;

    if let Some(sub_url) = sns_notification.subscribe_url {
        let is_subscription_confirmation = sns_notification
            .notification_type
            .eq("SubscriptionConfirmation");

        if is_subscription_confirmation {
            println!("[WEB] SNS subscription confirmation link: {}", sub_url);
            return Err("request is subscription confirmation event, not a email event".to_owned());
        }
    }

    let ses_evt = serde_json::from_str::<SesEvent>(&sns_notification.message)
        .map_err(|e| format!("failed to parse request body to SesEvent: {}", e))?;

    let request_uuid = ses_evt
        .mail
        .tags
        .get(MAIL_REQUEST_UUID_TAG_NAME)
        .ok_or(format!(
            "required tag: {} not present on mail tags",
            MAIL_REQUEST_UUID_TAG_NAME
        ))?
        .first()
        .ok_or(format!(
            "required tag: {} is present but is empty",
            MAIL_REQUEST_UUID_TAG_NAME
        ))?
        .to_owned();

    let event_type = ses_evt
        .event_type
        .or(ses_evt.notification_type)
        .ok_or("failed to get event type from ses event")?
        .to_case(Case::Snake);

    let err_msg = format!("object for event of type: {} not present", event_type);

    let original = match event_type.as_str() {
        "send" => Email::send(ses_evt.send.ok_or(err_msg)?),
        "open" => Email::open(ses_evt.open.ok_or(err_msg)?),
        "click" => Email::click(ses_evt.click.ok_or(err_msg)?),
        "bounce" => Email::bounce(ses_evt.bounce.ok_or(err_msg)?),
        "reject" => Email::reject(ses_evt.reject.ok_or(err_msg)?),
        "failure" => Email::failure(ses_evt.failure.ok_or(err_msg)?),
        "delivery" => Email::delivery(ses_evt.delivery.ok_or(err_msg)?),
        "complaint" => Email::complaint(ses_evt.complaint.ok_or(err_msg)?),
        "subscription" => Email::subscription(ses_evt.subscription.ok_or(err_msg)?),
        "delivery_delay" => Email::delivery_delay(ses_evt.delivery_delay.ok_or(err_msg)?),
        _ => return Err(format!("unknown event type: {}", event_type)),
    };

    Ok(EmailEvent {
        original,
        event_type,
        request_uuid,
        mail: ses_evt.mail,
    })
}

/// Attempts to parse the request body as Json object of a AWS SES event.
///
/// on success publishes the event to the mailer events queue.
#[tracing::instrument(skip_all)]
pub async fn handle_ses_event(
    State(state): State<AppState>,
    body: String,
) -> Result<String, StatusCode> {
    let span = Span::current();

    match get_email_event_from_json_str(&body) {
        Ok(email_event) => {
            span.set_attribute("event type", email_event.event_type.clone());
            span.set_attribute("email uuid", email_event.request_uuid.clone());

            if let Err(publish_error) = state.mailer_rmq.publish_event(email_event).await {
                error!(publish_error);

                span.set_status(Status::Error {
                    description: String::from("event publishing failed").into(),
                });

                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            span.set_status(Status::Ok);
            Ok("event handled correctly".to_owned())
        }
        Err(error) => {
            error!(error);

            span.set_status(Status::Error {
                description: String::from("failed to decode email event").into(),
            });

            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// forbids any incoming requests where the x-amz-sns-subscription-arn
/// does not match the `aws_email_sns_subscription_arn` in the application state,
/// in order to avoid potentially malicious requests from registering fake events
#[tracing::instrument(skip_all)]
pub async fn check_aws_sns_arn_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    nxt: Next,
) -> Result<Response, (StatusCode, String)> {
    let span = Span::current();

    span.set_attribute(
        "provided aws.sns.subscription_arn",
        format!("{:?}", req.headers().get("x-amz-sns-subscription-arn")),
    );

    span.set_attribute(
        "required aws.sns.subscription_arn",
        format!("{:?}", state.aws_email_sns_subscription_arn),
    );

    if let Some(sns_arn_to_match) = state.aws_email_sns_subscription_arn {
        if let Some(sns_arn_header) = req.headers().get("x-amz-sns-subscription-arn") {
            let request_sns_arn = sns_arn_header.to_str().unwrap_or("");

            if request_sns_arn.eq(&sns_arn_to_match) {
                return Ok(nxt.run(req).await);
            }
        }

        tracing::error!("invalid sns arn");
        return Err((StatusCode::FORBIDDEN, String::from("invalid SNS ARN")));
    }

    Ok(nxt.run(req).await)
}

pub async fn healthcheck(Query(params): Query<HashMap<String, String>>) -> (StatusCode, String) {
    if params.get("debug").map(|v| v == "true").unwrap_or(false) {
        let commit_sha = env::var("COMMIT_HASH").unwrap_or_else(|_| "unknown".to_string());

        return (StatusCode::OK, format!("OK, commit HASH: {}", commit_sha));
    }

    (StatusCode::OK, String::from("ok"))
}
