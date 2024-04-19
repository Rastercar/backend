use super::dto::{
    self, CreateTrackerDto, DeleteTrackerDto, GetTrackerPositionsDto, ListTrackersDto,
    UpdateTrackerDto,
};
use crate::{
    database::{self, error::DbError, helpers::set_if_some},
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{
                DbConnection, OrgBoundEntityFromPathId, OrganizationId, ValidatedJson,
                ValidatedQuery,
            },
            responses::{internal_error_res, SimpleError},
        },
        globals::TRACKER_ID_CACHE,
    },
    server::controller::AppState,
};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use migration::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, TryIntoModel,
};
use sea_query::{Cond, PostgresQueryBuilder, Query as SeaQuery};
use sea_query_binder::SqlxBinder;
use shared::entity::{
    sim_card, traits::QueryableByIdAndOrgId, vehicle_tracker, vehicle_tracker_last_location,
    vehicle_tracker_location,
};
use shared::{
    constants::{Permission, TrackerModel},
    entity::vehicle,
};
use std::str::FromStr;
use tracing::{info, Instrument, Span};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(create_tracker).layer(AclLayer::single(Permission::CreateTracker)),
        )
        //
        .route("/", get(list_trackers))
        //
        .route("/:tracker_id", get(get_tracker))
        //
        .route(
            "/:tracker_id",
            put(update_tracker).layer(AclLayer::single(Permission::UpdateTracker)),
        )
        //
        .route(
            "/:tracker_id",
            delete(delete_tracker).layer(AclLayer::single(Permission::DeleteTracker)),
        )
        //
        .route(
            "/:tracker_id/vehicle",
            put(set_tracker_vehicle).layer(AclLayer::single(Permission::UpdateTracker)),
        )
        //
        .route("/:tracker_id/get-location-list", post(get_location_list))
        .route("/:tracker_id/last-location", get(get_tracker_location))
        .route("/:tracker_id/sim-cards", get(list_tracker_sim_cards))
        //
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

#[tracing::instrument]
async fn delete_tracker_imei_from_cache(imei: String) {
    info!("removing tracker with imei {imei} from global cache");

    if let Some(tracker_id_cache) = TRACKER_ID_CACHE.get() {
        tracker_id_cache.write().await.delete(&imei)
    }
}

/// Get a tracker by ID
#[utoipa::path(
    get,
    tag = "tracker",
    path = "/tracker/{tracker_id}",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = entity::vehicle::Model,
        ),
    ),
)]
pub async fn get_tracker(
    OrgBoundEntityFromPathId(tracker): OrgBoundEntityFromPathId<vehicle_tracker::Entity>,
) -> Result<Json<vehicle_tracker::Model>, (StatusCode, SimpleError)> {
    Ok(Json(tracker))
}

