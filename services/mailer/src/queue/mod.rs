pub mod controller;

use crate::{config::app_config, utils::errors::ResultExt};
use lapin::{
    message::Delivery,
    options::{
        BasicConsumeOptions, BasicPublishOptions, BasicQosOptions, ExchangeDeclareOptions,
        QueueDeclareOptions,
    },
    publisher_confirm::PublisherConfirm,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, Consumer, ExchangeKind,
};
use serde::Serialize;
use std::{thread, time};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_stream::StreamExt;
use tracing::{event, Level};

pub trait Routable {
    /// Creates a routing to be used to send rabbitmq messages with
    /// the content being the serialized implementer of this trait
    fn routing_key(&self) -> String;
}

pub struct MailerRabbitmq {
    /// URI to connect to rabbitmq
    uri: String,

    /// name of the main queue to be consumed
    mailer_queue: String,

    /// consumer tag used to identify the mailer queue consumer
    consumer_tag: String,

    /// name of the exchange used to publish email events
    email_events_exchange: String,

    /// channel for consuming / pooling messages
    consume_channel: RwLock<Option<Channel>>,

    /// channel for publishing messages, see:
    ///
    /// https://stackoverflow.com/questions/25070042/rabbitmq-consuming-and-publishing-on-same-channel
    publish_channel: RwLock<Option<Channel>>,

    /// rabbitmq connection
    connection: RwLock<Option<Connection>>,

    /// tokio channel to send all the received rabbitmq deliveries to be handled.
    delivery_sender: UnboundedSender<Delivery>,
}

impl MailerRabbitmq {
    pub fn new(delivery_sender: UnboundedSender<Delivery>) -> MailerRabbitmq {
        let cfg = app_config();

        MailerRabbitmq {
            uri: cfg.rmq_uri.clone(),
            mailer_queue: cfg.rmq_queue.clone(),
            consumer_tag: cfg.rmq_consumer_tag.clone(),
            email_events_exchange: cfg.rmq_email_events_exchange.clone(),

            delivery_sender,

            // [IDEA]: find a more elegant solution ?
            // it might seem really dumb to have the channel and connection to be on a RwLock,
            // however, the channel and connection are only written on the first connection
            // and subsequent reconnects, so read access is free 99.99% of the time, adding little
            // to no overhead
            //
            // maybe do not not make the reconnect loop a part of this struct, this way `RwLock<Option<Channel>>`
            // could be simply `Channel`.
            //
            // however this would require recreating MailerRabbitmq with the connection after connecting/reconnecting
            // and thus the instance of the MailerRabbitmq would not be stable, so idk.
            connection: RwLock::new(None),

            consume_channel: RwLock::new(None),
            publish_channel: RwLock::new(None),
        }
    }

    /// Runs the RabbitMQ mail queue consumer, attempting to reconnect endlessly
    /// if the RabbitMQ connection is dropped.
    pub async fn start_consumer(&self) {
        let mut reconnect_delay = 2;

        let max_reconnect_delay = 60 * 10;

        loop {
            if let Err(err) = self.connect_and_consume().await {
                eprintln!("[RMQ] connection error: {}", err)
            }

            thread::sleep(time::Duration::from_secs(reconnect_delay));
            println!(
                "[RMQ] reconnecting, next attempt in: {} seconds",
                reconnect_delay
            );

            if reconnect_delay < max_reconnect_delay {
                reconnect_delay *= 2
            }
        }
    }

    /// Connects to rabbitmq, declaring all the queues, exchanges and consumers needed.
    /// lastly starts consuming deliveries from the mailer queue, returning only when the
    /// connection is dropped.
    async fn connect_and_consume(&self) -> Result<(), lapin::Error> {
        let props = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection = Connection::connect(&self.uri, props).await?;
        println!("[RMQ] connected");

        let publish_channel = connection.create_channel().await?;
        println!("[RMQ] consume channel created");

        let mut consume_channel = connection.create_channel().await?;
        println!("[RMQ] publish channel created");

        // Consumer prefetch count
        //
        // We do not want a unlimited prefetch count to avoid crashing the service if a ton
        // of email sending requests are received, even though that probably wont happen due
        // to the mailer queue message TTL.
        //
        // Since sending emails can take a lot of time due to the rate limiter, especially if
        // the emails have a lot of recipients and email tracking is true, a prefetch of 1 is
        // not optimal, so set it to at least 10 (yep i chose this value arbitrarily)
        //
        // https://www.cloudamqp.com/blog/how-to-optimize-the-rabbitmq-prefetch-count.html
        consume_channel
            .basic_qos(10, BasicQosOptions::default())
            .await?;

        let mut consumer = self
            .declare_exchanges_and_queues(&mut consume_channel)
            .await;

        *self.connection.write().await = Some(connection);
        *self.consume_channel.write().await = Some(consume_channel);
        *self.publish_channel.write().await = Some(publish_channel);

        self.consume_messages_until_error(&mut consumer).await
    }

