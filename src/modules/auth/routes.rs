use super::dto::{self, UserDto};
use super::middleware::RequestUser;
use super::session::{OptionalSessionToken, SessionToken};
use crate::database::models::{self};
use crate::modules::common::extractors::ValidatedJson;
use crate::modules::common::{error_codes, responses::SimpleError};
use crate::server::controller::AppState;
use anyhow::Result;
use axum::extract::Path;
use axum::headers::UserAgent;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router, TypedHeader,
};
use axum_client_ip::SecureClientIp;
use http::HeaderMap;

pub fn create_auth_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/sign-out", post(sign_out))
        .route("/sign-out/:session-id", post(sign_out_session_by_id))
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

/// Returns the request user
///
/// the request user is the user that owns the session on the session id (sid) cookie
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = User,
        ),
        (
            status = UNAUTHORIZED,
            description = "session not found",
            body = SimpleError,
        ),
    ),
)]
pub async fn me(req_user: Extension<RequestUser>) -> Json<UserDto> {
    Json(UserDto::from(req_user.0 .0))
}

/// Signs out of the current user session
///
/// signs out by deleting the user session present in the sid (session id)
/// request cookie
#[utoipa::path(
    post,
    path = "/auth/sign-out",
    tag = "auth",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            description = "sign out successful",
            headers(("Set-Cookie" = String, description = "expired cookie sid, so the client browser deletes the cookie"))
        ),
        (
            status = UNAUTHORIZED,
            description = "session not found",
            body = SimpleError,
        ),
    ),
)]
pub async fn sign_out(
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

/// Signs out of a session by its id
///
/// deletes the user session with the provided ID
#[utoipa::path(
    post,
    path = "/auth/sign-out/{session_id}",
    tag = "auth",
    params(
        ("session_id" = u128, Path, description = "id of the session to delete"),
    ),
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            description = "sign out successful",
            headers(("Set-Cookie" = String, description = "expired cookie sid, returned if the deleted session equals the request one"))
        ),
        (
            status = UNAUTHORIZED,
            description = "request does not contain a valid session or the session to be deleted does not belong to the user",
            body = SimpleError,
        ),
    ),
)]
async fn sign_out_session_by_id(
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

/// Signs in
///
/// Sign in by credentials (email, password)
#[utoipa::path(
    post,
    path = "/auth/sign-in",
    tag = "auth",
    request_body = SignIn,
    responses(
        (
            status = OK,
            description = "sign in successful",
            body = SignInResponse,
            headers(("Set-Cookie" = String, description = "new session id cookie"))
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto",
            body = SimpleError,
        ),
        (
            status = NOT_FOUND,
            description = "user with email not found",
            body = SimpleError,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid password",
            body = SimpleError,
        ),
    ),
)]
pub async fn sign_in(
    client_ip: SecureClientIp,
    old_session_token: OptionalSessionToken,
    State(state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ValidatedJson(payload): ValidatedJson<dto::SignIn>,
) -> Result<(HeaderMap, Json<dto::SignInResponse>), (StatusCode, SimpleError)> {
    use super::service::UserFromCredentialsError as Err;

    let user = state
        .auth_service
        .get_user_from_credentials(payload.email, payload.password)
        .await
        .map_err(|e| match e {
            Err::NotFound => (StatusCode::NOT_FOUND, SimpleError::from("user not found")),
            Err::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal()),
            Err::InvalidPassword => (
                StatusCode::UNAUTHORIZED,
                SimpleError::from("invalid password"),
            ),
        })?;

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

/// Signs up a new user rastercar user
///
/// creates the user, his organization and root access level, returning the created user
/// and his new session cookie.
#[utoipa::path(
    post,
    path = "/auth/sign-up",
    tag = "auth",
    request_body = RegisterOrganization,
    responses(
        (
            status = OK,
            description = "sign up successful",
            body = SignInResponse,
            headers(("Set-Cookie" = String, description = "new session id cookie"))
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message or / EMAIL_IN_USE error code, when a provided email address is in use by another entity",
            body = SimpleError,
        ),
    ),
)]
pub async fn sign_up(
    client_ip: SecureClientIp,
    State(state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ValidatedJson(payload): ValidatedJson<dto::RegisterOrganization>,
) -> Result<(HeaderMap, Json<dto::SignInResponse>), (StatusCode, SimpleError)> {
    let internal_err_res = (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal());

    let email_in_use = state
        .auth_service
        .check_email_in_use(payload.email.clone())
        .await
        .or(Err(internal_err_res.clone()))?;

    if email_in_use {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(error_codes::EMAIL_IN_USE),
        ));
    }

    let username_in_use = state
        .auth_service
        .get_user_id_by_username(payload.username.clone())
        .await
        .or(Err(internal_err_res.clone()))?
        .is_some();

    if username_in_use {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(error_codes::USERNAME_IN_USE),
        ));
    }

    let created_user = state
        .auth_service
        .register_user_and_organization(payload)
        .await
        .or(Err(internal_err_res))?;

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
