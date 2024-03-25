use crate::{
    mailer::SendEmailOptions,
    queue::controller::{
        dto::events::{EmailRequestFinishedEvent, EmailSendingReceivedEvent},
        router::QueueRouter,
        utils::ack_delivery,
    },
};
use lapin::message::Delivery;
use shared::dto::mailer::SendEmailIn;
use tracing::{event, Level};
use uuid::Uuid;
use validator::Validate;

impl QueueRouter {
    #[tracing::instrument(skip_all)]
    pub async fn send_email_handler(&self, delivery: Delivery) -> Result<(), String> {
        ack_delivery(&delivery).await?;

        let send_email_in = serde_json::from_slice::<SendEmailIn>(&delivery.data)
            .map_err(|e| format!("parse error: {:#?}", e))?;

        let uuid = send_email_in.uuid.unwrap_or(Uuid::new_v4());

        event!(Level::INFO, email_uuid = uuid.to_string());

        if let Err(e) = send_email_in.validate() {
            self.server
                .publish_event(EmailSendingReceivedEvent::rejected(uuid, send_email_in))
                .await?;

            return Err(e.to_string());
        }

        self.server
            .publish_event(EmailSendingReceivedEvent::started(
                uuid,
                send_email_in.clone(),
            ))
            .await?;

        self.mailer
            .send_emails(SendEmailOptions {
                uuid,
                to: send_email_in.to,
                from: send_email_in.sender,
                subject: send_email_in.subject,
                body_text: send_email_in.body_text,
                body_html: send_email_in.body_html,
                track_events: send_email_in.enable_tracking,
                reply_to_addresses: send_email_in.reply_to_addresses,
            })
            .await?;

        self.server
            .publish_event(EmailRequestFinishedEvent::new(uuid))
            .await?;

        Ok(())
    }
}