    /// Declares all the exchanges, queues and the consumer needed to run the application
    ///
    /// # EXITS
    ///
    /// exits the process if any declaration fails.
    async fn declare_exchanges_and_queues(&self, channel: &mut Channel) -> Consumer {
        channel
            .exchange_declare(
                &self.email_events_exchange,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    passive: false,
                    durable: true,
                    auto_delete: false,
                    internal: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await
            .unwrap_or_exit_process();
        println!("[RMQ] events exchange declared");

        let mut queue_options = FieldTable::default();

        // Mailer Queue TTL
        //
        // Its VERY important to have a short TTL for the mailer queue, otherwise
        // if the service fails for some reason, and consumers keep retrying the
        // mailer queue might fill up with `sendEmail` requests and when the service
        // comes back up it will send tons of duplicated emails
        queue_options.insert(
            "x-message-ttl".into(),
            lapin::types::AMQPValue::ShortUInt(30_000),
        );

        channel
            .queue_declare(
                &self.mailer_queue,
                QueueDeclareOptions {
                    nowait: false,
                    passive: false,
                    durable: true,
                    exclusive: false,
                    auto_delete: false,
                },
                queue_options,
            )
            .await
            .unwrap_or_exit_process();
        println!("[RMQ] mailer queue declared");

        let consumer = channel
            .basic_consume(
                &self.mailer_queue,
                &self.consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap_or_exit_process();
        println!("[RMQ] mailer queue consumer started");

        consumer
    }

    /// Consumes all the deliveries on the mailer queue, sending them to sender channel channel
    ///
    /// this methods only returns after the consumer returns an error or the rabbitmq connection is dropped.
    ///
    /// # PANICS
    ///
    /// panics if the delivery_sender channel is closed, as this should not happen in the entirety of the program
    async fn consume_messages_until_error(
        &self,
        consumer: &mut Consumer,
    ) -> Result<(), lapin::Error> {
        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    // the delivery_sender channel should be open for the entirety
                    // of the program so a panic here is desirable
                    self.delivery_sender
                        .send(delivery)
                        .expect("sender channel closed");
                }
                Err(err) => {
                    println!("[RMQ] mailer queue consumer error: {}", err);
                    return Err(err);
                }
            }
        }

        // this should be unreachable as the consumer stream should never end as long as
        // the connection is open and when its closed the error case above is triggered
        println!("[RMQ] mailer queue consumer stopped, stream ended");
        Ok(())
    }

    #[tracing::instrument(skip(self, payload, properties))]
    async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
        properties: BasicProperties,
    ) -> Result<PublisherConfirm, String> {
        self.publish_channel
            .read()
            .await
            .as_ref()
            .ok_or("failed to publish, RMQ publishing channel is not available")?
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload,
                properties,
            )
            .await
            .or(Err(String::from("failed to confirm publishing")))
    }

    /// Publishes a mailer event as json to the `email_events_exchange`, using
    /// the routing key from the event from the `Routable` trait.
    #[tracing::instrument(skip_all)]
    pub async fn publish_event<T>(&self, event: T) -> Result<PublisherConfirm, String>
    where
        T: Serialize + Routable,
    {
        let routing_key = event.routing_key();

        event!(Level::INFO, routing_key);

        let json = serde_json::to_string(&event).or(Err("failed to serialize event".to_owned()))?;

        self.publish(
            &self.email_events_exchange,
            routing_key.as_str(),
            json.as_bytes(),
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await
    }

    /// Closes the rabbitmq connection and the publish and consume channels
    pub async fn shutdown(&self) {
        println!("[RMQ] closing publish channel");
        if let Some(chan) = self.publish_channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                eprintln!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing consume channel");
        if let Some(chan) = self.publish_channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                eprintln!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing connection");
        if let Some(conn) = self.connection.read().await.as_ref() {
            if let Err(conn_close_err) = conn.close(200, "user shutdown").await {
                eprintln!("[RMQ] failed to close connection: {}", conn_close_err)
            }
        }

        *self.connection.write().await = None;
        *self.consume_channel.write().await = None;
        *self.publish_channel.write().await = None;
    }
}
