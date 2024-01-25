use axum::{
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use validator::ValidationErrors;

/// A struct for simple API error responses, contains a timestamp from the moment
/// of its creation and a error message
///
/// its meant to be sent as JSON so its `IntoResponse` implementation will set the
/// response body to JSON
#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SimpleError {
    error: String,
    timestamp: DateTime<Utc>,
}

impl SimpleError {
    /// Creates a simple error with a generic 'internal server error message'
    /// ideally this should be used whenever something that should almost never
    /// fail on the request lifecycle does fail.
    pub fn internal() -> SimpleError {
        SimpleError::from("internal server error")
    }
}

impl From<String> for SimpleError {
    fn from(v: String) -> Self {
        SimpleError {
            error: v,
            timestamp: Utc::now(),
        }
    }
}

impl IntoResponse for SimpleError {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl From<ValidationErrors> for SimpleError {
    fn from(v: ValidationErrors) -> Self {
        SimpleError::from(v.to_string())
    }
}

impl From<anyhow::Error> for SimpleError {
    /// since anyhow errors might contain private error messages such as DB errors
    /// or a stack description, always convert to a generic internal error
    fn from(_: anyhow::Error) -> Self {
        SimpleError::internal()
    }
}

impl From<&str> for SimpleError {
    fn from(v: &str) -> Self {
        SimpleError::from(String::from(v))
    }
}

pub fn internal_error_res() -> (StatusCode, SimpleError) {
    (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal())
}

pub fn internal_error_msg(msg: &str) -> (StatusCode, SimpleError) {
    (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::from(msg))
}
