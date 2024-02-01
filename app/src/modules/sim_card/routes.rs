use super::dto::ListSimCardsDto;
use crate::{
    database::error::DbError,
    modules::{
        auth::{self},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{DbConnection, OrganizationId, ValidatedQuery},
            responses::SimpleError,
        },
    },
    server::controller::AppState,
};
use axum::{routing::get, Json, Router};
use entity::sim_card;
use http::StatusCode;
use migration::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QueryTrait};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_sim_cards))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

// TODO: add me to open_api
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
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
    ),
)]
pub async fn list_sim_cards(
    ValidatedQuery(query): ValidatedQuery<Pagination>,
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
        });

    // TODO: this should be abstracted ?
    let paginated_query = db_query
        .order_by_asc(sim_card::Column::Id)
        .paginate(&db, query.page_size);

    let n = paginated_query
        .num_items_and_pages()
        .await
        .map_err(DbError::from)?;

    let records = paginated_query
        .fetch_page(query.page - 1)
        .await
        .map_err(DbError::from)?;

    let result = PaginationResult {
        page: query.page,
        records,
        page_size: query.page_size,
        item_count: n.number_of_items,
        page_count: n.number_of_pages,
    };

    Ok(Json(result))
}
