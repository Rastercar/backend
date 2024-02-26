use crate::modules::common::validators::{
    REGEX_CONTAINS_LOWERCASE_CHARACTER, REGEX_CONTAINS_NUMBER, REGEX_CONTAINS_SYMBOLIC_CHARACTER,
    REGEX_CONTAINS_UPPERCASE_CHARACTER, REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ListUsersDto {
    /// Search by email
    pub email: Option<String>,

    /// Search by access level
    pub access_level_id: Option<i32>,
}

#[derive(ToSchema, Validate, Deserialize)]
pub struct UpdateUserDto {
    #[validate(email)]
    pub email: Option<String>,

    #[validate(regex(
        path = "REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES",
        message = "username must contain only lowercase alphanumeric characters and underscores"
    ))]
    #[validate(length(min = 5, max = 32))]
    pub username: Option<String>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    #[validate(length(max = 500))]
    pub description: Option<Option<String>>,
}

#[derive(ToSchema, Validate, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordDto {
    pub old_password: String,

    #[validate(length(min = 5, max = 256))]
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
    pub new_password: String,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = user::dto::SimpleUserDto)]
pub struct SimpleUserDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub profile_picture: Option<String>,
    pub description: Option<String>,
}

impl From<entity::user::Model> for SimpleUserDto {
    fn from(m: entity::user::Model) -> Self {
        Self {
            id: m.id,
            email: m.email,
            username: m.username,
            created_at: m.created_at,
            description: m.description,
            email_verified: m.email_verified,
            profile_picture: m.profile_picture,
        }
    }
}
