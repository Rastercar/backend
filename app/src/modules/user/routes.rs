use super::super::auth::dto as auth_dto;
use super::dto::{self, ListUsersDto, SimpleUserDto};
use crate::database::error::DbError;
use crate::modules::access_level::dto::AccessLevelDto;
use crate::modules::auth::dto::SessionDto;
use crate::modules::auth::middleware::{AclLayer, RequestUserPassword};
use crate::modules::auth::session::SessionId;
use crate::modules::common::dto::{Pagination, PaginationResult, SingleImageDto};
use crate::modules::common::error_codes::EMAIL_ALREADY_VERIFIED;
use crate::modules::common::extractors::{DbConnection, OrganizationId, ValidatedQuery};
use crate::modules::common::responses::internal_error_msg;
use crate::services::mailer::service::ConfirmEmailRecipientType;
use crate::{
    modules::{
        auth::{self, dto::UserDto, middleware::RequestUser},
        common::{
            extractors::ValidatedJson,
            multipart_form_data,
            responses::{internal_error_res, SimpleError},
        },
    },
    server::controller::AppState,
    services::s3::S3Key,
};
use axum::extract::Path;
use axum::{
    extract::State,
    routing::{get, post, put},
    Extension, Json, Router,
};
use axum_typed_multipart::TypedMultipart;
use bcrypt::{hash, verify, DEFAULT_COST};
use entity::user;
use http::StatusCode;
use migration::Expr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QueryTrait};
use sea_query::extension::postgres::PgExpr;
use shared::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/:user_id", get(get_user))
        //
        .route("/:user_id/session", get(get_user_sessions))
        .layer(AclLayer::new(vec![Permission::ListUserSessions]))
        .route("/:user_id/access-level", get(get_user_access_level))
        .route("/:user_id/access-level", put(change_user_access_level))
        .layer(AclLayer::new(vec![Permission::ManageUserAccessLevels]))
        //
        .route("/me", get(me).patch(update_me))
        .route("/me/session", get(get_request_user_sessions))
        .route("/me/password", put(put_password))
        .route(
            "/me/profile-picture",
            put(put_profile_picture).delete(delete_profile_picture),
        )
        .route(
            "/me/request-email-address-confirmation",
            post(request_user_email_address_confirmation),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// List all sessions for the request user
#[utoipa::path(
    get,
    tag = "user",
    path = "/user/me/sessions",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = Vec<SessionDto>,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn get_request_user_sessions(
    State(state): State<AppState>,
    Extension(session): Extension<SessionId>,
    Extension(req_user): Extension<RequestUser>,
) -> Result<Json<Vec<SessionDto>>, (StatusCode, SimpleError)> {
    let current_session_id = session.get_id();

    let sessions = state
        .auth_service
        .get_active_user_sessions(req_user.0.id)
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to list sessions"),
        )))?
        .iter()
        .map(|s| {
            let mut session_dto = SessionDto::from(s.clone());

            let session_id = SessionId::from_database_value(s.session_token.clone())
                .expect("failed convert session id from database value")
                .get_id();

            if current_session_id == session_id {
                session_dto.same_as_from_request = true
            }

            session_dto
        })
        .collect();

    Ok(Json(sessions))
}

