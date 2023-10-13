use std::fs;

use super::{
    dto::SendEmailIn,
    templates::{ConfirmEmailReplacements, RecoverPasswordReplacements},
};
use crate::{
    config::app_config, rabbitmq::DEFAULT_EXCHANGE, services::mailer::dto::EmailRecipient,
};
use anyhow::Result;
use deadpool_lapin::Pool;
use lapin::{
    options::BasicPublishOptions, publisher_confirm::PublisherConfirm, BasicProperties, Channel,
};
use url;

/// rabbitmq queue to publish RPC requests to the mailer service
static MAILER_QUEUE: &str = "mailer";

/// RPC operation to send a email
static OP_SEND_EMAIL: &str = "sendEmail";

/// A abstraction to make RPC calls to the mailer microservice
#[derive(Clone)]
pub struct MailerService {
    rmq_conn_pool: Pool,
}

impl MailerService {
    pub fn new(rmq_conn_pool: Pool) -> MailerService {
        MailerService { rmq_conn_pool }
    }

    // [IMPROVE ME] for now, we create a rmq channel every time we want to do something,
    // destroying the channel when the op is done, this is not a problem if we have little
    // to no users, however this is far from ideal.
    //
    // a good scenario would be to have a connection pool for both connections and their associate channels
    // the implementation is not as simple as a channel can be locked and a connection dropped, etc.
    //
    // see: https://github.com/bikeshedder/deadpool/issues/47
    async fn get_channel(&self) -> Result<Channel> {
        Ok(self.rmq_conn_pool.get().await?.create_channel().await?)
    }

    async fn publish_to_mailer_service(
        &self,
        payload: &[u8],
        rpc_name: &str,
    ) -> Result<PublisherConfirm> {
        Ok(self
            .get_channel()
            .await?
            .basic_publish(
                DEFAULT_EXCHANGE,
                MAILER_QUEUE,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_kind(rpc_name.into()),
            )
            .await?)
    }

    pub async fn send_email(&self, input: SendEmailIn) -> Result<PublisherConfirm> {
        Ok(self
            .publish_to_mailer_service(serde_json::to_string(&input)?.as_bytes(), OP_SEND_EMAIL)
            .await?)
    }

    pub async fn send_recover_password_email(
        &self,
        email: String,
        reset_password_token: String,
        username: String,
    ) -> Result<PublisherConfirm> {
        let mut link = create_frontend_link("auth/change-password")?;
        link.set_query(Some(format!("token={}", reset_password_token).as_str()));

        let replacements = Some(Into::into(RecoverPasswordReplacements {
            username,
            reset_password_link: link.into(),
        }));

        let email = SendEmailIn::default()
            .with_subject("Rastercar: recover password")
            .with_body_html(&read_template("recover-password")?)
            .with_to(vec![EmailRecipient {
                email,
                replacements,
            }]);

        Ok(self.send_email(email).await?)
    }

    pub async fn send_confirm_email_address_email(
        &self,
        email: String,
        reset_password_token: String,
    ) -> Result<PublisherConfirm> {
        let mut link = create_frontend_link("auth/confirm-email-address")?;
        link.set_query(Some(format!("token={}", reset_password_token).as_str()));

        let replacements = Some(Into::into(ConfirmEmailReplacements {
            confirmation_link: link.into(),
        }));

        let email = SendEmailIn::default()
            .with_subject("Rastercar: confirm email")
            .with_body_html(&read_template("confirm-email")?)
            .with_to(vec![EmailRecipient {
                email,
                replacements,
            }]);

        Ok(self.send_email(email).await?)
    }
}

/// creates a link to the rastercar frontend
fn create_frontend_link(path: &str) -> Result<url::Url, url::ParseError> {
    Ok(app_config().frontend_url.join(path)?)
}

/// creates a link to the rastercar frontend
fn read_template(template: &str) -> std::io::Result<String> {
    Ok(fs::read_to_string(format!("templates/{}.hbs", template))?)
}
