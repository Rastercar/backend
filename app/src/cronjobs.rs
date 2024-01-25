use chrono::Utc;
use entity::session;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::time::Duration;
use tracing::info;

/// starts a tokio task that deletes all the expired user sessions every inteval
pub fn start_clear_sessions_cronjob(db: DatabaseConnection, interval: Duration) {
    info!("[CRON] clearing expired sessions every 5 minutes");

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
