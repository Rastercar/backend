use super::dto::ListSimCardsDto;
use crate::{
    database::{self, error::DbError},
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{DbConnection, OrganizationId, ValidatedQuery},
            responses::SimpleError,
        },
    },
    server::controller::AppState,
};
use axum::{
    extract::Path,
    routing::{delete, get},
    Json, Router,
};
use entity::sim_card;
use http::StatusCode;
use migration::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QueryTrait};
use shared::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_sim_cards))
        .route("/:sim_card_id", delete(delete_sim_card))
        .layer(AclLayer::new(vec![Permission::DeleteSimCard]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Deletes a SIM card
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
                query.filter(sim_card::Column::TrackerId.is_not_null())
            } else {
                query.filter(sim_card::Column::TrackerId.is_null())
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
