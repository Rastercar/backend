use super::dto;
use crate::{
    http::controller::AppState,
    modules::common::{error_codes, responses::SimpleError},
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use validator::Validate;

pub fn create_auth_router() -> Router<AppState> {
    Router::new().route("/register-organization", post(register_organization))
}

async fn register_organization(
    State(state): State<AppState>,
    Json(payload): Json<dto::RegisterUser>,
) -> Result<impl IntoResponse, (StatusCode, SimpleError)> {
    match payload.validate() {
        Ok(_) => {}
        Err(e) => return Err(((StatusCode::BAD_REQUEST), SimpleError::from(e))),
    }

    let internal_server_error_response =
        (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal());

    let email_in_use = state
        .auth_service
        .check_email_in_use(payload.email.clone())
        .await
        .or(Err(internal_server_error_response.clone()))?;

    if email_in_use {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(error_codes::EMAIL_IN_USE),
        ));
    }

    let created_user = state
        .auth_service
        .register_user_and_organization(payload)
        .await
        .or(Err(internal_server_error_response.clone()))?;

    state
        .auth_service
        .login_for_user(created_user, true)
        .await
        .or(Err(internal_server_error_response))?;

    // TODO: !
    Ok(String::from("ok!"))
}
