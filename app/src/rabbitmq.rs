use lapin::{
    options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    publisher_confirm::PublisherConfirm,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use serde::de;
use std::time::Duration;
use tokio::{sync::RwLock, time::sleep};
use tokio_stream::StreamExt;
use tracing::{error, info};

/// RabbitMQ default exchange (yes, its a empty string)
pub static DEFAULT_EXCHANGE: &str = "";

/// RabbitMQ queue to be binded to the tracker events exchange
pub static TRACKER_EVENTS_QUEUE: &str = "tracker";

/// RabbitMQ queue to publish requests to the mailer service
pub static MAILER_QUEUE: &str = "mailer";

/// RabbitMQ exchange to listen to tracker events, such as positions and alerts
pub static TRACKER_EVENTS_EXCHANGE: &str = "tracker_events";

struct ConnectionEntities {
    connection: Connection,
    consume_channel: Channel,
    publish_channel: Channel,
}

pub struct Rmq {
    /// RabbitMQ connetion URI
    amqp_uri: String,

    /// RabbitMQ connection
    connection: RwLock<Option<Connection>>,

    /// channel for consuming exchanges
    consume_channel: RwLock<Option<Channel>>,

    /// channel for publishing messages, see:
    ///
    /// https://stackoverflow.com/questions/25070042/rabbitmq-consuming-and-publishing-on-same-channel
    publish_channel: RwLock<Option<Channel>>,
}

/// Main abstraction for using RabbitMQ
impl Rmq {
    pub async fn new(amqp_uri: &str) -> Self {
        if let Ok(c) = Self::connect(amqp_uri).await {
            return Rmq {
                connection: RwLock::new(Some(c.connection)),
                amqp_uri: String::from(amqp_uri),
                consume_channel: RwLock::new(Some(c.consume_channel)),
                publish_channel: RwLock::new(Some(c.publish_channel)),
            };
        }

        error!("[RMQ] First connection to RabbitMQ failed");
        Rmq {
            connection: RwLock::new(None),
            amqp_uri: String::from(amqp_uri),
            consume_channel: RwLock::new(None),
            publish_channel: RwLock::new(None),
        }
    }

    // TODO: this consumer should be dynamic
    // check if its ok to have multiple consumers on a same channel
    //
    // TODO: accept some sort of callback on the consumer
    //
    // TODO: check consumer exclusivity
    // https://www.rabbitmq.com/docs/consumers#exclusivity
    pub async fn consume(&self) -> lapin::Result<()> {
        let mut consumer = self
            .consume_channel
            .read()
            .await
            .as_ref()
            .ok_or(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))?
            .basic_consume(
                TRACKER_EVENTS_QUEUE,
                "TODO",
                // TODO: check for autoack
                // pass consumer tag as arg
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    println!("{:?}", String::from_utf8(delivery.data));
                }
                Err(err) => {
                    println!("[RMQ] mailer queue consumer error: {}", err);
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    pub async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        options: BasicPublishOptions,
        payload: &[u8],
        properties: BasicProperties,
    ) -> lapin::Result<PublisherConfirm> {
        self.publish_channel
            .read()
            .await
            .as_ref()
            .ok_or(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Closed,
            ))?
            .basic_publish(exchange, routing_key, options, payload, properties)
            .await
    }

    /// Creates a connection to RabbitMQ, creating the
    /// needed exchanges for the application to work
    ///
    /// # PANICS
    ///
    /// panics whenever failing to declare any exchange or queue, as
    /// this kind of error is most likely due to a different
    /// configuration of existing exchanges/queues on the RabbitMQ
    /// instance and the config on the code, this kind of error wont
    /// work on retries unless this is panic 'worthy'
    async fn connect(amqp_uri: &str) -> lapin::Result<ConnectionEntities> {
        let connecion_properties = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection = Connection::connect(amqp_uri, connecion_properties).await?;
        info!("[RMQ] Reconnected to RabbitMQ");

        let consume_channel = connection.create_channel().await?;
        info!("[RMQ] consume channel created");
        let publish_channel = connection.create_channel().await?;
        info!("[RMQ] publish channel created");

        panic_on_err(
            consume_channel
                .exchange_declare(
                    TRACKER_EVENTS_EXCHANGE,
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
                .await,
        );
        info!("[RMQ] tracker events exchange declared");

        panic_on_err(
            consume_channel
                .queue_declare(
                    TRACKER_EVENTS_QUEUE,
                    QueueDeclareOptions {
                        passive: false,
                        durable: false,
                        exclusive: false,
                        auto_delete: true,
                        nowait: false,
                    },
                    FieldTable::default(),
                )
                .await,
        );
        info!("[RMQ] tracker events queue declared");

        // bind the tracker events queue to the tracker events exchange and listen to all events (#)
        consume_channel
            .queue_bind(
                TRACKER_EVENTS_QUEUE,
                TRACKER_EVENTS_EXCHANGE,
                "#",
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!("[RMQ] tracker events queue binded to tracker events exchange");

        Ok(ConnectionEntities {
            connection,
            publish_channel,
            consume_channel,
        })
    }

    /// Starts a tokio task that will keep checking the connection
    /// status every five seconds, if the connection is broken we
    /// attempt to reconnect and set the connection and channels
    pub async fn start_reconnection_task(&self) {
        loop {
            sleep(Duration::from_secs(5)).await;

            let is_connected = match self.connection.read().await.as_ref() {
                Some(connection) => connection.status().connected(),
                None => false,
            };

            if is_connected {
                continue;
            }

            *self.connection.write().await = None;
            *self.publish_channel.write().await = None;
            *self.consume_channel.write().await = None;

            match Self::connect(&self.amqp_uri).await {
                Ok(c) => {
                    *self.connection.write().await = Some(c.connection);
                    *self.consume_channel.write().await = Some(c.consume_channel);
                    *self.publish_channel.write().await = Some(c.publish_channel);
                }
                Err(err) => {
                    error!("[RMQ] Reconnection failed: {:?}", err);
                }
            }
        }
    }

    pub async fn shutdown(&self) {
        println!("[RMQ] closing publish channel");
        if let Some(chan) = self.publish_channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                info!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing consume channel");
        if let Some(chan) = self.publish_channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                info!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing connection");
        if let Some(conn) = self.connection.read().await.as_ref() {
            if let Err(conn_close_err) = conn.close(200, "user shutdown").await {
                info!("[RMQ] failed to close connection: {}", conn_close_err)
            }
        }

        *self.connection.write().await = None;
        *self.consume_channel.write().await = None;
        *self.publish_channel.write().await = None;
    }
}

fn panic_on_err<T>(err: Result<T, lapin::Error>) {
    if let Err(e) = err {
        panic!("[RMQ] critical error: {}", e);
    }
}
