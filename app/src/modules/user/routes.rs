use super::super::auth::dto as auth_dto;
use super::dto::{self, ProfilePicDto};
use crate::database::error::DbError;
use crate::modules::auth::middleware::RequestUserPassword;
use crate::modules::common::error_codes::EMAIL_ALREADY_VERIFIED;
use crate::modules::common::extractors::DbConnection;
use crate::modules::common::responses::internal_error_msg;
use crate::services::mailer::service::ConfirmEmailRecipientType;
use crate::{
    modules::{
        auth::{self, dto::UserDto, middleware::RequestUser},
        common::{
            extractors::ValidatedJson,
            multipart_form_data,
            responses::{internal_error_res, SimpleError},
        },
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::{
    extract::State,
    routing::{get, post, put},
    Extension, Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use bcrypt::{hash, verify, DEFAULT_COST};
use http::StatusCode;
use migration::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryTrait};
use tracing::error;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me).patch(update_me))
        .route("/me/password", put(put_password))
        .route(
            "/me/profile-picture",
            put(put_profile_picture).delete(delete_profile_picture),
        )
        .route(
            "/me/request-email-address-confirmation",
            post(request_email_address_confirmation),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Returns the request user
///
/// the request user is the user that owns the session on the session id (sid) cookie
#[utoipa::path(
    get,
    path = "/user/me",
    tag = "user",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = UserDto,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn me(Extension(req_user): Extension<RequestUser>) -> Json<UserDto> {
    Json(UserDto::from(req_user.0))
}

/// Updates the request user
#[utoipa::path(
    patch,
    path = "/user/me",
    tag = "user",
    security(("session_id" = [])),
    request_body = UpdateUserDto,
    responses(
        (
            status = OK,
            description = "the updated user",
            body = UserDto,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn update_me(
    DbConnection(db): DbConnection,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<dto::UpdateUserDto>,
) -> Result<Json<auth_dto::UserDto>, (StatusCode, SimpleError)> {
    let mut req_user = req_user.0;

    entity::user::Entity::update_many()
        .apply_if(payload.description.clone(), |query, v| {
            query.col_expr(entity::user::Column::Description, Expr::value(v))
        })
        .apply_if(payload.email.clone(), |query, v| {
            query.col_expr(entity::user::Column::Email, Expr::value(v))
        })
        .apply_if(payload.username.clone(), |query, v| {
            query.col_expr(entity::user::Column::Username, Expr::value(v))
        })
        .filter(entity::user::Column::Id.eq(req_user.id.clone()))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(new_description) = payload.description {
        req_user.description = new_description;
    }

    if let Some(new_username) = payload.username {
        req_user.username = new_username;
    }

    if let Some(new_email) = payload.email {
        req_user.email = new_email;
    }

    Ok(Json(req_user))
}

/// Changes the user password
#[utoipa::path(
    put,
    path = "/user/me/password",
    tag = "user",
    security(("session_id" = [])),
    request_body(content = ChangePasswordDto),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            example = json!("password changed successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "weak password",
            body = SimpleError,
        ),
    ),
)]
async fn put_password(
    DbConnection(db): DbConnection,
    Extension(req_user): Extension<RequestUser>,
    Extension(req_user_password): Extension<RequestUserPassword>,
    ValidatedJson(payload): ValidatedJson<dto::ChangePasswordDto>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let request_user = req_user.0;

    let old_password_valid =
        verify(payload.old_password, req_user_password.0.as_str()).or(Err(internal_error_res()))?;

    if !old_password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            SimpleError::from("invalid password"),
        ));
    }

    let new_password_hash = hash(payload.new_password, DEFAULT_COST)
        .or(Err(internal_error_msg("error hashing password")))?;

    entity::user::Entity::update_many()
        .col_expr(
            entity::user::Column::Password,
            Expr::value(new_password_hash),
        )
        .filter(entity::user::Column::Id.eq(request_user.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json("password changed successfully"))
}

/// Replaces the request user profile picture
#[utoipa::path(
    put,
    path = "/user/me/profile-picture",
    tag = "user",
    security(("session_id" = [])),
    request_body(content = ProfilePicDto, content_type = "multipart/form-data"),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            description = "S3 object key of the new profile picture",
            example = json!("rastercar/organization/1/user/2/profile-picture_20-10-2023_00:19:17.jpeg"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid file",
            body = SimpleError,
        ),
    ),
)]
async fn put_profile_picture(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    DbConnection(db): DbConnection,
    TypedMultipart(ProfilePicDto { image }): TypedMultipart<ProfilePicDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let filename = multipart_form_data::filename_from_img("profile-picture", &image)?;

    let request_user = req_user.0;

    let folder = match request_user.organization {
        Some(org) => format!("organization/{}/user/{}", org.id, request_user.id),
        None => format!("user/{}", request_user.id),
    };

    let key = S3Key { folder, filename };

    state
        .s3
        .upload(key.clone().into(), image.contents)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to upload new profile picture"),
            )
        })?;

    entity::user::Entity::update_many()
        .col_expr(
            entity::user::Column::ProfilePicture,
            Expr::value(String::from(key.clone())),
        )
        .filter(entity::user::Column::Id.eq(request_user.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(old_profile_pic) = request_user.profile_picture {
        if state.s3.delete(old_profile_pic.clone()).await.is_err() {
            error!("[] failed to delete S3 object: {}", old_profile_pic);
        }
    }

    Ok(Json(String::from(key)))
}

/// Removes the request user profile picture
#[utoipa::path(
    delete,
    path = "/user/me/profile-picture",
    tag = "user",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            example = json!("profile picture removed successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
async fn delete_profile_picture(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    DbConnection(db): DbConnection,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let request_user = req_user.0;

    if let Some(old_profile_pic) = request_user.profile_picture {
        entity::user::Entity::update_many()
            .col_expr(
                entity::user::Column::ProfilePicture,
                Expr::value::<Option<String>>(None),
            )
            .filter(entity::user::Column::Id.eq(request_user.id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;

        let _ = state.s3.delete(old_profile_pic).await;

        return Ok(Json("profile picture removed successfully"));
    }

    Ok(Json("user does not have a profile picture to remove"))
}

/// Requests a email address confirmation email
///
/// sends a email address confirmation email to be sent to the request user email address
#[utoipa::path(
    post,
    path = "/user/me/request-email-address-confirmation",
    tag = "user",
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("a confirmation email was sent"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / EMAIL_ALREADY_CONFIRMED",
            body = SimpleError,
        ),
    ),
)]
pub async fn request_email_address_confirmation(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    if req_user.0.email_verified {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(EMAIL_ALREADY_VERIFIED),
        ));
    }

    let token = state
        .auth_service
        .gen_and_set_user_confirm_email_token(req_user.0.id)
        .await
        .or(Err(internal_error_res()))?;

    state
        .mailer_service
        .send_confirm_email_address_email(req_user.0.email, token, ConfirmEmailRecipientType::User)
        .await
        .or(Err(internal_error_res()))?;

    Ok(Json("email address confirmation email queued successfully"))
}
