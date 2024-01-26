use crate::modules::common::responses::{internal_error_res, SimpleError};
use http::StatusCode;
use sea_orm::DbErr;

/// Wrapper for seaorm errors.
///
/// This is useful for wrapping database errors and safely returning them from
/// axum route handlers without worrying about leaking sensitive information,
/// as it implements `Into<(StatusCode, SimpleError)>`
pub struct DbError(pub DbErr);

impl From<DbErr> for DbError {
    fn from(err: DbErr) -> Self {
        DbError(err)
    }
}

impl From<DbError> for (StatusCode, SimpleError) {
    fn from(err: DbError) -> Self {
        match err.0 {
            DbErr::RecordNotFound(_) => {
                (StatusCode::NOT_FOUND, SimpleError::from("entity not found"))
            }

            _ => internal_error_res(),
        }
    }
}
