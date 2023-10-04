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
    static ref REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES: Regex =
        Regex::new(r"^[a-z0-9_]+$").unwrap();
}

// --- INPUT

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RegisterOrganization {
    #[validate(regex(
        path = "REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES",
        message = "username must contain only lowercase alphanumeric characters and underscores"
    ))]
    #[validate(length(min = 5, max = 32))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 5, max = 128))]
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

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SignIn {
    #[validate(length(min = 5, max = 128))]
    pub password: String,

    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct ForgotPassword {
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResetPassword {
    #[validate(length(min = 5, max = 128))]
    #[validate(regex(
        path = "REGEX_CONTAINS_NUMBER",
        message = "new password must contain a number"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_SYMBOLIC_CHARACTER",
        message = "new password must contain a symbol in: #?!@$%^&*-"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_UPPERCASE_CHARACTER",
        message = "new password must contain a uppercase character"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_LOWERCASE_CHARACTER",
        message = "new password must contain a lowercase character"
    ))]
    pub new_password: String,

    pub password_reset_token: String,
}

// --- OUTPUT

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignInResponse {
    pub user: UserDto,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessLevelDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub is_fixed: bool,
    pub permissions: Vec<Option<String>>,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub billing_email: String,
    pub blocked: bool,
    pub name: String,
    pub billing_email_verified: bool,
}

/// A rastercar user with his organization and access level
#[derive(Serialize, Clone, ToSchema)]
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
    pub organization: Option<OrganizationDto>,
    pub access_level: AccessLevelDto,
}

impl From<models::Organization> for OrganizationDto {
    fn from(m: models::Organization) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at,
            updated_at: m.updated_at,
            billing_email: m.billing_email,
            name: m.name,
            blocked: m.blocked,
            billing_email_verified: m.billing_email_verified,
        }
    }
}

impl From<models::AccessLevel> for AccessLevelDto {
    fn from(m: models::AccessLevel) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at,
            updated_at: m.updated_at,
            name: m.name,
            description: m.description,
            is_fixed: m.is_fixed,
            permissions: m.permissions,
        }
    }
}
