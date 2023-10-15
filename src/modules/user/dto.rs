use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};

/// DTO to update user profile pictures, should be extracted from `multipart/form-data`
/// requests containing a single field `image` field
#[derive(TryFromMultipart)]
pub struct ProfilePicDto {
    pub image: FieldData<Bytes>,
}
