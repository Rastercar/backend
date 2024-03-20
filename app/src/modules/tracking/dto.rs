use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDto {
    pub tracker_id: usize,
    // TODO: usize ? float ? ??
    pub lat: usize,
    pub lng: usize,
}
