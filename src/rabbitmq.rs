use deadpool_lapin::{Manager, Pool};
use lapin::ConnectionProperties;

/// creates a connection pool
///
/// # PANICS
/// panics if the tokyo runtime is never specified, this should never happen
pub fn get_connection_pool(rmq_url: &str) -> Pool {
    let props = ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let manager = Manager::new(rmq_url, props);

    deadpool::managed::Pool::builder(manager)
        .max_size(4)
        .build()
        .unwrap_or_else(|_| panic!("[RMQ] failed to build connection pool"))
}