/// List users belonging to a organization
#[utoipa::path(
    get,
    tag = "user",
    path = "/user",
    security(("session_id" = [])),
    params(
        Pagination,
        ListUsersDto
    ),
    responses(
        (
            status = OK,
            description = "paginated list of users",
            content_type = "application/json",
            body = PaginatedUser,
        ),
    ),
)]
pub async fn list_users(
    ValidatedQuery(pagination): ValidatedQuery<Pagination>,
    ValidatedQuery(filter): ValidatedQuery<ListUsersDto>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<PaginationResult<dto::SimpleUserDto>>, (StatusCode, SimpleError)> {
    let paginator = entity::user::Entity::find()
        .filter(entity::user::Column::OrganizationId.eq(org_id))
        .apply_if(filter.email, |query, email| {
            if email != "" {
                let col = Expr::col((entity::user::Entity, entity::user::Column::Email));
                query.filter(col.ilike(format!("%{}%", email)))
            } else {
                query
            }
        })
        .apply_if(filter.access_level_id, |query, access_level_id| {
            if access_level_id > 0 {
                let col = Expr::col((entity::user::Entity, entity::user::Column::AccessLevelId));
                query.filter(col.eq(access_level_id))
            } else {
                query
            }
        })
        .order_by_asc(entity::user::Column::Id)
        .paginate(&db, pagination.page_size);

    let n = paginator
        .num_items_and_pages()
        .await
        .map_err(DbError::from)?;

    let rows = paginator
        .fetch_page(pagination.page - 1)
        .await
        .map_err(DbError::from)?;

    let records: Vec<dto::SimpleUserDto> = rows.into_iter().map(SimpleUserDto::from).collect();

    let result = PaginationResult {
        page: pagination.page,
        records,
        page_size: pagination.page_size,
        item_count: n.number_of_items,
        page_count: n.number_of_pages,
    };

    Ok(Json(result))
}

/// Get a user by ID
#[utoipa::path(
    get,
    tag = "user",
    path = "/user/{user_id}",
    security(("session_id" = [])),
    params(
        ("user_id" = u128, Path, description = "id of the user"),
    ),
    responses(
        (
            status = OK,
            content_type = "application/json",
            body = user::dto::SimpleUserDto,
        )
    ),
)]
pub async fn get_user(
    Path(user_id): Path<i32>,
    OrganizationId(org_id): OrganizationId,
    DbConnection(db): DbConnection,
) -> Result<Json<dto::SimpleUserDto>, (StatusCode, SimpleError)> {
    let user = entity::user::Entity::find_by_id_and_org_id(org_id, user_id, &db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::from("user not found")))?;

    Ok(Json(dto::SimpleUserDto::from(user)))
}

/// Get a list of a user sessions
///
/// Required permissions: LIST_USER_SESSIONS
#[utoipa::path(
    get,
    tag = "user",
    path = "/user/{user_id}/sessions",
    security(("session_id" = [])),
    params(
        ("user_id" = u128, Path, description = "id of the user to get the sessions"),
    ),
    responses(
        (
            status = OK,
            body = Vec<SessionDto>,
        ),
    ),
)]
pub async fn get_user_sessions(
    Path(user_id): Path<i32>,
    Extension(session): Extension<SessionId>,
    State(state): State<AppState>,
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
) -> Result<Json<Vec<SessionDto>>, (StatusCode, SimpleError)> {
    let _ = entity::user::Entity::find_by_id_and_org_id(user_id, org_id, &db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::from("user not found")))?;

    let current_session_id = session.get_id();

    let sessions = state
        .auth_service
        .get_active_user_sessions(user_id)
        .await
        .or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::from("failed to list sessions"),
        )))?
        .iter()
        .map(|s| {
            let mut session_dto = SessionDto::from(s.clone());

            let session_id = SessionId::from_database_value(s.session_token.clone())
                .expect("failed convert session id from database value")
                .get_id();

            if current_session_id == session_id {
                session_dto.same_as_from_request = true
            }

            session_dto
        })
        .collect();

    Ok(Json(sessions))
}

/// Get a user access level
#[utoipa::path(
    get,
    tag = "user",
    path = "/user/{user_id}/access-level",
    security(("session_id" = [])),
    params(
        ("user_id" = u128, Path, description = "id of the user to get the acess level"),
    ),
    responses(
        (
            status = OK,
            body = access_level::dto::AccessLevelDto,
        ),
    ),
)]
pub async fn get_user_access_level(
    Path(user_id): Path<i32>,
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
) -> Result<Json<AccessLevelDto>, (StatusCode, SimpleError)> {
    let access_level = entity::access_level::Entity::find()
        .inner_join(entity::user::Entity)
        .filter(user::Column::Id.eq(user_id))
        .filter(user::Column::OrganizationId.eq(org_id))
        .one(&db)
        .await
        .map_err(DbError::from)?
        .ok_or((
            StatusCode::NOT_FOUND,
            SimpleError::from("user / access level not found"),
        ))?;

    Ok(Json(AccessLevelDto::from(access_level)))
}

