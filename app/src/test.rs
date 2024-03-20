use lapin::{
    options::ExchangeDeclareOptions, types::FieldTable, Channel, Connection, ConnectionProperties,
    ExchangeKind,
};
use std::time::Duration;
use tokio::{sync::RwLock, time::sleep};
use tracing::{error, info};

/// RabbitMQ queue to publish requests to the mailer service
static MAILER_QUEUE: &str = "mailer";

/// RabbitMQ exchange to listen to tracker events, such as positions and alerts
static TRACKER_EVENTS_EXCHANGE: &str = "tracker_events";

/// RabbitMQ exchange to listen for email events, such as clicks, opens, reports
static EMAIL_EVENTS_EXCHANGE: &str = "email_events";

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
}

fn panic_on_err<T>(err: Result<T, lapin::Error>) {
    if let Err(e) = err {
        panic!("[RMQ] critical error: {}", e);
    }
}
