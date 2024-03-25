use crate::modules::common::validators::REGEX_IS_MERCOSUL_OR_BR_VEHICLE_PLATE;
use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ListVehiclesDto {
    /// Search by plate
    pub plate: Option<String>,
}

#[derive(TryFromMultipart, ToSchema, Validate)]
#[try_from_multipart(rename_all = "camelCase")]
pub struct CreateVehicleDto {
    #[schema(value_type = String, format = Binary)]
    pub photo: Option<FieldData<Bytes>>,

    #[validate(regex(
        path = "REGEX_IS_MERCOSUL_OR_BR_VEHICLE_PLATE",
        message = "vehicle plate must be in format AAA#A## or AAA#### (A: a-z, #: 0-9)"
    ))]
    pub plate: String,

    pub brand: String,

    pub model: String,

    pub color: Option<String>,

    #[validate(range(min = 1900, max = 2100))]
    pub model_year: Option<i16>,

    pub chassis_number: Option<String>,

    #[validate(range(min = 1900, max = 2100))]
    pub fabrication_year: Option<i16>,

    pub additional_info: Option<String>,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVehicleDto {
    #[validate(regex(
        path = "REGEX_IS_MERCOSUL_OR_BR_VEHICLE_PLATE",
        message = "vehicle plate must be in format AAA#A## or AAA#### (A: a-z, #: 0-9)"
    ))]
    pub plate: Option<String>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub brand: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub model: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub color: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub chassis_number: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub additional_info: Option<Option<String>>,

    #[validate(range(min = 1900, max = 2100))]
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub model_year: Option<Option<i16>>,

    #[validate(range(min = 1900, max = 2100))]
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub fabrication_year: Option<Option<i16>>,
}
