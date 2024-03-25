use super::{routes::default, utils::get_delivery_type};
use crate::{mailer::Mailer, queue};
use lapin::message::Delivery;
use std::sync::Arc;
use tracing::error;

pub struct QueueRouter {
    pub server: Arc<queue::MailerRabbitmq>,
    pub mailer: Mailer,
}

impl QueueRouter {
    pub fn new(server: Arc<queue::MailerRabbitmq>, mailer: Mailer) -> QueueRouter {
        QueueRouter { server, mailer }
    }

    #[tracing::instrument(skip_all)]
    pub async fn handle_delivery(&self, delivery: Delivery) {
        let delivery_type = get_delivery_type(&delivery);

        let handler_res = match delivery_type.as_str() {
            "sendEmail" => self.send_email_handler(delivery).await,
            _ => default::handle_delivery_without_corresponding_rpc(delivery).await,
        };

        if let Err(err) = handler_res {
            error!(
                "handler for delivery of type: {} returned error: {}",
                delivery_type, err
            );
        }
    }
}
