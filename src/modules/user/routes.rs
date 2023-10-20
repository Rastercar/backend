use super::dto::ProfilePicDto;
use crate::{
    modules::{
        auth::{self, dto::UserDto, middleware::RequestUser},
        common::{
            multipart_form_data,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::{
    extract::State,
    routing::{delete, get, put},
    Extension, Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use tracing::error;

pub fn create_user_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/me/profile-picture", put(put_profile_picture))
        .route("/me/profile-picture", delete(delete_profile_picture))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::user_only_route,
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
pub async fn me(req_user: Extension<RequestUser>) -> Json<UserDto> {
    Json(UserDto::from(req_user.0 .0))
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
    req_user: Extension<RequestUser>,
    TypedMultipart(ProfilePicDto { image }): TypedMultipart<ProfilePicDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let img_extension =
        multipart_form_data::get_image_extension_from_field_or_fail_request(&image)?;

    let timestamp = chrono::Utc::now().format("%d-%m-%Y_%H:%M:%S");
    let filename = format!("profile-picture_{}.{}", timestamp, img_extension);

    let request_user = req_user.0 .0;

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
    req_user: Extension<RequestUser>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let conn = &mut state.get_db_conn().await?;

    let request_user = req_user.0 .0;

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
