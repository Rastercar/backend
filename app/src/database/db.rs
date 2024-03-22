use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::info;

pub async fn connect(db_url: &str) -> DatabaseConnection {
    let mut opt = ConnectOptions::new(db_url);

    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    info!("[DB] getting connection");
    Database::connect(opt)
        .await
        .unwrap_or_else(|e| panic!("[DB] failed to build connection pool: {e}"))
}

/// Apply all pending migrations
pub async fn run_migrations(db: &DatabaseConnection) {
    info!("[DB] running migrations");
    Migrator::up(db, None)
        .await
        .unwrap_or_else(|e| panic!("[DB] failed to run migrations: {e}"));
}
