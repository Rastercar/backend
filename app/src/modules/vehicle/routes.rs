use super::dto::CreateVehicleDto;
use crate::{
    modules::{
        auth::{self, constants::Permission, middleware::AclLayer},
        common::{
            extractors::{DbConnection, OrganizationId, ValidatedMultipart},
            multipart_form_data,
            responses::{internal_error_msg, SimpleError},
        },
        vehicle::repository,
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::extract::State;
use axum::{routing::post, Json, Router};
use http::StatusCode;
use migration::Expr;
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_vehicle))
        .route("/xd", post(xd))
        .layer(AclLayer::new(vec![Permission::CreateVehicle]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

#[axum::debug_handler]
pub async fn xd(DbConnection(db): DbConnection) -> String {
    String::from("dassda")
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
) -> Result<Json<entity::vehicle::Model>, (StatusCode, SimpleError)> {
    let created_vehicle = repository::create_vehicle(&state.db, &dto, org_id).await?;

    if let Some(photo) = dto.photo {
        let img_validation = multipart_form_data::filename_from_img("photo", &photo);

        let filename = match img_validation {
            Ok(filename) => filename,
            Err(e) => {
                // Creating the vehicle without the uploaded photo is not acceptable
                // therefore delete the created vehicle and return a error response.
                let _ = created_vehicle.delete(&state.db).await;

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
            let _ = created_vehicle.delete(&state.db).await;

            return Err(internal_error_msg("failed to upload vehicle photo"));
        };

        let uploaded_photo = String::from(key.clone());

        let update_photo_on_db_result = entity::vehicle::Entity::update_many()
            .col_expr(
                entity::vehicle::Column::Photo,
                Expr::value(uploaded_photo.clone()),
            )
            .filter(entity::vehicle::Column::Id.eq(created_vehicle.id))
            .exec(&state.db)
            .await;

        if let Err(_) = update_photo_on_db_result {
            let _ = state.s3.delete(uploaded_photo).await;
            let _ = created_vehicle.delete(&state.db).await;

            return Err(internal_error_msg("failed to set vehicle photo"));
        }
    }

    Ok(Json(created_vehicle))
}
