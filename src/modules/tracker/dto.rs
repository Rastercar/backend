use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

// TODO: remove debug
#[derive(Deserialize, ToSchema, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateTrackerDto {
    #[validate(length(min = 1))]
    pub model: String,

    #[validate(length(min = 1))]
    pub imei: String,

    /// ID of the vehicle to associate with the tracker
    #[validate(range(min = 1))]
    pub vehicle_id: Option<i32>,
}
