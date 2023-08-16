use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

/// creates a connection pool
///
/// # PANICS
/// panics if the tokyo runtime is never specified, this should never happen
pub async fn get_connection_pool(database_url: &String) -> Pool<AsyncPgConnection> {
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);

    Pool::builder(config)
        .max_size(8)
        .build()
        .unwrap_or_else(|_| panic!("[DB] failed to build connection pool"))
}

/// runs migrations on a single blocking connection, since we cannot use async diesel to run migrations
///
/// see: https://github.com/weiznich/diesel_async/issues/17
///
/// # PANICS
/// panics when failing to connect to the database or when running the migrations returns a error
pub fn run_migrations(database_url: &String) {
    use diesel::prelude::*;

    let mut connection = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("[DB] failed to connect to database to run migrations"));

    connection
        .run_pending_migrations(MIGRATIONS)
        .unwrap_or_else(|_| panic!("[DB] failed to run migrations"));
}
