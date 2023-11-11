use serde::Deserialize;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

use super::constants::TrackerModel;

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
