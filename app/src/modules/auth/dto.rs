use crate::modules::common::validators::{
    REGEX_CONTAINS_LOWERCASE_CHARACTER, REGEX_CONTAINS_NUMBER, REGEX_CONTAINS_SYMBOLIC_CHARACTER,
    REGEX_CONTAINS_UPPERCASE_CHARACTER, REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// --- INPUT

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterOrganization {
    #[validate(regex(
        path = "REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES",
        message = "username must contain only lowercase alphanumeric characters and underscores"
    ))]
    #[validate(length(min = 5, max = 32))]
    pub username: String,

    #[validate(email)]
    pub email: String,

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
    pub password: String,
}

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignIn {
    #[validate(length(min = 5, max = 256))]
    pub password: String,

    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResetPassword {
    #[validate(length(min = 5, max = 256))]
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

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub ip: String,
    pub public_id: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: String,

    /// if this session is the same that was used on the request that is returning this
    pub same_as_from_request: bool,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessLevelDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub is_fixed: bool,
    pub permissions: Vec<String>,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub blocked: bool,
    pub name: String,
    pub billing_email: String,
    pub billing_email_verified: bool,
}

/// A rastercar user with his organization and access level
#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub profile_picture: Option<String>,
    pub description: Option<String>,
    pub organization: Option<OrganizationDto>,
    pub access_level: AccessLevelDto,
}

impl From<entity::organization::Model> for OrganizationDto {
    fn from(m: entity::organization::Model) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at.into(),
            billing_email: m.billing_email,
            name: m.name,
            blocked: m.blocked,
            billing_email_verified: m.billing_email_verified,
        }
    }
}

impl From<entity::session::Model> for SessionDto {
    fn from(m: entity::session::Model) -> Self {
        Self {
            ip: m.ip.to_string(),
            public_id: m.public_id,
            user_agent: m.user_agent,
            created_at: m.created_at.into(),
            expires_at: m.expires_at.into(),
            same_as_from_request: false,
        }
    }
}

impl From<entity::access_level::Model> for AccessLevelDto {
    fn from(m: entity::access_level::Model) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at.into(),
            name: m.name,
            description: m.description,
            is_fixed: m.is_fixed,
            permissions: m.permissions,
        }
    }
}
