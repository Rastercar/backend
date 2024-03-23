use chrono::{DateTime, Utc};
use geozero::wkb;
use sea_orm::DatabaseConnection;
use sqlx::postgres::PgQueryResult;

pub async fn insert_vehicle_tracker_location(
    db: &DatabaseConnection,
    timestamp: DateTime<Utc>,
    tracker_id: i32,
    lat: f64,
    lng: f64,
) -> Result<PgQueryResult, sqlx::Error> {
    let point: geo_types::Geometry<f64> = geo_types::Point::new(lat, lng).into();

    sqlx::query(
        "INSERT INTO vehicle_tracker_location (time, vehicle_tracker_id, point) VALUES ($1, $2, ST_SetSRID($3, 4326))",
    )
    .bind(timestamp)
    .bind(tracker_id)
    .bind(wkb::Encode(point))
    .execute(db.get_postgres_connection_pool())
    .await
}
