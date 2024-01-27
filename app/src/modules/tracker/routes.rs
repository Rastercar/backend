use super::dto::CreateTrackerDto;
use crate::{
    database::error::DbError,
    modules::{
        auth::{self, constants::Permission, middleware::AclLayer},
        common::{
            dto::Pagination,
            extractors::{DbConnection, OrganizationId, ValidatedJson, ValidatedQuery},
            responses::{internal_error_res, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{
    routing::{get, post},
    Json, Router,
};
use entity::vehicle_tracker;
use http::StatusCode;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TryIntoModel,
};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_tracker))
        .layer(AclLayer::new(vec![Permission::CreateTracker]))
        .route("/", get(list_trackers))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Creates a new tracker
///
/// Required permissions: CREATE_TRACKER
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
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<CreateTrackerDto>,
) -> Result<Json<entity::vehicle_tracker::Model>, (StatusCode, SimpleError)> {
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

        let trackers_on_vehicle_cnt: i64 = entity::vehicle_tracker::Entity::find()
            .select_only()
            .column_as(entity::vehicle_tracker::Column::Id.count(), "count")
            .filter(entity::vehicle_tracker::Column::VehicleId.eq(vehicle_id))
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

    let created_tracker = entity::vehicle_tracker::ActiveModel {
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
    path = "/tracker",
    tag = "tracker",
    security(("session_id" = [])),
    params(
        Pagination
    ),
    responses(
        (
            status = OK,
            description = "paginated list of trackers",
            content_type = "application/json",
            body = PaginatedVehicleTracker,
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
    ),
)]
pub async fn list_trackers(
    ValidatedQuery(query): ValidatedQuery<Pagination>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<i32>, (StatusCode, SimpleError)> {
    println!("------------------------------------------");

    let db_q = vehicle_tracker::Entity::find()
        .order_by_asc(vehicle_tracker::Column::Id)
        .offset(query.page_size as u64)
        .paginate(&db, query.page_size as u64);

    let xd = db_q.num_items_and_pages().await.map_err(DbError::from)?;

    let xdd = db_q
        .fetch_page(query.page as u64)
        .await
        .map_err(DbError::from)?;

    // let xd = vehicle_tracker::Entity::find()
    //     .order_by_asc(vehicle_tracker::Column::Id)
    //     .offset(query.page_size as u64)
    //     .paginate(&db, query.page_size as u64)
    //     .num_items_and_pages()
    //     // .fetch()
    //     .await
    //     .map_err(DbError::from)?;

    dbg!(xd);
    dbg!(xdd);

    // let result = vehicle_tracker::dsl::vehicle_tracker
    //     .order(vehicle_tracker::id.asc())
    //     .filter(vehicle_tracker::dsl::organization_id.eq(org_id))
    //     .select(VehicleTracker::as_select())
    //     .paginate(query.page as i64)
    //     .per_page(query.page_size as i64)
    //     .load_with_pagination::<VehicleTracker>(&mut conn)
    //     .await
    //     .or(Err(internal_error_res()))?;

    // Ok(Json(result))

    todo!()
}
