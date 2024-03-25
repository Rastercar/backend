use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use shared::entity::session;
use std::time::Duration;

/// starts a tokio task that deletes all the expired user sessions every inteval
pub fn start_clear_sessions_cronjob(db: DatabaseConnection, interval: Duration) {
    println!("[CRON] clearing expired sessions every 5 minutes");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval);

        loop {
            interval.tick().await;

            let _ = session::Entity::delete_many()
                .filter(session::Column::ExpiresAt.lt(Utc::now()))
                .exec(&db)
                .await;
        }
    });
}
