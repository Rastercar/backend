use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDto {
    pub lat: f64,
    pub lng: f64,
    pub tracker_id: i32,
}
