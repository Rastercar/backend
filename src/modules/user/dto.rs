use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use utoipa::ToSchema;

/// DTO to update user profile pictures, should be extracted from `multipart/form-data`
/// requests containing a single field `image` field
#[derive(TryFromMultipart, ToSchema)]
pub struct ProfilePicDto {
    #[schema(value_type = String, format = Binary)]
    pub image: FieldData<Bytes>,
}
