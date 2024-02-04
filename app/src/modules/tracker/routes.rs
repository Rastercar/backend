use super::dto::{self, CreateTrackerDto, ListTrackersDto};
use crate::{
    database::{self, error::DbError},
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{DbConnection, OrganizationId, ValidatedJson, ValidatedQuery},
            responses::SimpleError,
        },
    },
    server::controller::AppState,
};
use axum::{
    extract::Path,
    routing::{delete, get, post, put},
    Json, Router,
};
use entity::vehicle_tracker;
use http::StatusCode;
use migration::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, TryIntoModel,
};
use shared::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_tracker))
        .layer(AclLayer::new(vec![Permission::CreateTracker]))
        .route("/", get(list_trackers))
        .route("/:tracker_id", delete(delete_tracker))
        .layer(AclLayer::new(vec![Permission::UpdateTracker]))
        .route("/:tracker_id/vehicle", put(set_tracker_vehicle))
        .layer(AclLayer::new(vec![Permission::UpdateTracker]))
        .route("/:tracker_id/sim-cards", get(list_tracker_sim_cards))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Deletes a SIM card
#[utoipa::path(
    delete,
    tag = "tracker",
    path = "/tracker/{tracker_id}",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker to delete"),
    ),
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("tracker deleted successfully"),
        ),
    ),
)]
pub async fn delete_tracker(
    Path(tracker_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let delete_result = vehicle_tracker::Entity::delete_many()
        .filter(vehicle_tracker::Column::Id.eq(tracker_id))
        .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if delete_result.rows_affected < 1 {
        let err_msg = "tracker does not exist or does not belong to the request user organization";
        Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)))
    } else {
        Ok(Json(String::from("tracker deleted successfully")))
    }
}

