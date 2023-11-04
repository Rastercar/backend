use crate::modules::common::validators::REGEX_IS_MERCOSUL_OR_BR_VEHICLE_PLATE;
use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use utoipa::ToSchema;
use validator::Validate;

#[derive(TryFromMultipart, ToSchema, Validate)]
#[try_from_multipart(rename_all = "camelCase")]
pub struct CreateVehicleDto {
    // TODO: this returns error on large files (same for Update Profile Picture DTO)
    // we must find a way to return some error code / message
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
