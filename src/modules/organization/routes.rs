use super::dto::UpdateOrganizationDto;
use crate::{
    database::{models, schema::organization},
    modules::{
        auth::{
            self, jwt,
            middleware::{AclLayer, RequestUser},
        },
        common::{
            self,
            error_codes::EMAIL_ALREADY_VERIFIED,
            extractors::ValidatedJson,
            responses::{internal_error_response, SimpleError},
        },
    },
    server::controller::AppState,
    services::mailer::service::ConfirmEmailRecipientType,
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
        .route(
            "/confirm-email-address-by-token",
            post(confirm_email_address_by_token),
        )
        .layer(AclLayer::new(vec![String::from("UPDATE_ORGANIZATION")]))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Updates the user organization
///
/// Required permissions: UPDATE_ORGANIZATION
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

/// Requests org email address confirmation
///
/// Required permissions: UPDATE_ORGANIZATION
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

        let token = state
            .auth_service
            .gen_and_set_org_confirm_email_token(user_org.id)
            .await
            .or(Err(internal_error_response()))?;

        state
            .mailer_service
            .send_confirm_email_address_email(
                user_org.billing_email,
                token,
                ConfirmEmailRecipientType::Organization,
            )
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("email address confirmation email queued successfully"));
    }

    return Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("user does not have a organization to verify emails"),
    ));
}

/// Confirm org email address by token
///
/// Confirms the email address of the organization with this token
#[utoipa::path(
    post,
    path = "/organization/confirm-email-address-by-token",
    tag = "organization",
    request_body = Token,
    responses(
        (
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
            status = BAD_REQUEST,
            description = "invalid dto error message / EMAIL_ALREADY_CONFIRMED",
            body = SimpleError,
        ),
    ),
)]
pub async fn confirm_email_address_by_token(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<common::dto::Token>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let conn = &mut state.get_db_conn().await?;

    jwt::decode(&payload.token).or(Err((
        StatusCode::UNAUTHORIZED,
        SimpleError::from("invalid token"),
    )))?;

    let maybe_org = organization::table
        .select(models::Organization::as_select())
        .filter(organization::dsl::confirm_billing_email_token.eq(&payload.token))
        .first::<models::Organization>(conn)
        .await
        .optional()
        .or(Err(internal_error_response()))?;

    if let Some(org) = maybe_org {
        if org.billing_email_verified {
            return Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(EMAIL_ALREADY_VERIFIED),
            ));
        }

        diesel::update(organization::dsl::organization)
            .filter(organization::dsl::id.eq(org.id))
            .set((
                organization::dsl::billing_email_verified.eq(true),
                organization::dsl::confirm_billing_email_token.eq::<Option<String>>(None),
            ))
            .execute(conn)
            .await
            .or(Err(internal_error_response()))?;

        return Ok(Json("email confirmed successfully"));
    }

    Err((
        StatusCode::NOT_FOUND,
        SimpleError::from("user not found with this reset password token"),
    ))
}