/// Change a user access level
///
/// Required permissions: MANAGE_USER_ACCESS_LEVELS
#[utoipa::path(
    put,
    tag = "user",
    path = "/user/{user_id}/access-level",
    security(("session_id" = [])),
    request_body = ChangeUserAccessLevelDto,
    params(
        ("user_id" = u128, Path, description = "id of the user to change the acess level"),
    ),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            example = json!("access level changed successfully"),
        ),
    ),
)]
pub async fn change_user_access_level(
    Path(user_id): Path<i32>,
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<dto::ChangeUserAccessLevelDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    if req_user.0.id == user_id {
        return Err((
            StatusCode::FORBIDDEN,
            SimpleError::from("cannot change your own access level"),
        ));
    }

    let user_to_update = entity::user::Entity::find_by_id_and_org_id(user_id, org_id, &db)
        .await
        .map_err(DbError::from)?
        .ok_or((StatusCode::NOT_FOUND, SimpleError::from("user not found")))?;

    let new_access_level =
        entity::access_level::Entity::find_by_id_and_org_id(payload.access_level_id, org_id, &db)
            .await
            .map_err(DbError::from)?
            .ok_or((
                StatusCode::NOT_FOUND,
                SimpleError::from("new access level not found"),
            ))?;

    if user_to_update.access_level_id != new_access_level.id {
        entity::user::Entity::update_many()
            .col_expr(
                user::Column::AccessLevelId,
                Expr::value(new_access_level.id),
            )
            .filter(entity::user::Column::Id.eq(user_to_update.id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;
    }

    Ok(Json(String::from("access level changed successfully")))
}

/// Returns the request user
///
/// the request user is the user that owns the session on the session id (sid) cookie
#[utoipa::path(
    get,
    tag = "user",
    path = "/user/me",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = UserDto,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn me(Extension(req_user): Extension<RequestUser>) -> Json<UserDto> {
    Json(UserDto::from(req_user.0))
}

/// Updates the request user
#[utoipa::path(
    patch,
    tag = "user",
    path = "/user/me",
    security(("session_id" = [])),
    request_body = UpdateUserDto,
    responses(
        (
            status = OK,
            description = "the updated user",
            body = UserDto,
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
pub async fn update_me(
    DbConnection(db): DbConnection,
    Extension(req_user): Extension<RequestUser>,
    ValidatedJson(payload): ValidatedJson<dto::UpdateUserDto>,
) -> Result<Json<auth_dto::UserDto>, (StatusCode, SimpleError)> {
    let mut req_user = req_user.0;

    entity::user::Entity::update_many()
        .apply_if(payload.description.clone(), |query, v| {
            query.col_expr(entity::user::Column::Description, Expr::value(v))
        })
        .apply_if(payload.email.clone(), |query, v| {
            query.col_expr(entity::user::Column::Email, Expr::value(v))
        })
        .apply_if(payload.username.clone(), |query, v| {
            query.col_expr(entity::user::Column::Username, Expr::value(v))
        })
        .filter(entity::user::Column::Id.eq(req_user.id.clone()))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(new_description) = payload.description {
        req_user.description = new_description;
    }

    if let Some(new_username) = payload.username {
        req_user.username = new_username;
    }

    if let Some(new_email) = payload.email {
        req_user.email = new_email;
    }

    Ok(Json(req_user))
}

/// Changes the user password
#[utoipa::path(
    put,
    tag = "user",
    path = "/user/me/password",
    security(("session_id" = [])),
    request_body(content = ChangePasswordDto),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            example = json!("password changed successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
        (
            status = BAD_REQUEST,
            description = "weak password",
            body = SimpleError,
        ),
    ),
)]
async fn put_password(
    DbConnection(db): DbConnection,
    Extension(req_user): Extension<RequestUser>,
    Extension(req_user_password): Extension<RequestUserPassword>,
    ValidatedJson(payload): ValidatedJson<dto::ChangePasswordDto>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let request_user = req_user.0;

    let old_password_valid =
        verify(payload.old_password, req_user_password.0.as_str()).or(Err(internal_error_res()))?;

    if !old_password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            SimpleError::from("invalid password"),
        ));
    }

    let new_password_hash = hash(payload.new_password, DEFAULT_COST)
        .or(Err(internal_error_msg("error hashing password")))?;

    entity::user::Entity::update_many()
        .col_expr(
            entity::user::Column::Password,
            Expr::value(new_password_hash),
        )
        .filter(entity::user::Column::Id.eq(request_user.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    Ok(Json("password changed successfully"))
}

/// Replaces the request user profile picture
#[utoipa::path(
    put,
    tag = "user",
    path = "/user/me/profile-picture",
    security(("session_id" = [])),
    request_body(content = SingleImageDto, content_type = "multipart/form-data"),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            description = "S3 object key of the new profile picture",
            example = json!("rastercar/organization/1/user/2/profile-picture_20-10-2023_00:19:17.jpeg"),
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
async fn put_profile_picture(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    DbConnection(db): DbConnection,
    TypedMultipart(SingleImageDto { image }): TypedMultipart<SingleImageDto>,
) -> Result<Json<String>, (StatusCode, SimpleError)> {
    let filename = multipart_form_data::filename_from_img("profile-picture", &image)?;

    let request_user = req_user.0;

    let folder = match request_user.organization {
        Some(org) => format!("organization/{}/user/{}", org.id, request_user.id),
        None => format!("user/{}", request_user.id),
    };

    let key = S3Key { folder, filename };

    state
        .s3
        .upload(key.clone().into(), image.contents)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to upload new profile picture"),
            )
        })?;

    entity::user::Entity::update_many()
        .col_expr(
            entity::user::Column::ProfilePicture,
            Expr::value(String::from(key.clone())),
        )
        .filter(entity::user::Column::Id.eq(request_user.id))
        .exec(&db)
        .await
        .map_err(DbError::from)?;

    if let Some(old_profile_pic) = request_user.profile_picture {
        let _ = state.s3.delete(old_profile_pic).await;
    }

    Ok(Json(String::from(key)))
}

