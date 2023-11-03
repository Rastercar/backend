use super::dto::CreateVehicleDto;
use crate::{
    database::{models, schema},
    modules::{
        auth::{
            self,
            constants::Permission,
            middleware::{AclLayer, RequestUser},
        },
        common::{
            multipart_form_data,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::{extract::State, Extension};
use axum::{routing::post, Json, Router};
use axum_typed_multipart::TypedMultipart;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_vehicle))
        .layer(AclLayer::new(vec![Permission::CreateVehicle]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

// TODO: add me to open api
/// Creates a new vehicle
#[utoipa::path(
    post,
    path = "/vehicle",
    tag = "vehicle",
    security(("session_id" = [])),
    request_body(content = CreateVehicleDto, content_type = "multipart/form-data"),
    responses(
        (
            // TODO: must be created vehicle
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("password recovery email queued to be sent successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "expired or invalid token",
            body = SimpleError,
        ),
        (
            // TODO: set plate unique constraint to be org_id AND plate, set this on up.sql and create new schema
            status = BAD_REQUEST,
            description = "invalid dto error message / PLATE_IN_USE",
            body = SimpleError,
        ),
    ),
)]
pub async fn create_vehicle(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    TypedMultipart(dto): TypedMultipart<CreateVehicleDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let conn = &mut state.get_db_conn().await?;

    let org_id = req_user
        .0
        .organization
        .ok_or((StatusCode::FORBIDDEN, SimpleError::from("msg")))?
        .id;

    // TODO: check plate in use
    // TODO: move me to a method that returns a error enum with the following
    // CreateVehicleError::PlateInUse
    // CreateVehicleError::DatabaseError
    // CreateVehicleError::PhotoUploadError
    // CreateVehicleError::PhotoUpdateError
    //
    // also document the sad state of affairs regarding the transaction and s3 stuff
    // (maybe just give up and use a transaction ?)
    //
    // also check if theres a way to extract unique violation exceptions from postgres
    //
    // if not a good idea might be locking in a transaction, to be sure the plate wont
    // be created beforehand, but thats overkill
    let created_vehicle = diesel::insert_into(schema::vehicle::dsl::vehicle)
        .values((
            schema::vehicle::dsl::plate.eq(&dto.plate),
            schema::vehicle::dsl::brand.eq(&dto.brand),
            schema::vehicle::dsl::model.eq(&dto.model),
            schema::vehicle::dsl::color.eq(&dto.color),
            schema::vehicle::dsl::model_year.eq(&dto.model_year),
            schema::vehicle::dsl::chassis_number.eq(&dto.chassis_number),
            schema::vehicle::dsl::additional_info.eq(&dto.additional_info),
            schema::vehicle::dsl::organization_id.eq(org_id),
            schema::vehicle::dsl::fabrication_year.eq(&dto.fabrication_year),
        ))
        .get_result::<models::Vehicle>(conn)
        .await
        .or(Err(internal_error_response()))?;

    if let Some(photo) = dto.photo {
        let filename = multipart_form_data::create_filename_with_timestamp_from_uploaded_photo(
            "photo", &photo,
        )?;

        let folder = format!("organization/{}/vehicle/{}", org_id, created_vehicle.id);

        let key = S3Key { folder, filename };

        state
            .s3
            .upload(key.clone().into(), photo.contents)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    SimpleError::from("failed to upload vehicle photo picture"),
                )
            })?;

        let uploaded_vehicle_photo = String::from(key.clone());

        let update_photo_on_db_result = diesel::update(schema::vehicle::dsl::vehicle)
            .filter(schema::vehicle::dsl::id.eq(created_vehicle.id))
            .set(schema::vehicle::dsl::photo.eq(&uploaded_vehicle_photo))
            .execute(conn)
            .await;

        if let Err(_) = update_photo_on_db_result {
            let _ = state.s3.delete(uploaded_vehicle_photo).await;
        }
    }

    todo!();
}
