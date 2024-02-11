use serde::Deserialize;
use shared::TrackerModel;
use utoipa::{IntoParams, ToSchema};
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
pub struct UpdateTrackerDto {
    pub imei: Option<String>,

    pub model: Option<TrackerModel>,
}

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
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
    /// Vehicle ID to associate the SIM card to
    ///
    /// we use the `Option<Option<i32>>` format here to distinguish
    /// between `undefined` and `null` values when parsing JSON
    /// to avoid wrongfully interpreting `tracker_id` as `null`
    /// when the key is not present in the request body main object.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[validate(required)]
    pub vehicle_id: Option<Option<i32>>,
}

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct DeleteTrackerDto {
    /// If the sim cards associated with the tracker to be deleted, should be deleted aswell
    pub delete_associated_sim_cards: Option<bool>,
}
