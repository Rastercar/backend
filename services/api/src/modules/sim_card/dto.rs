use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateSimCardDto {
    #[validate(length(min = 1))]
    pub ssn: String,

    pub phone_number: String,

    pub apn_user: String,
    pub apn_address: String,
    pub apn_password: String,

    pub pin: Option<String>,
    pub pin2: Option<String>,

    pub puk: Option<String>,
    pub puk2: Option<String>,

    /// ID of the vehicle to associate with the tracker
    #[validate(range(min = 1))]
    pub vehicle_tracker_id: Option<i32>,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSimCardDto {
    pub ssn: Option<String>,

    pub phone_number: Option<String>,

    pub apn_user: Option<String>,

    pub apn_address: Option<String>,

    pub apn_password: Option<String>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub pin: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub pin2: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub puk: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub puk2: Option<Option<String>>,
}

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

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SetSimCardTrackerDto {
    /// Tracker ID to associate the SIM card to
    ///
    /// we use the `Option<Option<i32>>` format here to distinguish
    /// between `undefined` and `null` values when parsing JSON
    /// to avoid wrongfully interpreting `vehicle_tracker_id` as `null`
    /// when the key is not present in the request body main object.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[validate(required)]
    pub vehicle_tracker_id: Option<Option<i32>>,
}
