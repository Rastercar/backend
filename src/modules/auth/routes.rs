use super::dto;
use super::service::UserFromCredentialsError;
use super::session::{OptionalSessionToken, SessionToken};
use crate::database::models;
use crate::modules::common::extractors::ValidatedJson;
use crate::modules::common::{error_codes, responses::SimpleError};
use crate::server::controller::AppState;
use anyhow::Result;
use axum::headers::UserAgent;
use axum::{extract::State, http::StatusCode, routing::post, Router};
use axum::{Json, TypedHeader};
use axum_client_ip::SecureClientIp;
use http::HeaderMap;

pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/sign-up", post(sign_up))
        .route("/sign-in", post(sign_in))
}

fn sign_in_or_up_response(
    user: models::User,
    ses_token: SessionToken,
) -> (HeaderMap, Json<dto::SignInResponse>) {
    let mut headers = HeaderMap::new();

    headers.insert("Set-Cookie", ses_token.into_set_cookie_header());

    let res_body = dto::SignInResponse {
        user: dto::UserDto::from(user),
        message: String::from("login successful"),
    };

    (headers, Json(res_body))
}

async fn sign_in(
    client_ip: SecureClientIp,
    session_token: OptionalSessionToken,
    State(state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ValidatedJson(payload): ValidatedJson<dto::SignIn>,
) -> Result<(HeaderMap, Json<dto::SignInResponse>), (StatusCode, SimpleError)> {
    if session_token.0.is_some() {
        return Err((
            (StatusCode::BAD_REQUEST),
            SimpleError::from("already signed in"),
        ));
    }

    let user = state
        .auth_service
        .get_user_from_credentials(payload.email, payload.password)
        .await
        .map_err(|e| match e {
            UserFromCredentialsError::NotFound => {
                (StatusCode::NOT_FOUND, SimpleError::from("user not found"))
            }
            UserFromCredentialsError::InternalError => {
                (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal())
            }
            UserFromCredentialsError::InvalidPassword => (
                StatusCode::UNAUTHORIZED,
                SimpleError::from("invalid password"),
            ),
        })?;

    state
        .auth_service
        .delete_unregistered_users_by_email(&user.email)
        .await
        .ok();

    let session_token = state
        .auth_service
        .new_session(
            state.db_conn_pool,
            user.id,
            client_ip.0,
            user_agent.to_string(),
        )
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to create session"),
        )))?;

    Ok(sign_in_or_up_response(user, session_token))
}

async fn sign_up(
    client_ip: SecureClientIp,
    State(state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ValidatedJson(payload): ValidatedJson<dto::RegisterOrganization>,
) -> Result<(HeaderMap, Json<dto::SignInResponse>), (StatusCode, SimpleError)> {
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

    let session_token = state
        .auth_service
        .new_session(
            state.db_conn_pool,
            created_user.id,
            client_ip.0,
            user_agent.to_string(),
        )
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to create session"),
        )))?;

    Ok(sign_in_or_up_response(created_user, session_token))
}
