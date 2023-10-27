use super::super::auth::dto as auth_dto;
use super::dto::{self, ProfilePicDto};
use crate::modules::auth::middleware::RequestUserPassword;
use crate::modules::common::responses::internal_error_response_with_msg;
use crate::{
    modules::{
        auth::{self, dto::UserDto, middleware::RequestUser},
        common::{
            extractors::ValidatedJson,
            multipart_form_data,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::{
    extract::State,
    routing::{delete, get, patch, put},
    Extension, Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use tracing::error;

pub fn create_user_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/me", patch(update_me))
        .route("/me/password", put(put_password))
        .route("/me/profile-picture", put(put_profile_picture))
        .route("/me/profile-picture", delete(delete_profile_picture))
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
            description = "session not found",
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
            description = "session not found",
            body = SimpleError,
        ),
    ),
)]
pub async fn update_me(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<dto::UpdateUserDto>,
) -> Result<Json<auth_dto::UserDto>, (StatusCode, SimpleError)> {
    use crate::database::schema::user::dsl::*;

    let conn = &mut state.get_db_conn().await?;

    let mut req_user = req_user.0;

    diesel::update(user)
        .filter(id.eq(&req_user.id))
        .set(&payload)
        .execute(conn)
        .await
        .or(Err(internal_error_response()))?;

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
            description = "session not found",
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
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    Extension(req_user_password): Extension<RequestUserPassword>,
    ValidatedJson(payload): ValidatedJson<dto::ChangePasswordDto>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    use crate::database::schema::user::dsl::*;

    let conn = &mut state.get_db_conn().await?;

    let req_user = req_user.0;

    let old_password_valid = verify(payload.old_password, req_user_password.0.as_str())
        .or(Err(internal_error_response()))?;

    if !old_password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            SimpleError::from("invalid password"),
        ));
    }

    let new_password_hash = hash(payload.new_password, DEFAULT_COST).or(Err(
        internal_error_response_with_msg("error hashing password"),
    ))?;

    diesel::update(user)
        .filter(id.eq(&req_user.id))
        .set(password.eq(new_password_hash))
        .execute(conn)
        .await
        .or(Err(internal_error_response()))?;

    Ok(Json("password changed successfully"))
}

/// Replaces the request user profile picture
#[utoipa::path(
    put,
    path = "/user/me/profile-picture",
    tag = "user",
    security(("session_id" = [])),
    request_body(content = ProfilePicDto, description = "New post data", content_type = "multipart/form-data"),
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
            description = "session not found",
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
    TypedMultipart(ProfilePicDto { image }): TypedMultipart<ProfilePicDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let img_extension =
        multipart_form_data::get_image_extension_from_field_or_fail_request(&image)?;

    let timestamp = chrono::Utc::now().format("%d-%m-%Y_%H:%M:%S");
    let filename = format!("profile-picture_{}.{}", timestamp, img_extension);

    let request_user = req_user.0;

    let folder = match request_user.organization {
        Some(org) => format!("organization/{}/user/{}", org.id, request_user.id),
        None => format!("user/{}", request_user.id),
    };

    let key = S3Key { folder, filename };

    state
        .s3
        .upload(key.clone(), image.contents)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to upload new profile picture"),
            )
        })?;

    let conn = &mut state.get_db_conn().await?;

    {
        use crate::database::schema::user::dsl::*;

        diesel::update(user)
            .filter(id.eq(request_user.id))
            .set(profile_picture.eq(String::from(key.clone())))
            .execute(conn)
            .await
            .or(Err(internal_error_response()))?;
    }

    if let Some(old_profile_pic) = request_user.profile_picture {
        if state.s3.delete(&old_profile_pic).await.is_err() {
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
            description = "session not found",
            body = SimpleError,
        ),
    ),
)]
async fn delete_profile_picture(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let conn = &mut state.get_db_conn().await?;

    let request_user = req_user.0;

    if let Some(old_profile_pic) = request_user.profile_picture {
        use crate::database::schema::user::dsl::*;

        diesel::update(user)
            .filter(id.eq(request_user.id))
            .set(profile_picture.eq::<Option<String>>(None))
            .execute(conn)
            .await
            .or(Err(internal_error_response()))?;

        if state.s3.delete(&old_profile_pic).await.is_err() {
            error!("failed to delete S3 object: {}", old_profile_pic);
        }

        return Ok(Json("profile picture removed successfully"));
    }

    Ok(Json("user does not have a profile picture to remove"))
}
