use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use utoipa::ToSchema;
use validator::Validate;

#[derive(TryFromMultipart, ToSchema, Validate)]
#[try_from_multipart(rename_all = "camelCase")]
pub struct CreateVehicleDto {
    #[schema(value_type = String, format = Binary)]
    pub photo: Option<FieldData<Bytes>>,

    // TODO: validation (this is not validated automagically like validatedJson)
    // #[validate(email)]
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
