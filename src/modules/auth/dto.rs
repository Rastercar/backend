use crate::database::models;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

lazy_static! {
    static ref REGEX_CONTAINS_NUMBER: Regex = Regex::new(r"[0-9]").unwrap();
    static ref REGEX_CONTAINS_UPPERCASE_CHARACTER: Regex = Regex::new(r"[A-Z]").unwrap();
    static ref REGEX_CONTAINS_LOWERCASE_CHARACTER: Regex = Regex::new(r"[a-z]").unwrap();
    static ref REGEX_CONTAINS_SYMBOLIC_CHARACTER: Regex = Regex::new(r"[#?!@$%^&*-]").unwrap();
}

#[derive(Deserialize, Serialize, Validate, Debug, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RegisterOrganization {
    #[validate(length(min = 5, max = 60))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 5, max = 120))]
    #[validate(regex(
        path = "REGEX_CONTAINS_NUMBER",
        message = "password must contain a number"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_SYMBOLIC_CHARACTER",
        message = "password must contain a symbol in: #?!@$%^&*-"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_UPPERCASE_CHARACTER",
        message = "password must contain a uppercase character"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_LOWERCASE_CHARACTER",
        message = "password must contain a lowercase character"
    ))]
    pub password: String,
}

#[derive(Deserialize, Serialize, Validate, Debug, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SignIn {
    #[validate(length(min = 5, max = 400))]
    pub password: String,

    #[validate(email)]
    pub email: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignInResponse {
    pub user: UserDto,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub profile_picture: Option<String>,
    pub description: Option<String>,
    pub organization_id: i32,
    pub access_level_id: i32,
}

impl From<models::User> for UserDto {
    fn from(value: models::User) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            username: value.username,
            email: value.email,
            email_verified: value.email_verified,
            profile_picture: value.profile_picture,
            description: value.description,
            organization_id: value.organization_id,
            access_level_id: value.access_level_id,
        }
    }
}
