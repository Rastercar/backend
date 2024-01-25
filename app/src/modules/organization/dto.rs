use crate::database::schema::organization;
use diesel::query_builder::AsChangeset;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(ToSchema, Validate, Deserialize, AsChangeset)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = organization)]
pub struct UpdateOrganizationDto {
    #[validate(email)]
    pub billing_email: Option<String>,

    #[validate(length(min = 5, max = 32))]
    pub name: Option<String>,
}
