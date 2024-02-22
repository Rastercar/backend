use crate::database::error::DbError;
use crate::modules::auth;
use crate::modules::common::dto::{Pagination, PaginationResult};
use crate::modules::common::extractors::{DbConnection, OrganizationId, ValidatedQuery};
use crate::modules::common::responses::SimpleError;
use crate::server::controller::AppState;
use anyhow::Result;
use axum::extract::Path;
use axum::{http::StatusCode, routing::get, Json, Router};
use entity::access_level;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QueryTrait};
use sea_query::extension::postgres::PgExpr;
use sea_query::Expr;

use super::dto::{self, AccessLevelDto, ListAccessLevelsDto};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_access_level))
        .route("/:access_level_id", get(access_level_by_id))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// List access levels
#[utoipa::path(
    get,
    tag = "access-level",
    path = "/access-level",
    security(("session_id" = [])),
    params(
        Pagination,
        ListAccessLevelsDto
    ),
    responses(
        (
            status = OK,
            description = "paginated list of access levels",
            content_type = "application/json",
            body = PaginatedAccessLevel,
        ),
    ),
)]
pub async fn list_access_level(
    ValidatedQuery(pagination): ValidatedQuery<Pagination>,
    ValidatedQuery(filter): ValidatedQuery<ListAccessLevelsDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<PaginationResult<AccessLevelDto>>, (StatusCode, SimpleError)> {
    let paginator = entity::access_level::Entity::find()
        .filter(entity::access_level::Column::OrganizationId.eq(org_id))
        .apply_if(filter.name, |query, name| {
            if name != "" {
                let col = Expr::col((
                    entity::access_level::Entity,
                    entity::access_level::Column::Name,
                ));
                query.filter(col.ilike(format!("%{}%", name)))
            } else {
                query
            }
        })
        .order_by_asc(entity::access_level::Column::Id)
        .paginate(&db, pagination.page_size);

    let n = paginator
        .num_items_and_pages()
        .await
        .map_err(DbError::from)?;

    let rows = paginator
        .fetch_page(pagination.page - 1)
        .await
        .map_err(DbError::from)?;

    let records: Vec<dto::AccessLevelDto> = rows.into_iter().map(AccessLevelDto::from).collect();

    let result = PaginationResult {
        page: pagination.page,
        records,
        page_size: pagination.page_size,
        item_count: n.number_of_items,
        page_count: n.number_of_pages,
    };

    Ok(Json(result))
}

/// Get a access level by id
#[utoipa::path(
    get,
    tag = "access-level",
    path = "/access-level/{access_level_id}",
    security(("session_id" = [])),
    params(
        ("access_level_id" = u128, Path, description = "id of the access level to get"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = access_level::dto::AccessLevelDto,
        ),
    ),
)]
pub async fn access_level_by_id(
    Path(access_level_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<AccessLevelDto>, (StatusCode, SimpleError)> {
    let v = access_level::Entity::find_by_id_and_org_id(access_level_id, org_id, &db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?;

    Ok(Json(AccessLevelDto::from(v)))
}
