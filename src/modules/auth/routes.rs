use super::dto::{self, SessionDto};
use super::jwt;
use super::middleware::RequestUser;
use super::session::{OptionalSessionId, SessionId};
use crate::database::models::{self};
use crate::database::schema::session;
use crate::database::schema::user::{self};
use crate::modules::common;
use crate::modules::common::error_codes::EMAIL_ALREADY_VERIFIED;
use crate::modules::common::extractors::{DbConnection, ValidatedJson};
use crate::modules::common::responses::{
    internal_error_response, internal_error_response_with_msg,
};
use crate::modules::common::{error_codes, responses::SimpleError};
use crate::server::controller::AppState;
use anyhow::Result;
use axum::extract::Path;
use axum::headers::UserAgent;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router, TypedHeader,
};
use axum_client_ip::SecureClientIp;
use bcrypt::{hash, DEFAULT_COST};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::HeaderMap;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sign-out", post(sign_out))
        .route(
            "/sign-out/:public-session-id",
            delete(sign_out_session_by_id),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            super::middleware::require_user,
        ))
        .route("/sign-up", post(sign_up))
        .route("/sign-in", post(sign_in))
        .route(
            "/request-recover-password-email",
            post(request_recover_password_email),
        )
        .route(
            "/change-password-by-recovery-token",
            post(change_password_by_recovery_token),
        )
        .route(
            "/confirm-email-address-by-token",
            post(confirm_email_address_by_token),
        )
}

fn sign_in_or_up_response(
    user: dto::UserDto,
    ses_token: SessionId,
) -> (HeaderMap, Json<dto::SignInResponse>) {
    let mut headers = HeaderMap::new();

    headers.insert("Set-Cookie", ses_token.into_set_cookie_header());

    let res_body = dto::SignInResponse { user };

    (headers, Json(res_body))
}

/// List all sessions for the request user
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = Vec<SessionDto>,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn list_sessions(
    Extension(session): Extension<SessionId>,
    Extension(req_user): Extension<RequestUser>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionDto>>, (StatusCode, SimpleError)> {
    let current_session_id = session.get_id();

    let sessions = state
        .auth_service
        .get_active_user_sessions(&req_user.0.id)
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to list sessions"),
        )))?
        .iter()
        .map(|s| {
            let mut session_dto = SessionDto::from(s.clone());

            let session_id = SessionId::from_database_value(s.session_token.clone())
                .expect("failed convert session id from database value")
                .get_id();

            if current_session_id == session_id {
                session_dto.same_as_from_request = true
            }

            session_dto
        })
        .collect();

    Ok(Json(sessions))
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
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn sign_out(
    Extension(session): Extension<SessionId>,
    State(state): State<AppState>,
) -> Result<(StatusCode, HeaderMap), (StatusCode, SimpleError)> {
    state.auth_service.delete_session(&session).await.or(Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        SimpleError::from("failed to delete session"),
    )))?;

    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", session.into_delete_cookie_header());

    Ok((StatusCode::OK, headers))
}

/// Signs out of a session by its public id
///
/// deletes the user session with the provided public ID, a public id can be found on any endpoint that list sessions
#[utoipa::path(
    delete,
    path = "/auth/sign-out/{public_session_id}",
    tag = "auth",
    params(
        ("session_id" = u128, Path, description = "public id of the session to delete"),
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
    Extension(req_user): Extension<RequestUser>,
    Extension(req_user_session): Extension<SessionId>,
    Path(public_session_id): Path<i32>,
    DbConnection(mut conn): DbConnection,
    State(state): State<AppState>,
) -> Result<(StatusCode, HeaderMap), (StatusCode, SimpleError)> {
    let maybe_session_to_delete: Option<models::Session> = session::dsl::session
        .select(models::Session::as_select())
        .filter(session::dsl::public_id.eq(&public_session_id))
        .first::<models::Session>(&mut conn)
        .await
        .optional()
        .or(Err(internal_error_response()))?;

    if let Some(session_to_delete) = maybe_session_to_delete {
        let request_user = req_user.0;

        if session_to_delete.user_id != request_user.id {
            return Err((
                StatusCode::UNAUTHORIZED,
                SimpleError::from("session does not belong to the request user"),
            ));
        }

        let session_to_delete_id = SessionId::from_database_value(session_to_delete.session_token)
            .expect("failed to convert session id from database");

        state
            .auth_service
            .delete_session(&session_to_delete_id)
            .await
            .or(Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to delete session"),
            )))?;

        let mut headers = HeaderMap::new();

        if req_user_session.get_id() == session_to_delete_id.get_id() {
            headers.insert(
                "Set-Cookie",
                session_to_delete_id.into_delete_cookie_header(),
            );
        }

        return Ok((StatusCode::OK, headers));
    }

    Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("session does not exist"),
    ))
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
    old_session_token: OptionalSessionId,
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
            Err::InternalError => internal_error_response(),
            Err::InvalidPassword => (
                StatusCode::UNAUTHORIZED,
                SimpleError::from("invalid password"),
            ),
        })?;

    let session_token = state
        .auth_service
        .new_session(user.id, client_ip.0, user_agent.to_string())
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to create session"),
        )))?;

    if let Some(old_ses_token) = old_session_token.get_value() {
        state.auth_service.delete_session(&old_ses_token).await.ok();
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
    let email_in_use = state
        .auth_service
        .check_email_in_use(&payload.email)
        .await
        .or(Err(internal_error_response()))?;

    if email_in_use {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(error_codes::EMAIL_IN_USE),
        ));
    }

    let username_in_use = state
        .auth_service
        .get_user_id_by_username(&payload.username)
        .await
        .or(Err(internal_error_response()))?
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
        .or(Err(internal_error_response()))?;

    let session_token = state
        .auth_service
        .new_session(created_user.id, client_ip.0, user_agent.to_string())
        .await
        .or(Err(internal_error_response_with_msg(
            "failed to create session",
        )))?;

    Ok(sign_in_or_up_response(created_user, session_token))
}

