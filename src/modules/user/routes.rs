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
    routing::{get, put},
    Extension, Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

pub fn create_user_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/met", get(me))
        .route("/me/profile-picture", put(update_profile_picture))
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

// TODO: open api
async fn update_profile_picture(
    State(state): State<AppState>,
    req_user: Extension<RequestUser>,
    TypedMultipart(ProfilePicDto { image }): TypedMultipart<ProfilePicDto>,
) -> Result<String, (StatusCode, SimpleError)> {
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
            .set(profile_picture.eq(String::from(key)))
            .execute(conn)
            .await
            .or(Err(internal_error_response()))?;
    }

    if let Some(old_user_profile_picture_key) = request_user.profile_picture {
        if let Err(deletion_error) = state.s3.delete(old_user_profile_picture_key).await {
            // TODO: !
            dbg!(deletion_error);
        }
    }

    Ok(String::from("profile picture updated successfully"))
}
