use super::dto;
use super::middleware::RequestUser;
use super::service::UserFromCredentialsError;
use super::session::{OptionalSessionToken, SessionToken};
use crate::database::models;
use crate::modules::common::extractors::ValidatedJson;
use crate::modules::common::{error_codes, responses::SimpleError};
use crate::server::controller::AppState;
use anyhow::Result;
use axum::extract::Path;
use axum::headers::UserAgent;
use axum::{extract::State, http::StatusCode, routing::post, Extension, Json, Router, TypedHeader};
use axum_client_ip::SecureClientIp;
use http::HeaderMap;

pub fn create_auth_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/sign-out", post(logout))
        .route("/sign-out/:session-id", post(logout_session_by_id))
        .layer(axum::middleware::from_fn_with_state(
            state,
            super::middleware::user_only_route,
        ))
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
    };

    (headers, Json(res_body))
}

async fn logout(
    session: Extension<SessionToken>,
    State(state): State<AppState>,
) -> Result<(StatusCode, HeaderMap), (StatusCode, SimpleError)> {
    let session_token = session.0;

    state
        .auth_service
        .delete_session(session_token)
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to delete session"),
        )))?;

    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", session_token.into_delete_cookie_header());

    Ok((StatusCode::OK, headers))
}

async fn logout_session_by_id(
    req_user: Extension<RequestUser>,
    req_user_session: Extension<SessionToken>,
    Path(session_id): Path<u128>,
    State(state): State<AppState>,
) -> Result<(StatusCode, HeaderMap), (StatusCode, SimpleError)> {
    let session_to_delete = SessionToken::from(session_id);
    let request_user = req_user.0 .0;

    let maybe_user_on_session_to_delete = state
        .auth_service
        .get_user_from_session_token(session_to_delete)
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::internal(),
        )))?;

    match maybe_user_on_session_to_delete {
        None => Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from("session does not exist"),
        )),
        Some(user_on_session_to_delete) => {
            if user_on_session_to_delete.id != request_user.id {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    SimpleError::from("session does not belong to the request user"),
                ));
            }

            state
                .auth_service
                .delete_session(session_to_delete)
                .await
                .or(Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    SimpleError::from("failed to delete session"),
                )))?;

            let mut headers = HeaderMap::new();

            if req_user_session.get_id() == session_to_delete.get_id() {
                headers.insert("Set-Cookie", session_to_delete.into_delete_cookie_header());
            }

            Ok((StatusCode::OK, headers))
        }
    }
}

async fn sign_in(
    client_ip: SecureClientIp,
    old_session_token: OptionalSessionToken,
    State(state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ValidatedJson(payload): ValidatedJson<dto::SignIn>,
) -> Result<(HeaderMap, Json<dto::SignInResponse>), (StatusCode, SimpleError)> {
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

    if let Some(old_ses_token) = old_session_token.get_value() {
        state.auth_service.delete_session(old_ses_token).await.ok();
    }

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