/// Requests a password reset email
///
/// Sends a reset password email to the provided email address if
/// a active user account exists with it.
#[utoipa::path(
    post,
    path = "/auth/request-recover-password-email",
    tag = "auth",
    request_body = EmailAddress,
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("password recovery email queued to be sent successfully"),
        ),
        (
            status = NOT_FOUND,
            description = "the is no active user with the email address",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message",
            body = SimpleError,
        ),
    ),
)]
pub async fn request_recover_password_email(
    DbConnection(mut conn): DbConnection,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<common::dto::EmailAddress>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let maybe_user = models::User::by_email(&payload.email)
        .first::<models::User>(&mut conn)
        .await
        .optional()
        .or(Err(internal_error_response()))?;

    if let Some(usr) = maybe_user {
        let token = state
            .auth_service
            .gen_and_set_user_reset_password_token(usr.id)
            .await
            .or(Err(internal_error_response()))?;

        state
            .mailer_service
            .send_recover_password_email(payload.email, token, usr.username)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("password recovery email queued successfully"));
    }

    Err((
        StatusCode::NOT_FOUND,
        SimpleError::from("user not found with this email"),
    ))
}

/// Recover password by token
///
/// Sets a new password for the account in the recover password JWT.
#[utoipa::path(
    post,
    path = "/auth/change-password-by-recovery-token",
    tag = "auth",
    request_body = ResetPassword,
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("password recovery email queued to be sent successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "new password too weak",
            body = SimpleError,
        ),
    ),
)]
pub async fn change_password_by_recovery_token(
    DbConnection(mut conn): DbConnection,
    ValidatedJson(payload): ValidatedJson<dto::ResetPassword>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    jwt::decode(&payload.password_reset_token).or(Err((
        StatusCode::UNAUTHORIZED,
        SimpleError::from("invalid token"),
    )))?;

    let maybe_user = models::User::all()
        .filter(user::dsl::reset_password_token.eq(&payload.password_reset_token))
        .first::<models::User>(&mut conn)
        .await
        .optional()
        .or(Err(internal_error_response()))?;

    if let Some(usr) = maybe_user {
        let new_password_hash =
            hash(&payload.new_password, DEFAULT_COST).or(Err(internal_error_response()))?;

        diesel::update(user::dsl::user)
            .filter(user::dsl::id.eq(usr.id))
            .set((
                user::dsl::reset_password_token.eq::<Option<String>>(None),
                user::dsl::password.eq(new_password_hash),
            ))
            .execute(&mut conn)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("password changed successfully"));
    }

    Err((
        StatusCode::NOT_FOUND,
        SimpleError::from("user not found with this reset password token"),
    ))
}

/// Confirm email address by token
///
/// Confirms the email address of the user with this token
#[utoipa::path(
    post,
    path = "/auth/confirm-email-address-by-token",
    tag = "auth",
    request_body = Token,
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("password recovery email queued to be sent successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / EMAIL_ALREADY_CONFIRMED",
            body = SimpleError,
        ),
    ),
)]
pub async fn confirm_email_address_by_token(
    DbConnection(mut conn): DbConnection,
    ValidatedJson(payload): ValidatedJson<common::dto::Token>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    jwt::decode(&payload.token).or(Err((
        StatusCode::UNAUTHORIZED,
        SimpleError::from("invalid token"),
    )))?;

    let maybe_user = models::User::all()
        .filter(user::dsl::confirm_email_token.eq(&payload.token))
        .first::<models::User>(&mut conn)
        .await
        .optional()
        .or(Err(internal_error_response()))?;

    if let Some(usr) = maybe_user {
        if usr.email_verified {
            return Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(EMAIL_ALREADY_VERIFIED),
            ));
        }

        diesel::update(user::dsl::user)
            .filter(user::dsl::id.eq(usr.id))
            .set((
                user::dsl::email_verified.eq(true),
                user::dsl::confirm_email_token.eq::<Option<String>>(None),
            ))
            .execute(&mut conn)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("email confirmed successfully"));
    }

    Err((
        StatusCode::NOT_FOUND,
        SimpleError::from("user not found with this reset password token"),
    ))
}
