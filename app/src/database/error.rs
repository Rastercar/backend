use crate::modules::common::responses::{internal_error_res, SimpleError};
use convert_case::{Case, Casing};
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

impl From<DbError> for (StatusCode, SimpleError) {
    fn from(err: DbError) -> Self {
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

fn handle_sqlx_error(sqlx_error: SqlxError) -> (StatusCode, SimpleError) {
    match sqlx_error {
        SqlxError::Database(e) => {
            if !e.is_unique_violation() {
                return internal_error_res();
            }

            if let Some(constraint) = e.constraint() {
                if let Some(column_name) = get_column_name_from_unique_constraint_name(constraint) {
                    let snake_cased_col_name = column_name.to_case(Case::ScreamingSnake);

                    let error_msg = format!("{}_IN_USE", snake_cased_col_name);

                    return (StatusCode::BAD_REQUEST, SimpleError::from(error_msg));
                }
            }

            internal_error_res()
        }
        _ => internal_error_res(),
    }
}

/// Extracts the column name from the name of a database unique constraint.
/// assuming the naming pattern: `<table_name>_<column>_unique`.
///
/// returns `Some(<column>)` if the pattern is ok otherwise `None`.
fn get_column_name_from_unique_constraint_name(unique_constraint_name: &str) -> Option<&str> {
    if let Some(non_suffixed_constraint_name) = unique_constraint_name.strip_suffix("_unique") {
        return non_suffixed_constraint_name.split('_').last();
    }

    None
}
