use super::dto::{CreateVehicleDto, ListVehiclesDto, UpdateVehicleDto};
use crate::{
    database::{
        error::DbError,
        helpers::{paginated_query_to_pagination_result, set_if_some},
    },
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult},
            extractors::{
                DbConnection, OrganizationId, ValidatedJson, ValidatedMultipart, ValidatedQuery,
            },
            multipart_form_data,
            responses::{internal_error_msg, SimpleError},
        },
        vehicle::repository,
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::extract::{Path, State};
use axum::{
    routing::{get, post, put},
    Json, Router,
};
use entity::vehicle;
use http::StatusCode;
use migration::{extension::postgres::PgExpr, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QueryTrait,
};
use shared::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_vehicles))
        .route("/", post(create_vehicle))
        .layer(AclLayer::new(vec![Permission::CreateVehicle]))
        .route("/:vehicle_id", get(vehicle_by_id))
        .route("/:vehicle_id", put(update_vehicle))
        .layer(AclLayer::new(vec![Permission::UpdateVehicle]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Get a vehicle by id
#[utoipa::path(
    get,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to get"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = entity::vehicle::Model,
        ),
    ),
)]
pub async fn vehicle_by_id(
    Path(vehicle_id): Path<i64>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<entity::vehicle::Model>, (StatusCode, SimpleError)> {
    let v = vehicle::Entity::find()
        .filter(vehicle::Column::OrganizationId.eq(org_id))
        .filter(vehicle::Column::Id.eq(vehicle_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?;

    Ok(Json(v))
}

/// Update a vehicle
#[utoipa::path(
    put,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to update"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = entity::vehicle::Model,
        ),
    ),
)]
pub async fn update_vehicle(
    Path(vehicle_id): Path<i64>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
    ValidatedJson(dto): ValidatedJson<UpdateVehicleDto>,
) -> Result<Json<entity::vehicle::Model>, (StatusCode, SimpleError)> {
    let mut v: vehicle::ActiveModel = vehicle::Entity::find()
        .filter(vehicle::Column::OrganizationId.eq(org_id))
        .filter(vehicle::Column::Id.eq(vehicle_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::entity_not_found()))?
        .into();

    v.plate = set_if_some(dto.plate);
    v.brand = set_if_some(dto.brand);
    v.model = set_if_some(dto.model);
    v.color = set_if_some(dto.color);
    v.model_year = set_if_some(dto.model_year);
    v.chassis_number = set_if_some(dto.chassis_number);
    v.additional_info = set_if_some(dto.additional_info);
    v.fabrication_year = set_if_some(dto.fabrication_year);

    let updated_vehicle = v.update(&db).await.map_err(DbError::from)?;

    Ok(Json(updated_vehicle))
}

/// Lists the vehicles that belong to the same org as the request user
#[utoipa::path(
    get,
    tag = "vehicle",
    path = "/vehicle",
    security(("session_id" = [])),
    params(
        Pagination
    ),
    responses(
        (
            status = OK,
            description = "paginated list of vehicles",
            content_type = "application/json",
            body = PaginatedVehicle,
        ),
    ),
)]
pub async fn list_vehicles(
    ValidatedQuery(pagination): ValidatedQuery<Pagination>,
    ValidatedQuery(filter): ValidatedQuery<ListVehiclesDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<PaginationResult<entity::vehicle::Model>>, (StatusCode, SimpleError)> {
    let db_query = vehicle::Entity::find()
        .filter(vehicle::Column::OrganizationId.eq(org_id))
        .apply_if(filter.plate, |query, plate| {
            if plate != "" {
                let col = Expr::col((vehicle::Entity, vehicle::Column::Plate));
                query.filter(col.ilike(format!("%{}%", plate)))
            } else {
                query
            }
        })
        .order_by_asc(vehicle::Column::Id)
        .paginate(&db, pagination.page_size);

    let result = paginated_query_to_pagination_result(db_query, pagination)
        .await
        .map_err(DbError::from)?;

    Ok(Json(result))
}

/// Creates a new vehicle
///
/// Required permissions: CREATE_VEHICLE
#[utoipa::path(
    post,
    tag = "vehicle",
    path = "/vehicle",
    security(("session_id" = [])),
    request_body(content = CreateVehicleDto, content_type = "multipart/form-data"),
    responses(
        (
            status = OK,
            description = "the created vehicle",
            content_type = "application/json",
            body = entity::vehicle::Model,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / PLATE_IN_USE",
            body = SimpleError,
        ),
    ),
)]
pub async fn create_vehicle(
    State(state): State<AppState>,
    OrganizationId(org_id): OrganizationId,
    ValidatedMultipart(dto): ValidatedMultipart<CreateVehicleDto>,
) -> Result<Json<entity::vehicle::Model>, (StatusCode, SimpleError)> {
    let created_vehicle = repository::create_vehicle(&state.db, &dto, org_id).await?;

    if let Some(photo) = dto.photo {
        let img_validation = multipart_form_data::filename_from_img("photo", &photo);

        let filename = match img_validation {
            Ok(filename) => filename,
            Err(e) => {
                // Creating the vehicle without the uploaded photo is not acceptable
                // therefore delete the created vehicle and return a error response.
                let _ = created_vehicle.delete(&state.db).await;

                return Err(e);
            }
        };

        let folder = format!("organization/{}/vehicle/{}", org_id, created_vehicle.id);

        let key = S3Key { folder, filename };

        if state
            .s3
            .upload(key.clone().into(), photo.contents)
            .await
            .is_err()
        {
            let _ = created_vehicle.delete(&state.db).await;

            return Err(internal_error_msg("failed to upload vehicle photo"));
        };

        let uploaded_photo = String::from(key.clone());

        let update_photo_on_db_result = entity::vehicle::Entity::update_many()
            .col_expr(
                entity::vehicle::Column::Photo,
                Expr::value(uploaded_photo.clone()),
            )
            .filter(entity::vehicle::Column::Id.eq(created_vehicle.id))
            .exec(&state.db)
            .await;

        if let Err(_) = update_photo_on_db_result {
            let _ = state.s3.delete(uploaded_photo).await;
            let _ = created_vehicle.delete(&state.db).await;

            return Err(internal_error_msg("failed to set vehicle photo"));
        }
    }

    Ok(Json(created_vehicle))
}
