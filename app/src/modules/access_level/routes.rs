use crate::database::error::DbError;
use crate::database::helpers::set_if_some;
use crate::modules::auth;
use crate::modules::auth::middleware::{AclLayer, RequestUser};
use crate::modules::common::dto::{Pagination, PaginationResult};
use crate::modules::common::extractors::{
    DbConnection, OrgBoundEntityFromPathId, OrganizationId, ValidatedJson, ValidatedQuery,
};
use crate::modules::common::responses::SimpleError;
use crate::server::controller::AppState;
use anyhow::Result;
use axum::extract::Path;
use axum::Extension;
use axum::{
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use entity::access_level;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set,
};
use sea_query::extension::postgres::PgExpr;
use sea_query::Expr;
use shared::Permission;

use super::dto::{
    self, AccessLevelDto, CreateAccessLevelDto, ListAccessLevelsDto, UpdateAccessLevelDto,
};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_access_level))
        .route("/", post(create_access_level))
        .layer(AclLayer::new(vec![Permission::ManageUserAccessLevels]))
        .route("/:access_level_id", get(access_level_by_id))
        .route("/:access_level_id", put(update_access_level))
        .layer(AclLayer::new(vec![Permission::ManageUserAccessLevels]))
        .route("/:access_level_id", delete(delete_access_level))
        .layer(AclLayer::new(vec![Permission::ManageUserAccessLevels]))
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
    OrgBoundEntityFromPathId(v): OrgBoundEntityFromPathId<entity::access_level::Entity>,
) -> Result<Json<AccessLevelDto>, (StatusCode, SimpleError)> {
    Ok(Json(AccessLevelDto::from(v)))
}

/// Create a access level
///
/// Required permissions: MANAGE_USER_ACCESS_LEVELS
#[utoipa::path(
    post,
    tag = "access-level",
    path = "/access-level",
    security(("session_id" = [])),
    request_body(content = CreateAccessLevelDto, content_type = "application/json"),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = access_level::dto::AccessLevelDto,
        ),
    ),
)]
pub async fn create_access_level(
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<CreateAccessLevelDto>,
) -> Result<Json<AccessLevelDto>, (StatusCode, SimpleError)> {
    let access_level_model = entity::access_level::ActiveModel {
        name: Set(dto.name),
        description: Set(dto.description),
        permissions: Set(dto.permissions),
        is_fixed: Set(false),
        organization_id: Set(Some(org_id)),
        ..Default::default()
    };

    let created_access_level: AccessLevelDto = access_level_model
        .insert(&db)
        .await
        .map_err(DbError::from)?
        .into();

    Ok(Json(created_access_level))
}

/// Update a access level
///
/// Required permissions: MANAGE_USER_ACCESS_LEVELS
#[utoipa::path(
    put,
    tag = "access-level",
    path = "/access-level/{access_level_id}",
    security(("session_id" = [])),
    params(
        ("access_level_id" = u128, Path, description = "id of the access level to update"),
    ),
    request_body(content = UpdateAccessLevelDto, content_type = "application/json"),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = access_level::dto::AccessLevelDto,
        ),
    ),
)]
pub async fn update_access_level(
    Path(access_level_id): Path<i64>,
    OrganizationId(org_id): OrganizationId,
    Extension(req_user): Extension<RequestUser>,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<UpdateAccessLevelDto>,
) -> Result<Json<AccessLevelDto>, (StatusCode, SimpleError)> {
    if req_user.0.access_level.id as i64 == access_level_id {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot update your own access level"),
        ));
    }

    let access_level_to_update = access_level::Entity::find()
        .filter(access_level::Column::OrganizationId.eq(org_id))
        .filter(access_level::Column::Id.eq(access_level_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?;

    if access_level_to_update.is_fixed {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot change fixed access levels"),
        ));
    }

    let mut access_level_to_update: access_level::ActiveModel = access_level_to_update.into();

    access_level_to_update.name = set_if_some(dto.name);
    access_level_to_update.description = set_if_some(dto.description);
    access_level_to_update.permissions = set_if_some(dto.permissions);

    let updated_access_level = access_level_to_update
        .update(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(AccessLevelDto::from(updated_access_level)))
}

/// Deletes a access level
///
/// Required permissions: MANAGE_USER_ACCESS_LEVELS
#[utoipa::path(
    delete,
    tag = "access-level",
    path = "/access-level/{access_level_id}",
    security(("session_id" = [])),
    params(
        ("access_level_id" = u128, Path, description = "id of the access level to delete"),
    ),
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("Access level deleted successfully"),
        ),
    ),
)]
pub async fn delete_access_level(
    Extension(req_user): Extension<RequestUser>,
    Path(access_level_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    if req_user.0.access_level.id == access_level_id {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot delete your own access level"),
        ));
    }

    let access_level_to_delete = access_level::Entity::find()
        .filter(access_level::Column::OrganizationId.eq(org_id))
        .filter(access_level::Column::Id.eq(access_level_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?;

    if access_level_to_delete.is_fixed {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot delete a fixed access level"),
        ));
    }

    let users_on_access_level_count: i64 = entity::user::Entity::find()
        .select_only()
        .column_as(entity::user::Column::Id.count(), "count")
        .filter(entity::user::Column::AccessLevelId.eq(access_level_id))
        .into_tuple()
        .one(&db)
        .await
        .map_err(DbError::from)?
        .unwrap_or(0);

    if users_on_access_level_count > 0 {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot delete access level with associated users"),
        ));
    }

    let delete_result = access_level::Entity::delete_many()
        .filter(access_level::Column::Id.eq(access_level_id))
        .filter(access_level::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if delete_result.rows_affected < 1 {
        let err_msg = "Access level not exist or does not belong to the request user organization";
        Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)))
    } else {
        Ok(Json(String::from("access level deleted successfully")))
    }
}
