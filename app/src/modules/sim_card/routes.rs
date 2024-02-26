use super::dto::{self, CreateSimCardDto, ListSimCardsDto};
use crate::{
    database::{self, error::DbError, helpers::set_if_some},
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{DbConnection, OrganizationId, ValidatedJson, ValidatedQuery},
            responses::{internal_error_res, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{
    extract::Path,
    routing::{delete, get, post, put},
    Json, Router,
};
use entity::{sim_card, vehicle_tracker};
use http::StatusCode;
use migration::Expr;
use sea_orm::{
    sea_query::extension::postgres::PgExpr, ActiveModelTrait, QuerySelect, Set, TryIntoModel,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QueryTrait};
use shared::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_sim_cards))
        //
        .route("/", post(create_sim_card))
        .layer(AclLayer::new(vec![Permission::CreateSimCard]))
        //
        .route("/:sim_card_id", get(get_sim_card))
        //
        .route("/:sim_card_id", put(update_sim_card))
        .layer(AclLayer::new(vec![Permission::UpdateSimCard]))
        //
        .route("/:sim_card_id", delete(delete_sim_card))
        .layer(AclLayer::new(vec![Permission::DeleteSimCard]))
        //
        .route("/:sim_card_id/tracker", put(set_sim_card_tracker))
        .layer(AclLayer::new(vec![Permission::UpdateTracker]))
        //
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Creates a SIM card
///
/// Required permissions: CREATE_SIM_CARD
#[utoipa::path(
    post,
    tag = "sim-card",
    path = "/sim-card",
    security(("session_id" = [])),
    request_body = CreateSimCardDto,
    responses(
        (
            status = OK,
            description = "the created SIM card",
            content_type = "application/json",
            body = entity::sim_card::Model,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / SSN_IN_USE / PHONE_NUMBER_IN_USE",
            body = SimpleError,
        ),
    ),
)]
pub async fn create_sim_card(
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<CreateSimCardDto>,
) -> Result<Json<sim_card::Model>, (StatusCode, SimpleError)> {
    if let Some(vehicle_tracker_id) = dto.vehicle_tracker_id {
        let tracker = vehicle_tracker::Entity::find()
            .filter(vehicle_tracker::Column::Id.eq(vehicle_tracker_id))
            .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
            .one(&db)
            .await
            .map_err(DbError::from)?
            .ok_or((
                StatusCode::BAD_REQUEST,
                SimpleError::from(format!(
                    "vehicle_tracker: {} not found for org {}",
                    vehicle_tracker_id, org_id
                )),
            ))?;

        let sim_cards_on_tracker_count: i64 = sim_card::Entity::find()
            .select_only()
            .column_as(sim_card::Column::Id.count(), "count")
            .filter(sim_card::Column::VehicleTrackerId.eq(vehicle_tracker_id))
            .into_tuple()
            .one(&db)
            .await
            .map_err(DbError::from)?
            .unwrap_or(0);

        if sim_cards_on_tracker_count + 1 > tracker.model.get_info().sim_card_slots.into() {
            let err_msg = format!(
                "vehicle_tracker: {} does not have a empty SIM card slot",
                vehicle_tracker_id
            );
            return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
        }
    }

    let created_sim_card = sim_card::ActiveModel {
        ssn: Set(dto.ssn),
        phone_number: Set(dto.phone_number),

        apn_user: Set(dto.apn_user),
        apn_password: Set(dto.apn_password),
        apn_address: Set(dto.apn_address),

        pin: Set(dto.pin),
        pin2: Set(dto.pin2),

        puk: Set(dto.puk),
        puk2: Set(dto.puk2),

        vehicle_tracker_id: Set(dto.vehicle_tracker_id),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .save(&db)
    .await
    .map_err(DbError::from)?
    .try_into_model()
    .map_err(DbError::from)?;

    Ok(Json(created_sim_card))
}

/// Updates a SIM card
///
/// Required permissions: UPDATE_SIM_CARD
#[utoipa::path(
    put,
    tag = "sim-card",
    path = "/sim-card/{sim_card_id}",
    security(("session_id" = [])),
    params(
        ("sim_card_id" = u128, Path, description = "id of the sim card to update"),
    ),
    request_body(content = UpdateSimCardDto),
    responses(
        (
            status = OK,
            description = "the updated SIM card",
            content_type = "application/json",
            body = entity::sim_card::Model,
        )
    ),
)]
pub async fn update_sim_card(
    Path(sim_card_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<dto::UpdateSimCardDto>,
) -> Result<Json<sim_card::Model>, (StatusCode, SimpleError)> {
    let mut v: sim_card::ActiveModel = sim_card::Entity::find()
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .filter(sim_card::Column::Id.eq(sim_card_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?
        .into();

    v.ssn = set_if_some(dto.ssn);
    v.phone_number = set_if_some(dto.phone_number);
    v.apn_user = set_if_some(dto.apn_user);
    v.apn_address = set_if_some(dto.apn_address);
    v.apn_password = set_if_some(dto.apn_password);
    v.pin = set_if_some(dto.pin);
    v.pin2 = set_if_some(dto.pin2);
    v.puk = set_if_some(dto.puk);
    v.puk2 = set_if_some(dto.puk2);

    let updated_sim_card = v.update(&db).await.map_err(DbError::from)?;

    Ok(Json(updated_sim_card))
}

/// Sets a sim card tracker
///
/// Required permissions: UPDATE_SIM_CARD
#[utoipa::path(
    put,
    tag = "sim-card",
    path = "/sim-card/{sim_card_id}/tracker",
    security(("session_id" = [])),
    params(
        ("sim_card_id" = u128, Path, description = "id of the sim card to associate to the tracker"),
    ),
    request_body(content = SetSimCardTrackerDto),
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("sim card tracker set successfully"),
        ),
        (
            status = BAD_REQUEST,
            description = "sim card <id> is already has a tracker",
            body = SimpleError,
        ),
    ),
)]
pub async fn set_sim_card_tracker(
    Path(sim_card_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(payload): ValidatedJson<dto::SetSimCardTrackerDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    // here we can unwrap vehicle_tracker_id because its guaranteed
    // by the DTO validation to be `Some`
    let tracker_id_or_none = payload.vehicle_tracker_id.ok_or(internal_error_res())?;

    let sim_card = sim_card::Entity::find_by_id(sim_card_id)
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((
            StatusCode::NOT_FOUND,
            SimpleError::from("sim card not found"),
        ))?;

    if let Some(new_tracker_id) = tracker_id_or_none {
        let tracker = vehicle_tracker::Entity::find_by_id(new_tracker_id)
            .filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
            .one(&db)
            .await
            .map_err(DbError::from)?
            .ok_or((
                StatusCode::NOT_FOUND,
                SimpleError::from("tracker not found"),
            ))?;

        if sim_card.vehicle_tracker_id == Some(new_tracker_id) {
            let success_msg = format!(
                "sim card is already associated with tracker: {}",
                new_tracker_id
            );
            return Ok(Json(String::from(success_msg)));
        }

        let sim_cards_associated_with_tracker: i64 = sim_card::Entity::find()
            .select_only()
            .column_as(sim_card::Column::Id.count(), "count")
            .filter(sim_card::Column::VehicleTrackerId.eq(new_tracker_id))
            .into_tuple()
            .one(&db)
            .await
            .map_err(DbError::from)?
            .unwrap_or(0);

        if sim_cards_associated_with_tracker + 1 > tracker.model.get_info().sim_card_slots.into() {
            let err_msg = "associating the sim card with the tracker would overflow the SIM slots for the tracker model";
            return Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)));
        }
    }

    sim_card::Entity::update_many()
        .col_expr(
            sim_card::Column::VehicleTrackerId,
            Expr::value(tracker_id_or_none),
        )
        .filter(sim_card::Column::Id.eq(sim_card_id))
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(String::from("sim card tracker set successfully")))
}

