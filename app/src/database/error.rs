use crate::modules::common::responses::{internal_error_res, SimpleError};
use http::StatusCode;
use sea_orm::{DbErr, RuntimeErr, SqlxError};

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

fn handle_sqlx_error(sqlx_error: SqlxError) -> (StatusCode, SimpleError) {
    match sqlx_error {
        SqlxError::Database(e) => match e.code() {
            Some(postgres_error_code) => {}
            None => todo!(),
        },
        _ => internal_error_res(),
    }
}

impl From<DbError> for (StatusCode, SimpleError) {
    fn from(err: DbError) -> Self {
        dbg!("=============================");
        dbg!(&err.0);

        match err.0 {
            DbErr::RecordNotFound(_) => {
                (StatusCode::NOT_FOUND, SimpleError::from("entity not found"))
            }

            DbErr::Exec(RuntimeErr::SqlxError(error)) => handle_sqlx_error(error),
            DbErr::Query(RuntimeErr::SqlxError(error)) => handle_sqlx_error(error),

            _ => internal_error_res(),
        }
    }
}
