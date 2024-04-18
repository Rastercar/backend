use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PositionDto {
    pub lat: f64,
    pub lng: f64,
    pub timestamp: DateTime<Utc>,
    pub tracker_id: i32,
}

/// SocketIO connection payload
#[derive(Deserialize)]
pub struct AuthPayload {
    /// A short lived token for a rastercar API user
    pub token: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct GetTrackersLastPositionsDto {
    /// ids of the trackers to get positions of
    #[validate(length(min = 1, max = 20))]
    pub ids: Vec<i32>,
}
