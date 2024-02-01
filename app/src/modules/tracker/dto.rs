use serde::Deserialize;
use shared::TrackerModel;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

fn is_supported_tracker_model(model: &str) -> Result<(), ValidationError> {
    let allowed_models = TrackerModel::to_string_vec();

    if !allowed_models.contains(&String::from(model)) {
        return Err(ValidationError::new("model not allowed"));
    }

    Ok(())
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateTrackerDto {
    #[validate(custom = "is_supported_tracker_model")]
    pub model: String,

    #[validate(length(min = 1))]
    pub imei: String,

    /// ID of the vehicle to associate with the tracker
    #[validate(range(min = 1))]
    pub vehicle_id: Option<i32>,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListTrackersDto {
    /// Search trackers by IMEI
    pub imei: Option<String>,

    /// If the trackers should be filtered if they are associated
    /// to a vehicle or not, `None` means `any`
    pub with_associated_vehicle: Option<bool>,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SetTrackerVehicleDto {
    /// Vehicle to associate the tracker to
    pub vehicle_id: i32,
}
