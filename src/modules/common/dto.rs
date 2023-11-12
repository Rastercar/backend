use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
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

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    10
}

#[derive(Deserialize, IntoParams, Validate)]
pub struct Pagination {
    #[serde(default = "default_page")]
    #[validate(range(min = 1, max = 99999))]
    pub page: usize,

    #[serde(default = "default_page_size")]
    #[validate(range(min = 1, max = 100))]
    pub page_size: usize,
}
