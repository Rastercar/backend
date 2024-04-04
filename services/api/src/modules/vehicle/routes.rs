use super::dto::{CreateVehicleDto, ListVehiclesDto, UpdateVehicleDto};
use crate::{
    database::{
        error::DbError,
        helpers::{paginated_query_to_pagination_result, set_if_some},
    },
    modules::{
        auth::{self, middleware::AclLayer},
        common::{
            dto::{Pagination, PaginationResult, SingleImageDto},
            extractors::{
                DbConnection, OrgBoundEntityFromPathId, OrganizationId, ValidatedJson,
                ValidatedMultipart, ValidatedQuery,
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
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use http::StatusCode;
use migration::{extension::postgres::PgExpr, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QueryTrait,
};
use shared::constants::Permission;
use shared::entity::{vehicle, vehicle_tracker};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_vehicles))
        //
        .route(
            "/",
            post(create_vehicle).route_layer(AclLayer::single(Permission::CreateVehicle)),
        )
        //
        .route("/:vehicle_id", get(vehicle_by_id))
        //
        .route(
            "/:vehicle_id",
            put(update_vehicle).route_layer(AclLayer::single(Permission::UpdateVehicle)),
        )
        //
        .route(
            "/:vehicle_id",
            delete(delete_vehicle).route_layer(AclLayer::single(Permission::DeleteVehicle)),
        )
        //
        .route("/:vehicle_id/tracker", get(get_vehicle_tracker))
        //
        .route(
            "/:vehicle_id/photo",
            put(update_vehicle_photo).route_layer(AclLayer::single(Permission::UpdateVehicle)),
        )
        //
        .route(
            "/:vehicle_id/photo",
            delete(delete_vehicle_photo).route_layer(AclLayer::single(Permission::UpdateVehicle)),
        )
        //
        .route_layer(axum::middleware::from_fn_with_state(
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
    OrgBoundEntityFromPathId(v): OrgBoundEntityFromPathId<vehicle::Entity>,
) -> Result<Json<vehicle::Model>, (StatusCode, SimpleError)> {
    Ok(Json(v))
}

/// Get a vehicle tracker
#[utoipa::path(
    get,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}/tracker",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to get the tracker"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = Option<entity::vehicle::Model>,
        ),
        (
            status = NOT_FOUND,
        ),
    ),
)]
pub async fn get_vehicle_tracker(
    Path(vehicle_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<Option<vehicle_tracker::Model>>, (StatusCode, SimpleError)> {
    let tracker = vehicle_tracker::Entity::find_by_vehicle_and_org_id(vehicle_id, org_id, &db)
        .await
        .map_err(DbError::from)?;

    Ok(Json(tracker))
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
    request_body(content = UpdateVehicleDto, content_type = "application/json"),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = entity::vehicle::Model,
        ),
    ),
)]
pub async fn update_vehicle(
    DbConnection(db): DbConnection,
    OrgBoundEntityFromPathId(vehicle): OrgBoundEntityFromPathId<vehicle::Entity>,
    ValidatedJson(dto): ValidatedJson<UpdateVehicleDto>,
) -> Result<Json<vehicle::Model>, (StatusCode, SimpleError)> {
    let mut v: vehicle::ActiveModel = vehicle.into();

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

/// Update a vehicle photo
#[utoipa::path(
    put,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}/photo",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to update"),
    ),
    request_body(content = SingleImageDto, content_type = "multipart/form-data"),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            description = "S3 object key of the new vehicle photo",
            example = json!("rastercar/organization/1/vehicle/2/photo-10-2023_00:19:17.jpeg"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid file",
            body = SimpleError,
        ),
    ),
)]
pub async fn update_vehicle_photo(
    Path(vehicle_id): Path<i32>,
    State(state): State<AppState>,
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
    OrgBoundEntityFromPathId(req_vehicle): OrgBoundEntityFromPathId<vehicle::Entity>,
    TypedMultipart(SingleImageDto { image }): TypedMultipart<SingleImageDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let key = S3Key {
        folder: format!("organization/{}/vehicle/{}", org_id, vehicle_id),
        filename: multipart_form_data::filename_from_img("photo", &image)?,
    };

    state
        .s3
        .upload(key.clone().into(), image.contents)
        .await
        .map_err(|_| internal_error_msg("failed to upload vehicle photo"))?;

    vehicle::Entity::update_many()
        .col_expr(
            vehicle::Column::Photo,
            Expr::value(String::from(key.clone())),
        )
        .filter(vehicle::Column::Id.eq(vehicle_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(old_photo) = req_vehicle.photo {
        let _ = state.s3.delete(old_photo).await;
    }

    Ok(Json(String::from(key)))
}

/// Deletes a vehicle photo
#[utoipa::path(
    delete,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}/photo",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to update"),
    ),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            description = "success message",
            example = json!("photo deleted successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn delete_vehicle_photo(
    Path(vehicle_id): Path<i32>,
    State(state): State<AppState>,
    DbConnection(db): DbConnection,
    OrgBoundEntityFromPathId(req_vehicle): OrgBoundEntityFromPathId<vehicle::Entity>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    vehicle::Entity::update_many()
        .col_expr(vehicle::Column::Photo, Expr::value::<Option<String>>(None))
        .filter(vehicle::Column::Id.eq(vehicle_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(old_photo) = req_vehicle.photo {
        let _ = state.s3.delete(old_photo).await;
    }

    Ok(Json(String::from("photo deleted successfuly")))
}

/// Deletes a vehicle
#[utoipa::path(
    delete,
    tag = "vehicle",
    path = "/vehicle/{vehicle_id}",
    security(("session_id" = [])),
    params(
        ("vehicle_id" = u128, Path, description = "id of the vehicle to delete"),
    ),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            description = "success message",
            example = json!("vehicle deleted successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn delete_vehicle(
    Path(vehicle_id): Path<i32>,
    State(state): State<AppState>,
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
    OrgBoundEntityFromPathId(req_vehicle): OrgBoundEntityFromPathId<vehicle::Entity>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let delete_result = vehicle::Entity::delete_many()
        .filter(vehicle::Column::Id.eq(vehicle_id))
        .filter(vehicle::Column::OrganizationId.eq(org_id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(photo) = req_vehicle.photo {
        let _ = state.s3.delete(photo).await;
    }

    if delete_result.rows_affected < 1 {
        let err_msg = "vehicle does not exist or does not belong to the request user organization";
        Err((StatusCode::BAD_REQUEST, SimpleError::from(err_msg)))
    } else {
        Ok(Json(String::from("vehicle deleted successfully")))
    }
}

/// Lists the vehicles that belong to the same org as the request user
#[utoipa::path(
    get,
    tag = "vehicle",
    path = "/vehicle",
    security(("session_id" = [])),
    params(
        Pagination,
        ListVehiclesDto
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
) -> Result<Json<PaginationResult<vehicle::Model>>, (StatusCode, SimpleError)> {
    let db_query = vehicle::Entity::find()
        .filter(vehicle::Column::OrganizationId.eq(org_id))
        .apply_if(filter.plate, |query, plate| {
            if !plate.is_empty() {
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
) -> Result<Json<vehicle::Model>, (StatusCode, SimpleError)> {
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

        let update_photo_on_db_result = vehicle::Entity::update_many()
            .col_expr(vehicle::Column::Photo, Expr::value(uploaded_photo.clone()))
            .filter(vehicle::Column::Id.eq(created_vehicle.id))
            .exec(&state.db)
            .await;

        if update_photo_on_db_result.is_err() {
            let _ = state.s3.delete(uploaded_photo).await;
            let _ = created_vehicle.delete(&state.db).await;

            return Err(internal_error_msg("failed to set vehicle photo"));
        }
    }

    Ok(Json(created_vehicle))
}
