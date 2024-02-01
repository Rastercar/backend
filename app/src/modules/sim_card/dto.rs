use serde::Deserialize;
use utoipa::IntoParams;
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
