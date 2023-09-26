use super::dto::SendEmailIn;
use crate::rabbitmq::DEFAULT_EXCHANGE;
use anyhow::Result;
use deadpool_lapin::Pool;
use lapin::{
    options::BasicPublishOptions, publisher_confirm::PublisherConfirm, BasicProperties, Channel,
};

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
}