/// Update a tracker
#[utoipa::path(
    put,
    tag = "tracker",
    path = "/tracker/{tracker_id}",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker to update"),
    ),
    request_body(content = UpdateTrackerDto, content_type = "application/json"),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = entity::vehicle_tracker::Model,
        ),
    ),
)]
#[tracing::instrument(skip_all)]
pub async fn update_tracker(
    Path(tracker_id): Path<i64>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<UpdateTrackerDto>,
) -> Result<Json<vehicle_tracker::Model>, (StatusCode, SimpleError)> {
    let tt = vehicle_tracker::Entity::find()
        .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        .filter(vehicle_tracker::Column::Id.eq(tracker_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?;

    let old_imei = tt.imei.clone();

    let mut t: vehicle_tracker::ActiveModel = tt.into();

    t.imei = set_if_some(dto.imei.clone());

    if let Some(model) = dto.model {
        t.model = Set(model)
    }

    let updated_tracker = t.update(&db).await.map_err(DbError::from)?;

    // If the imei has changed, we need to delete the old IMEI from the cache
    // otherwise the old imei cache will keep relating the old imei to the ID
    if dto.imei.is_some() {
        let span = Span::current();
        tokio::spawn(delete_tracker_imei_from_cache(old_imei).instrument(span));
    }

    Ok(Json(updated_tracker))
}

/// Deletes a tracker
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
#[tracing::instrument(skip_all)]
pub async fn delete_tracker(
    Query(dto): Query<DeleteTrackerDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    OrgBoundEntityFromPathId(tracker): OrgBoundEntityFromPathId<vehicle_tracker::Entity>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    if dto.delete_associated_sim_cards.unwrap_or(false) {
        sim_card::Entity::delete_many()
            .filter(sim_card::Column::VehicleTrackerId.eq(tracker.id))
            .filter(sim_card::Column::OrganizationId.eq(org_id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;
    }

    vehicle_tracker::Entity::delete_many()
        .filter(vehicle_tracker::Column::Id.eq(tracker.id))
        .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    // if there was a deleted tracker, we know it belongs to the user org so
    // we delete from the vehicle tracker location manually since this
    // table does not have a FK with ON DELETE CASCADE; to the vehicle_tracker
    // table for performance reasons
    vehicle_tracker_location::Entity::delete_many()
        .filter(vehicle_tracker_location::Column::VehicleTrackerId.eq(tracker.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    let span = Span::current();

    tokio::spawn(delete_tracker_imei_from_cache(tracker.imei).instrument(span));

    Ok(Json(String::from("tracker deleted successfully")))
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
) -> Result<Json<Vec<sim_card::Model>>, (StatusCode, SimpleError)> {
    let cards = sim_card::Entity::find()
        .filter(sim_card::Column::VehicleTrackerId.eq(tracker_id))
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .all(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(cards))
}

/// Get a list of tracker locations
#[utoipa::path(
    post,
    tag = "tracker",
    path = "/tracker/{tracker_id}/get-location-list",
    security(("session_id" = [])),
    request_body(content = GetTrackerPositionsDto),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker"),
    ),
    responses(
        (
            status = OK,
            description = "tracker location",
            body = Vec<TrackerLocationDto>,
            content_type = "application/json",
        ),
    ),
)]
pub async fn get_location_list(
    OrgBoundEntityFromPathId(tracker): OrgBoundEntityFromPathId<vehicle_tracker::Entity>,
    DbConnection(db): DbConnection,
    ValidatedJson(search_query): ValidatedJson<GetTrackerPositionsDto>,
) -> Result<Json<Vec<dto::TrackerLocationDto>>, (StatusCode, SimpleError)> {
    let (q, args) = SeaQuery::select()
        .column(vehicle_tracker_location::Column::Time)
        .column(vehicle_tracker_location::Column::Point)
        .from(vehicle_tracker_location::Entity)
        .cond_where(
            Cond::all()
                .add(Expr::col(vehicle_tracker_location::Column::VehicleTrackerId).eq(tracker.id))
                .add_option(
                    search_query
                        .after
                        .map(|a| Expr::col(vehicle_tracker_location::Column::Time).gt(a)),
                )
                .add_option(
                    search_query
                        .before
                        .map(|b| Expr::col(vehicle_tracker_location::Column::Time).lt(b)),
                ),
        )
        .order_by(
            vehicle_tracker_location::Column::Time,
            search_query.order.into(),
        )
        .limit(search_query.limit.unwrap_or(15))
        .to_owned()
        .build_sqlx(PostgresQueryBuilder);

    let rows: Vec<(
        DateTime<Utc>,
        geozero::wkb::Decode<geo_types::Geometry<f64>>,
    )> = sqlx::query_as_with(&q, args)
        .fetch_all(db.get_postgres_connection_pool())
        .await
        .map_err(|_| internal_error_res())?;

    let positions: Vec<dto::TrackerLocationDto> = rows
        .iter()
        .filter_map(|row| {
            if let Some(geo_types::Geometry::Point(point)) = row.1.geometry {
                let loc = dto::TrackerLocationDto {
                    point: point.into(),
                    time: row.0,
                };

                return Some(loc);
            }

            None
        })
        .collect();

    Ok(Json(positions))
}

/// Get the most recent tracker location
#[utoipa::path(
    get,
    tag = "tracker",
    path = "/tracker/{tracker_id}/last-location",
    security(("session_id" = [])),
    params(
        ("tracker_id" = u128, Path, description = "id of the tracker"),
    ),
    responses(
        (
            status = OK,
            description = "tracker location",
            body = Option<TrackerLocationDto>,
            content_type = "application/json",
        ),
    ),
)]
pub async fn get_tracker_location(
    Path(tracker_id): Path<i32>,
    DbConnection(db): DbConnection,
) -> Result<Json<Option<dto::TrackerLocationDto>>, (StatusCode, SimpleError)> {
    let (q, args) =
        SeaQuery::select()
            .column(vehicle_tracker_last_location::Column::Time)
            .column(vehicle_tracker_last_location::Column::Point)
            .from(vehicle_tracker_last_location::Entity)
            .cond_where(Cond::all().add(
                Expr::col(vehicle_tracker_last_location::Column::VehicleTrackerId).eq(tracker_id),
            ))
            .to_owned()
            .build_sqlx(PostgresQueryBuilder);

    let row: Option<(
        DateTime<Utc>,
        geozero::wkb::Decode<geo_types::Geometry<f64>>,
    )> = sqlx::query_as_with(&q, args)
        .fetch_optional(db.get_postgres_connection_pool())
        .await
        .map_err(|_| internal_error_res())?;

    if let Some(time_and_loc) = row {
        if let Some(geo_types::Geometry::Point(point)) = time_and_loc.1.geometry {
            let loc = dto::TrackerLocationDto {
                point: point.into(),
                time: time_and_loc.0,
            };

            return Ok(Json(Some(loc)));
        }
    }

    Ok(Json(None))
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
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    OrgBoundEntityFromPathId(tracker): OrgBoundEntityFromPathId<vehicle_tracker::Entity>,
    ValidatedJson(payload): ValidatedJson<dto::SetTrackerVehicleDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    // here we can unwrap vehicle_id because its guaranteed by the DTO validation to be `Some`
    let vehicle_id_or_none = payload.vehicle_id.ok_or(internal_error_res())?;

    if let Some(vehicle_id) = vehicle_id_or_none {
        vehicle::Entity::find_by_id_and_org_id(vehicle_id, org_id, &db)
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

        let trackers_associated_with_vehicle: i64 =
            vehicle::Entity::get_associated_tracker_count(vehicle_id, &db)
                .await
                .map_err(DbError::from)?;

        if trackers_associated_with_vehicle > 0 {
            let err_msg = format!("vehicle: {} already has a tracker", vehicle_id);
            return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
        }
    }

    vehicle_tracker::Entity::update_many()
        .col_expr(
            vehicle_tracker::Column::VehicleId,
            Expr::value(vehicle_id_or_none),
        )
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
        let count: i64 = vehicle::Entity::find()
            .select_only()
            .column_as(vehicle::Column::Id.count(), "count")
            .filter(vehicle::Column::Id.eq(vehicle_id))
            .filter(vehicle::Column::OrganizationId.eq(org_id))
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

    let tracker_model = TrackerModel::from_str(&dto.model).or(Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("invalid tracker model"),
    )))?;

    let created_tracker = vehicle_tracker::ActiveModel {
        imei: Set(dto.imei),
        model: Set(tracker_model),
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
            if !imei.is_empty() {
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
