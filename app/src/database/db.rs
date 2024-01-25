use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{info, log};

pub async fn create_db_conn(db_url: &str) -> DatabaseConnection {
    let mut opt = ConnectOptions::new(db_url);

    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);

    info!("[DB] getting connection");
    Database::connect(opt)
        .await
        .unwrap_or_else(|_| panic!("[DB] failed to build connection pool"))
}

/// Apply all pending migrations
pub async fn run_migrations(db: &DatabaseConnection) {
    info!("[DB] running migrations");
    Migrator::up(db, None)
        .await
        .unwrap_or_else(|_| panic!("[DB] failed to run migrations"));
}

/// TODO: RM ME !
/// creates a connection pool
///
/// # PANICS
/// panics if the tokyo runtime is never specified, this should never happen
pub async fn create_connection_pool(database_url: &str) -> Pool<AsyncPgConnection> {
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);

    Pool::builder(config)
        .max_size(8)
        .build()
        .unwrap_or_else(|_| panic!("[DB] failed to build connection pool"))
}
