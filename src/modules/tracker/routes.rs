use super::dto::CreateTrackerDto;
use crate::{
    database::{
        error::DbError,
        models::VehicleTracker,
        schema::{vehicle, vehicle_tracker},
    },
    modules::{
        auth::{self, constants::Permission, middleware::AclLayer},
        common::{
            extractors::{DbConnection, OrganizationId, ValidatedJson},
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{routing::post, Json, Router};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

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
    OrganizationId(org_id): OrganizationId,
    DbConnection(mut conn): DbConnection,
    ValidatedJson(dto): ValidatedJson<CreateTrackerDto>,
) -> Result<Json<VehicleTracker>, (StatusCode, SimpleError)> {
    // TODO: invert the relationship between vehicle and tracker (make tracker_id in vehicle instead of the oposite, who the
    // fuck would install 2 trackers on the same vehicle anyway ?)
    if let Some(vehicle_id) = dto.vehicle_id {
        let count: i64 = vehicle::dsl::vehicle
            .filter(vehicle::dsl::id.eq(vehicle_id))
            .filter(vehicle::dsl::organization_id.eq(org_id))
            .count()
            .get_result(&mut conn)
            .await
            .or(Err(internal_error_response()))?;

        if count != 1 {
            return Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(format!(
                    "vehicle: {} not found for org {}",
                    vehicle_id, org_id
                )),
            ));
        }
    }

    let created_tracker = diesel::insert_into(vehicle_tracker::dsl::vehicle_tracker)
        .values((
            vehicle_tracker::dsl::imei.eq(dto.imei),
            vehicle_tracker::dsl::model.eq(dto.model),
            vehicle_tracker::dsl::vehicle_id.eq(dto.vehicle_id),
            vehicle_tracker::dsl::organization_id.eq(org_id),
        ))
        .get_result::<VehicleTracker>(&mut conn)
        .await
        .map_err(|e| DbError::from(e))?;

    Ok(Json(created_tracker))
}
