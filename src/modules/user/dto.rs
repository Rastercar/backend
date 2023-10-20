use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use diesel::query_builder::AsChangeset;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::database::schema::user;

/// DTO to update user profile pictures, should be extracted from `multipart/form-data`
/// requests containing a single field `image` field
#[derive(TryFromMultipart, ToSchema)]
pub struct ProfilePicDto {
    #[schema(value_type = String, format = Binary)]
    pub image: FieldData<Bytes>,
}

#[derive(ToSchema, Validate, Deserialize, AsChangeset)]
#[diesel(table_name = user)]
pub struct UpdateUserDto {
    #[validate(email)]
    pub email: Option<String>,

    #[validate(length(min = 5, max = 32))]
    pub username: Option<String>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    #[validate(length(max = 500))]
    pub description: Option<Option<String>>,
}
