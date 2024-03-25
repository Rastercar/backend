use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(ToSchema, Validate, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrganizationDto {
    #[validate(email)]
    pub billing_email: Option<String>,

    #[validate(length(min = 5, max = 32))]
    pub name: Option<String>,
}