/// List SIM cards that belong to a tracker
#[utoipa::path(
    get,
    tag = "tracker",
    path = "/tracker/{tracker_id}/sim-cards",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker"),
    ),
    responses(
        (
            status = OK,
            description = "tracker sim cards",
            body = Vec<entity::sim_card::Model>,
            content_type = "application/json",
        ),
    ),
)]
pub async fn list_tracker_sim_cards(
    Path(tracker_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<Vec<entity::sim_card::Model>>, (StatusCode, SimpleError)> {
    let trackers = entity::sim_card::Entity::find()
        .filter(entity::sim_card::Column::TrackerId.eq(tracker_id))
        .filter(entity::sim_card::Column::OrganizationId.eq(org_id))
        .all(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(trackers))
}

/// Sets a tracker vehicle
///
/// Required permissions: UPDATE_TRACKER
#[utoipa::path(
    put,
    tag = "tracker",
    path = "/tracker/{tracker_id}/vehicle",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker to associate to the vehicle"),
    ),
    request_body(content = SetTrackerVehicleDto),
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("tracker vehicle set successfully"),
        ),
        (
            status = BAD_REQUEST,
            description = "tracker <id> is already has a vehicle",
            body = SimpleError,
        ),
    ),
)]
pub async fn set_tracker_vehicle(
    Path(tracker_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(payload): ValidatedJson<dto::SetTrackerVehicleDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let tracker = vehicle_tracker::Entity::find_by_id(tracker_id)
        .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((
            StatusCode::NOT_FOUND,
            SimpleError::from("tracker not found"),
        ))?;

    let vehicle = entity::vehicle::Entity::find_by_id(payload.vehicle_id)
        .filter(entity::vehicle::Column::OrganizationId.eq(org_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((
            StatusCode::NOT_FOUND,
            SimpleError::from("vehicle not found"),
        ))?;

    if tracker.vehicle_id.is_some() {
        let err_msg = format!("tracker {} is already has a vehicle", tracker.id);
        return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
    }

    let trackers_associated_with_vehicle: i64 = vehicle_tracker::Entity::find()
        .select_only()
        .column_as(vehicle_tracker::Column::Id.count(), "count")
        .filter(vehicle_tracker::Column::VehicleId.eq(payload.vehicle_id))
        .into_tuple()
        .one(&db)
        .await
        .map_err(DbError::from)?
        .unwrap_or(0);

    if trackers_associated_with_vehicle > 0 {
        let err_msg = format!("vehicle: {} already has a tracker", payload.vehicle_id);
        return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
    }

    vehicle_tracker::Entity::update_many()
        .col_expr(vehicle_tracker::Column::VehicleId, Expr::value(vehicle.id))
        .filter(vehicle_tracker::Column::Id.eq(tracker.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(String::from("tracker vehicle set successfully")))
}

/// Creates a new tracker
///
/// Required permissions: CREATE_TRACKER
#[utoipa::path(
    post,
    tag = "tracker",
    path = "/tracker",
    security(("session_id" = [])),
    request_body = CreateTrackerDto,
    responses(
        (
            status = OK,
            description = "the created tracker",
            content_type = "application/json",
            body = entity::vehicle_tracker::Model,
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
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<CreateTrackerDto>,
) -> Result<Json<vehicle_tracker::Model>, (StatusCode, SimpleError)> {
    if let Some(vehicle_id) = dto.vehicle_id {
        let count: i64 = entity::vehicle::Entity::find()
            .select_only()
            .column_as(entity::vehicle::Column::Id.count(), "count")
            .filter(entity::vehicle::Column::Id.eq(vehicle_id))
            .filter(entity::vehicle::Column::OrganizationId.eq(org_id))
            .into_tuple()
            .one(&db)
            .await
            .map_err(DbError::from)?
            .unwrap_or(0);

        if count < 1 {
            let err_msg = format!("vehicle: {} not found for org {}", vehicle_id, org_id);
            return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
        }

        let trackers_on_vehicle_cnt: i64 = vehicle_tracker::Entity::find()
            .select_only()
            .column_as(vehicle_tracker::Column::Id.count(), "count")
            .filter(vehicle_tracker::Column::VehicleId.eq(vehicle_id))
            .into_tuple()
            .one(&db)
            .await
            .map_err(DbError::from)?
            .unwrap_or(0);

        if trackers_on_vehicle_cnt > 0 {
            let err_msg = format!("vehicle: {} already has a tracker installed", vehicle_id);
            return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
        }
    }

    let created_tracker = vehicle_tracker::ActiveModel {
        imei: Set(dto.imei),
        model: Set(dto.model),
        vehicle_id: Set(dto.vehicle_id),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .save(&db)
    .await
    .map_err(DbError::from)?
    .try_into_model()
    .map_err(DbError::from)?;

    Ok(Json(created_tracker))
}

/// Lists the trackers that belong to the same org as the request user
#[utoipa::path(
    get,
    tag = "tracker",
    path = "/tracker",
    security(("session_id" = [])),
    params(
        Pagination,
        ListTrackersDto
    ),
    responses(
        (
            status = OK,
            description = "paginated list of trackers",
            content_type = "application/json",
            body = PaginatedVehicleTracker,
        ),
    ),
)]
pub async fn list_trackers(
    ValidatedQuery(pagination): ValidatedQuery<Pagination>,
    ValidatedQuery(filter): ValidatedQuery<ListTrackersDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<PaginationResult<vehicle_tracker::Model>>, (StatusCode, SimpleError)> {
    let db_query = vehicle_tracker::Entity::find()
        .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        .apply_if(filter.with_associated_vehicle, |query, with_vehicle| {
            if with_vehicle {
                query.filter(vehicle_tracker::Column::VehicleId.is_not_null())
            } else {
                query.filter(vehicle_tracker::Column::VehicleId.is_null())
            }
        })
        .apply_if(filter.imei, |query, imei| {
            if imei != "" {
                let col = Expr::col((vehicle_tracker::Entity, vehicle_tracker::Column::Imei));
                query.filter(col.ilike(format!("%{}%", imei)))
            } else {
                query
            }
        })
        .order_by_asc(vehicle_tracker::Column::Id)
        .paginate(&db, pagination.page_size);

    let result = database::helpers::paginated_query_to_pagination_result(db_query, pagination)
        .await
        .map_err(DbError::from)?;

    Ok(Json(result))
}
