use crate::database::schema::user;
use crate::modules::common::validators::{
    REGEX_CONTAINS_LOWERCASE_CHARACTER, REGEX_CONTAINS_NUMBER, REGEX_CONTAINS_SYMBOLIC_CHARACTER,
    REGEX_CONTAINS_UPPERCASE_CHARACTER, REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES,
};
use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use diesel::query_builder::AsChangeset;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// DTO to update user profile pictures, should be extracted from `multipart/form-data`
/// requests containing a single field `image` field
#[derive(TryFromMultipart, ToSchema)]
pub struct ProfilePicDto {
    #[schema(value_type = String, format = Binary)]
    pub image: FieldData<Bytes>,
}

// TODO: RM AS CHANGESET

#[derive(ToSchema, Validate, Deserialize, AsChangeset)]
#[diesel(table_name = user)]
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
