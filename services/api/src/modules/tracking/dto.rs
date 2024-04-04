use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDto {
    pub lat: f64,
    pub lng: f64,
    pub timestamp: DateTime<Utc>,
    pub tracker_id: i32,
}