/// Removes the request user profile picture
#[utoipa::path(
    delete,
    tag = "user",
    path = "/user/me/profile-picture",
    security(("session_id" = [])),
    responses(
        (
            status = OK,
            body = String,
            content_type = "application/json",
            example = json!("profile picture removed successfully"),
        ),
        (
            status = UNAUTHORIZED,
            description = "invalid session",
            body = SimpleError,
        ),
    ),
)]
async fn delete_profile_picture(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
    DbConnection(db): DbConnection,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    let request_user = req_user.0;

    if let Some(old_profile_pic) = request_user.profile_picture {
        entity::user::Entity::update_many()
            .col_expr(
                entity::user::Column::ProfilePicture,
                Expr::value::<Option<String>>(None),
            )
            .filter(entity::user::Column::Id.eq(request_user.id))
            .exec(&db)
            .await
            .map_err(DbError::from)?;

        let _ = state.s3.delete(old_profile_pic).await;

        return Ok(Json("profile picture removed successfully"));
    }

    Ok(Json("user does not have a profile picture to remove"))
}

/// Requests a email address confirmation email
///
/// sends a email address confirmation email to be sent to the request user email address
#[utoipa::path(
    post,
    tag = "user",
    path = "/user/me/request-email-address-confirmation",
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
pub async fn request_user_email_address_confirmation(
    State(state): State<AppState>,
    Extension(req_user): Extension<RequestUser>,
) -> Result<Json<&'static str>, (StatusCode, SimpleError)> {
    if req_user.0.email_verified {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(EMAIL_ALREADY_VERIFIED),
        ));
    }

    let token = state
        .auth_service
        .gen_and_set_user_confirm_email_token(req_user.0.id)
        .await
        .or(Err(internal_error_res()))?;

    state
        .mailer_service
        .send_confirm_email_address_email(req_user.0.email, token, ConfirmEmailRecipientType::User)
        .await
        .or(Err(internal_error_res()))?;

    Ok(Json("email address confirmation email queued successfully"))
}
