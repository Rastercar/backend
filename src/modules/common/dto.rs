use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
pub struct EmailAddress {
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct Token {
    #[validate(length(min = 5))]
    pub token: String,
}
