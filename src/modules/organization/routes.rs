use super::dto::UpdateOrganizationDto;
use crate::{
    database::models,
    modules::{
        auth::{
            self,
            middleware::{AclLayer, RequestUser},
        },
        common::{
            extractors::ValidatedJson,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{extract::State, routing::patch, Extension, Json, Router};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", patch(update_org))
        .layer(AclLayer::new(vec![String::from("UPDATE_ORGANIZATION")]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Updates the user organization
///
/// Required permissions: UPDATE_USER
#[utoipa::path(
    patch,
    path = "/organization",
    tag = "organization",
    security(("session_id" = [])),
    request_body = UpdateOrganizationDto,
    responses(
        (
            status = OK,
            description = "the updated organization",
            body = OrganizationDto,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = FORBIDDEN,
            description = "user lacks permissions",
            body = SimpleError,
        ),
    ),
)]
pub async fn update_org(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<UpdateOrganizationDto>,
) -> Result<Json<auth::dto::OrganizationDto>, (StatusCode, SimpleError)> {
    if let Some(org) = req_user.0.organization {
        let conn = &mut state.get_db_conn().await?;

        use crate::database::schema::organization::dsl::*;

        let org = diesel::update(organization)
            .filter(id.eq(&org.id))
            .set(&payload)
            .get_result::<models::Organization>(conn)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json(auth::dto::OrganizationDto::from(org)));
    }

    Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("cannot update org because user does not belong to one"),
    ))
}
