use super::dto::UpdateOrganizationDto;
use crate::{
    database::models,
    modules::{
        auth::{
            self,
            middleware::{AclLayer, RequestUser},
        },
        common::{
            error_codes::EMAIL_ALREADY_VERIFIED,
            extractors::ValidatedJson,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{
    extract::State,
    routing::{patch, post},
    Extension, Json, Router,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", patch(update_org))
        .route(
            "/request-email-address-confirmation",
            post(request_email_address_confirmation),
        )
        // .route(
        //     "/confirm-email-address-by-token",
        //     post(confirm_email_address_by_token),
        // )
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

/// Requests a organization email address confirmation email
///
/// Required permissions: UPDATE_USER
///
/// Sends a billing email address confirmation email to the request user organization email address
#[utoipa::path(
    post,
    path = "/organization/request-billing-email-address-confirmation",
    tag = "organization",
    responses(
        (
            status = OK,
            description = "success message",
            body = String,
            content_type = "application/json",
            example = json!("a confirmation email was sent"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "invalid dto error message / EMAIL_ALREADY_CONFIRMED",
            body = SimpleError,
        ),
    ),
)]
pub async fn request_email_address_confirmation(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    if let Some(user_org) = req_user.0.organization {
        if user_org.billing_email_verified {
            return Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(EMAIL_ALREADY_VERIFIED),
            ));
        }

        // TODO: token should be set on organization table
        let token = state
            .auth_service
            // TODO: should be org ID here
            .gen_and_set_user_confirm_email_token(1)
            .await
            .or(Err(internal_error_response()))?;

        // TODO: change template !
        state
            .mailer_service
            .send_confirm_email_address_email(user_org.billing_email, token)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("email address confirmation email queued successfully"));
    }

    return Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("user does not have a organization to verify emails"),
    ));
}
