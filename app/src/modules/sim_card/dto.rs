use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ListSimCardsDto {
    /// Search SIM cards by phone
    pub phone_number: Option<String>,

    /// If the sim cards should be filtered if they are associated
    /// to a tracker or not, `None` means `any`
    pub with_associated_tracker: Option<bool>,
}

// TODO: rm debug
#[derive(Deserialize, ToSchema, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetSimCardTrackerDto {
    /// Tracker ID to associate the SIM card to
    ///
    /// we use the `Option<Option<i32>>` format here to distinguish
    /// between `undefined` and `null` values when parsing JSON
    /// to avoid wrongfully interpreting `tracker_id` as `null`
    /// when the key is not present in the request body main object.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[validate(required)]
    pub tracker_id: Option<Option<i32>>,
}
