use super::dto::CreateVehicleDto;
use crate::{
    database::models::Vehicle,
    modules::{
        auth::{self, constants::Permission, middleware::AclLayer},
        common::{
            extractors::{OrganizationId, ValidatedMultipart},
            multipart_form_data,
            responses::{internal_error_response_with_msg, SimpleError},
        },
        vehicle::repository,
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::extract::State;
use axum::{routing::post, Json, Router};
use http::StatusCode;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_vehicle))
        .layer(AclLayer::new(vec![Permission::CreateVehicle]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Creates a new vehicle
///
/// Required permissions: CREATE_VEHICLE
#[utoipa::path(
    post,
    path = "/vehicle",
    tag = "vehicle",
    security(("session_id" = [])),
    request_body(content = CreateVehicleDto, content_type = "multipart/form-data"),
    responses(
        (
            status = OK,
            description = "the created vehicle",
            content_type = "application/json",
            body = Vehicle,
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / PLATE_IN_USE",
            body = SimpleError,
        ),
    ),
)]
pub async fn create_vehicle(
    State(state): State<AppState>,
    OrganizationId(org_id): OrganizationId,
    ValidatedMultipart(dto): ValidatedMultipart<CreateVehicleDto>,
) -> Result<Json<Vehicle>, (StatusCode, SimpleError)> {
    let conn = &mut state.get_db_conn().await?;

    let mut created_vehicle = repository::create_vehicle(conn, &dto, org_id).await?;

    if let Some(photo) = dto.photo {
        let filename = match multipart_form_data::create_filename_with_timestamp_from_uploaded_photo(
            "photo", &photo,
        ) {
            Ok(f) => f,
            Err(e) => {
                // Creating the vehicle without the uploaded photo is not acceptable
                // therefore delete the created vehicle and return a error response.
                let _ = created_vehicle.delete_self(conn).await;
                return Err(e);
            }
        };

        let folder = format!("organization/{}/vehicle/{}", org_id, created_vehicle.id);

        let key = S3Key { folder, filename };

        if state
            .s3
            .upload(key.clone().into(), photo.contents)
            .await
            .is_err()
        {
            let _ = created_vehicle.delete_self(conn).await;

            return Err(internal_error_response_with_msg(
                "failed to upload vehicle photo",
            ));
        };

        let uploaded_photo = String::from(key.clone());

        let update_photo_on_db_result = created_vehicle
            .set_photo(conn, Some(uploaded_photo.clone()))
            .await;

        if let Err(_) = update_photo_on_db_result {
            let _ = state.s3.delete(uploaded_photo).await;
            let _ = created_vehicle.delete_self(conn).await;

            return Err(internal_error_response_with_msg(
                "failed to set vehicle photo",
            ));
        }
    }

    Ok(Json(created_vehicle))
}
