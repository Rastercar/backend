use super::dto::UpdateOrganizationDto;
use crate::{
    database::error::DbError,
    modules::{
        auth::{
            self, jwt,
            middleware::{AclLayer, RequestUser},
        },
        common::{
            self,
            error_codes::EMAIL_ALREADY_VERIFIED,
            extractors::{DbConnection, ValidatedJson},
            responses::{internal_error_res, SimpleError},
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
use http::StatusCode;
use migration::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryTrait};
use shared::Permission;

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
        .layer(AclLayer::new(vec![Permission::UpdateOrganization]))
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
    tag = "organization",
    path = "/organization",
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
    DbConnection(db): DbConnection,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<UpdateOrganizationDto>,
) -> Result<Json<auth::dto::OrganizationDto>, (StatusCode, SimpleError)> {
    if let Some(org) = req_user.0.organization {
        entity::organization::Entity::update_many()
            .apply_if(payload.name, |query, v| {
                query.col_expr(entity::organization::Column::Name, Expr::value(v))
            })
            .apply_if(payload.billing_email, |query, v| {
                query.col_expr(entity::organization::Column::BillingEmail, Expr::value(v))
            })
            .filter(entity::organization::Column::Id.eq(org.id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;

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
    tag = "organization",
    path = "/organization/request-billing-email-address-confirmation",
    security(("session_id" = [])),
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
            .or(Err(internal_error_res()))?;

        state
            .mailer_service
            .send_confirm_email_address_email(
                user_org.billing_email,
                token,
                ConfirmEmailRecipientType::Organization,
            )
            .await
            .or(Err(internal_error_res()))?;

        return Ok(Json("email address confirmation email queued successfully"));
    }

    return Err((
        StatusCode::BAD_REQUEST,
        SimpleError::from("user does not have a organization to verify emails"),
    ));
}

/// Confirm org email address by token
///
/// Required permissions: UPDATE_ORGANIZATION
///
/// Confirms the email address of the organization with this token
#[utoipa::path(
    post,
    tag = "organization",
    path = "/organization/confirm-email-address-by-token",
    request_body = Token,
    security(("session_id" = [])),
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
    DbConnection(db): DbConnection,
    ValidatedJson(payload): ValidatedJson<common::dto::Token>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    jwt::decode(&payload.token).or(Err((
        StatusCode::UNAUTHORIZED,
        SimpleError::from("invalid token"),
    )))?;

    let maybe_org = entity::organization::Entity::find()
        .filter(entity::organization::Column::ConfirmBillingEmailToken.eq(&payload.token))
        .one(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(org) = maybe_org {
        if org.billing_email_verified {
            return Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(EMAIL_ALREADY_VERIFIED),
            ));
        }

        entity::organization::Entity::update_many()
            .col_expr(
                entity::organization::Column::BillingEmailVerified,
                Expr::value(false),
            )
            .col_expr(
                entity::organization::Column::ConfirmBillingEmailToken,
                Expr::value::<Option<String>>(None),
            )
            .filter(entity::organization::Column::Id.eq(org.id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;

        return Ok(Json("email confirmed successfully"));
    }

    Err((
        StatusCode::NOT_FOUND,
        SimpleError::from("user not found with this reset password token"),
    ))
}
