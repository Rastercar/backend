use crate::config;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    publisher_confirm::PublisherConfirm,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use std::{thread, time};
use tokio::sync::{mpsc::UnboundedReceiver, RwLock};
use tracing::{Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct Options {
    pub rmq_uri: String,

    pub tracker_events_exchange: String,
}

/// A listener that recieves RabbitMQ messages on the reciever channel
/// and publishes those messages to the tracker events exchange.
///
/// [IMPROVEMENT]
///
/// Currently if the RabbitMQ connection is lost, any messages recieved
/// by the rust channel will be ignored, a good idea might be to create
/// queue that stores a limited number of messages until the connection
/// is restored, and then publish its contents on reconnection
pub struct RmqListener {
    options: Options,

    /// Channel used to publish messages to the tracker_events_exchange
    /// note that since were only publishing and not consuming,
    /// a single channel is optimal.
    channel: RwLock<Option<Channel>>,

    /// RabbitMQ connection, this
    connection: RwLock<Option<Connection>>,

    /// channel to receive messages to publish to the tracker events exchange
    receiver: RwLock<UnboundedReceiver<(RmqMessage, tracing::Span)>>,
}

#[derive(Debug)]
pub struct RmqMessage {
    /// Message content, most likely serialized JSON
    pub body: String,

    /// RabbitMQ routing key
    pub routing_key: String,
}

impl RmqListener {
    pub fn new(
        cfg: &config::AppConfig,
        receiver: UnboundedReceiver<(RmqMessage, tracing::Span)>,
    ) -> RmqListener {
        let options = Options {
            rmq_uri: cfg.rmq_uri.to_owned(),
            tracker_events_exchange: cfg.tracker_events_exchange.to_owned(),
        };

        RmqListener {
            options,
            channel: RwLock::new(None),
            connection: RwLock::new(None),
            receiver: RwLock::new(receiver),
        }
    }

    /// Starts a infinite loop that will attempt to recconect
    /// to RabbitMQ, once a connection is stablished calls `self.run`
    pub async fn start(&self) {
        let mut reconnect_delay = 2;

        let max_reconnect_delay = 60 * 10;

        loop {
            if let Err(err) = self.run().await {
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

    /// Creates and sets the RabbitMQ connection and channel and then starts listening
    /// to the RUST messages channel indefinitely, publishing the recieved messages to the
    /// tracker events exchange
    ///
    /// Returns `Err` when failing to connect to RabbitMQ or when a connection error happens
    /// after failing to publish
    ///
    /// [IMPROVEMENT]
    ///
    /// have some way to check for connection issues and attempt to reconnect immediately,
    /// a way to do this is to create a noop rabbitmq consumer and returns if the consumer
    /// is broken, this is done on the mailer service but in our case the consumer would be
    /// useless, it would be ideal to check for connection errors without creating a consumer
    async fn run(&self) -> Result<(), lapin::Error> {
        let conn_options = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection = Connection::connect(&self.options.rmq_uri, conn_options).await?;
        println!("[RMQ] connected");

        let channel = connection.create_channel().await?;
        println!("[RMQ] channel created");

        let declare_exchange_result = channel
            .exchange_declare(
                &self.options.tracker_events_exchange,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    nowait: false,
                    passive: false,
                    durable: true,
                    internal: false,
                    auto_delete: false,
                },
                FieldTable::default(),
            )
            .await;

        // If the exchange could not be declared successfully, this is most
        // likely due to differences between a the existing exchange config
        // such error will always happen regardless of retries.
        //
        // This is required for the whole application to work so exit on failure
        declare_exchange_result.unwrap_or_else(|e| {
            panic!("[RMQ] failed to declare tracker events exchange: {}", e);
        });

        println!("[RMQ] tracker events exchange created");

        *self.connection.write().await = Some(connection);
        *self.channel.write().await = Some(channel);

        while let Some((delivery, span)) = self.receiver.write().await.recv().await {
            if let Err(err) = self.send_message(&delivery).instrument(span).await {
                match err {
                    lapin::Error::InvalidChannelState(_)
                    | lapin::Error::InvalidConnectionState(_) => {
                        // The current connection and/or channel is in a bad state,
                        // drop it so lapin can run the destructors if there is any.
                        *self.connection.write().await = None;
                        *self.channel.write().await = None;

                        // Its very important to return the error here
                        // so `self.run` attempts to reconnect
                        return Err(err);
                    }
                    _ => {
                        // in this case a non connection error happened
                        // so we wont return and attempt a reconnect
                    }
                }
            }
        }

        println!("[RMQ] receiver channel closed");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn send_message(&self, message: &RmqMessage) -> Result<PublisherConfirm, lapin::Error> {
        let span = Span::current();
        let ctx = span.context();

        let amqp_headers = shared::tracer::create_amqp_headers_with_span_ctx(&ctx);

        self.channel
            .read()
            .await
            .as_ref()
            // self.channel should never have a value of None when this method is called
            // if it somehow happens, treat it like a channel error so a recconect is attempted
            .ok_or(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))?
            .basic_publish(
                &self.options.tracker_events_exchange,
                &message.routing_key,
                BasicPublishOptions::default(),
                message.body.as_bytes(),
                BasicProperties::default().with_headers(FieldTable::from(amqp_headers)),
            )
            .await
    }

    /// closes self.channel and self.connection and then sets both to `None`
    pub async fn shutdown(&self) {
        println!("[RMQ] closing channel");
        if let Some(chan) = self.channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                println!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing connection");
        if let Some(conn) = self.connection.read().await.as_ref() {
            if let Err(conn_close_err) = conn.close(200, "user shutdown").await {
                println!("[RMQ] failed to close connection: {}", conn_close_err)
            }
        }

        *self.channel.write().await = None;
        *self.connection.write().await = None;
    }
}
