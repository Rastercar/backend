use deadpool_lapin::{Manager, Pool};
use lapin::ConnectionProperties;

// TODO: NUKE THIS AND REMOVE MAILER FROM USAGE

/// the default rabbitmq exchange, (yes its a empty string)
///
/// see: https://www.rabbitmq.com/tutorials/amqp-concepts.html
pub static DEFAULT_EXCHANGE: &str = "";

/// creates a connection pool
///
/// # PANICS
/// panics if the tokyo runtime is never specified, this should never happen
pub fn create_connection_pool(rmq_url: &str) -> Pool {
    let props = ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let manager = Manager::new(rmq_url, props);

    deadpool::managed::Pool::builder(manager)
        .max_size(2)
        .build()
        .unwrap_or_else(|_| panic!("[RMQ] failed to build connection pool"))
}
