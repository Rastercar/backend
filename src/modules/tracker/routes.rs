use crate::{
    database::models::VehicleTracker,
    modules::{
        auth::{self, constants::Permission, middleware::AclLayer},
        common::{
            extractors::{OrganizationId, ValidatedJson},
            responses::SimpleError,
        },
    },
    server::controller::AppState,
};
use axum::{routing::post, Json, Router};
use http::StatusCode;

use super::dto::CreateTrackerDto;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_tracker))
        .layer(AclLayer::new(vec![Permission::CreateVehicle]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Creates a new tracker
#[utoipa::path(
    post,
    path = "/tracker",
    tag = "tracker",
    security(("session_id" = [])),
    request_body = CreateTrackerDto,
    responses(
        (
            status = OK,
            description = "the created tracker",
            content_type = "application/json",
            body = VehicleTracker,
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / IMEI_IN_USE",
            body = SimpleError,
        ),
    ),
)]
pub async fn create_tracker(
    // State(state): State<AppState>,
    OrganizationId(org_id): OrganizationId,
    ValidatedJson(dto): ValidatedJson<CreateTrackerDto>,
) -> Result<Json<VehicleTracker>, (StatusCode, SimpleError)> {
    println!("{}", org_id);
    println!("{:#?}", dto);

    // TODO: implement a conn extractor from req parts to avoid this line everywhere
    // let conn = &mut state.get_db_conn().await?;

    todo!();

    // let mut created_tracker = repository::create_vehicle(conn, &dto, org_id).await?;

    // Ok(Json(created_tracker))
}
