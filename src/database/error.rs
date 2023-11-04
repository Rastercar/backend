use crate::modules::common::responses::{internal_error_response, SimpleError};
use convert_case::{Case, Casing};
use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};
use http::StatusCode;

/// Wrapper for diesel errors.
///
/// This is useful for wrapping database errors and safely returning them from
/// axum route handlers without worrying about leaking sensitive information.   
pub struct DbError(DieselError);

impl From<DieselError> for DbError {
    fn from(err: DieselError) -> Self {
        DbError(err)
    }
}

impl From<DbError> for (StatusCode, SimpleError) {
    fn from(err: DbError) -> Self {
        match err.0 {
            DieselError::DatabaseError(db_err, info) => {
                if let DatabaseErrorKind::UniqueViolation = db_err {
                    if let Some(column_name) = get_column_name_from_db_error_info(info.as_ref()) {
                        let snake_cased_col_name = column_name.to_case(Case::ScreamingSnake);

                        let error_msg = format!("{}_IN_USE", snake_cased_col_name);

                        return (StatusCode::BAD_REQUEST, SimpleError::from(error_msg));
                    }
                }

                internal_error_response()
            }

            DieselError::NotFound => (StatusCode::NOT_FOUND, SimpleError::from("entity not found")),

            _ => internal_error_response(),
        }
    }
}

/// Extracts the column name from the name of a database unique constraint.
/// assuming the naming pattern: `<table_name>_<column>_unique`.
///
/// returns `Some(<column>)` if the pattern is ok otherwise `None`.
fn get_column_name_from_unique_constraint_name(unique_constraint_name: &str) -> Option<&str> {
    if let Some(non_suffixed_constraint_name) = unique_constraint_name.strip_suffix("_unique") {
        return non_suffixed_constraint_name.split("_").last();
    }

    None
}

/// Returns the column name from the database error information
///
/// - if the error contains the column name, returns it.
///
/// - if the error is from a unique constraint, returns the column name
/// inside the unique constraint.
///
/// otherwise returns `None`
fn get_column_name_from_db_error_info(info: &dyn DatabaseErrorInformation) -> Option<&str> {
    let err_col_name = info.column_name();

    if err_col_name.is_some() {
        return err_col_name;
    }

    if let Some(constraint_name) = info.constraint_name() {
        return get_column_name_from_unique_constraint_name(constraint_name);
    }

    return None;
}
