use axum::body::Bytes;
use axum_typed_multipart::FieldData;
use http::StatusCode;

use super::responses::SimpleError;

/// asserts a multipart/form-data field is a image with a valid extension, returning the extension
pub fn get_image_extension_from_field_or_fail_request(
    field: &FieldData<Bytes>,
) -> Result<String, (StatusCode, SimpleError)> {
    let file_name = field
        .metadata
        .file_name
        .clone()
        .ok_or((StatusCode::BAD_REQUEST, SimpleError::from("empty filename")))?;

    let allowed_file_types = vec!["jpe", "jpg", "jpeg", "png", "webp"];

    let (_, file_extension) = file_name.rsplit_once('.').ok_or((
        StatusCode::BAD_REQUEST,
        SimpleError::from("empty file extension"),
    ))?;

    if allowed_file_types.contains(&file_extension) {
        Ok(String::from(file_extension))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from("invalid file extension"),
        ))
    }
}

/// validates field is a image and creates filename from a uploaded photo with the following format:
///
/// `<prefix>_<now_timestamp>_<uploaded_file_extension>`
///
/// eg: photo_02-10-2023_10:20:59.jpeg
pub fn filename_from_img(
    prefix: &str,
    img: &FieldData<Bytes>,
) -> Result<String, (StatusCode, SimpleError)> {
    let file_extension = get_image_extension_from_field_or_fail_request(img)?;

    let timestamp = chrono::Utc::now().format("%d-%m-%Y_%H:%M:%S");

    Ok(format!("{}_{}.{}", prefix, timestamp, file_extension))
}