/// Deletes a SIM card
///
/// Required permissions: DELETE_SIM_CARD
#[utoipa::path(
    delete,
    tag = "sim-card",
    path = "/sim-card/{sim_card_id}",
    security(("session_id" = [])),
    params(
        ("sim_card_id" = u128, Path, description = "id of the SIM card to delete"),
    ),
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("SIM card deleted successfully"),
        ),
    ),
)]
pub async fn delete_sim_card(
    Path(sim_card_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let delete_result = sim_card::Entity::delete_many()
        .filter(sim_card::Column::Id.eq(sim_card_id))
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if delete_result.rows_affected < 1 {
        let err_msg = "SIM card does not exist or does not belong to the request user organization";
        Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)))
    } else {
        Ok(Json(String::from("sim card deleted successfully")))
    }
}

/// Get a SIM card by ID
#[utoipa::path(
    get,
    tag = "sim-card",
    path = "/sim-card/{sim_card_id}",
    security(("session_id" = [])),
    params(
        ("sim_card_id" = u128, Path, description = "id of the SIM card"),
    ),
    responses(
        (
            status = OK,
            description = "the SIM card",
            content_type = "application/json",
            body = entity::sim_card::Model,
        )
    ),
)]
pub async fn get_sim_card(
    Path(sim_card_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<entity::sim_card::Model>, (StatusCode, SimpleError)> {
    let sim_card = sim_card::Entity::find()
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .filter(sim_card::Column::Id.eq(sim_card_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::from("SIM not found")))?;

    Ok(Json(sim_card))
}

/// Lists the SIM cards that belong to the same org as the request user
#[utoipa::path(
    get,
    tag = "sim-card",
    path = "/sim-card",
    security(("session_id" = [])),
    params(
        Pagination,
        ListSimCardsDto
    ),
    responses(
        (
            status = OK,
            description = "paginated list of SIM cards",
            content_type = "application/json",
            body = PaginatedSimCard,
        ),
    ),
)]
pub async fn list_sim_cards(
    ValidatedQuery(pagination): ValidatedQuery<Pagination>,
    ValidatedQuery(filter): ValidatedQuery<ListSimCardsDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<PaginationResult<entity::sim_card::Model>>, (StatusCode, SimpleError)> {
    let db_query = sim_card::Entity::find()
        .filter(sim_card::Column::OrganizationId.eq(org_id))
        .apply_if(filter.with_associated_tracker, |query, with_vehicle| {
            if with_vehicle {
                query.filter(sim_card::Column::VehicleTrackerId.is_not_null())
            } else {
                query.filter(sim_card::Column::VehicleTrackerId.is_null())
            }
        })
        .apply_if(filter.phone_number, |query, phone| {
            if phone != "" {
                let col = Expr::col((sim_card::Entity, sim_card::Column::PhoneNumber));
                query.filter(col.ilike(format!("%{}%", phone)))
            } else {
                query
            }
        })
        .order_by_asc(sim_card::Column::Id)
        .paginate(&db, pagination.page_size);

    let result = database::helpers::paginated_query_to_pagination_result(db_query, pagination)
        .await
        .map_err(DbError::from)?;

    Ok(Json(result))
}
