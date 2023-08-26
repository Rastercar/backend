use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::time::Duration;

/// starts a tokio task that deletes all the expired user sessions every five minutes
pub fn start_clear_sessions_cronjob(db_conn_pool: Pool<AsyncPgConnection>) {
    use crate::database::schema::session::dsl::*;

    println!("[CRON] clearing expired sessions every 5 minutes");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));

        loop {
            interval.tick().await;

            if let Some(conn) = &mut db_conn_pool.get().await.ok() {
                diesel::delete(session.filter(expires_at.lt(Utc::now())))
                    .execute(conn)
                    .await
                    .ok();
            }
        }
    });
}
