use std::future::Future;

use super::{
    dto::{self, UserDto},
    service::UserDtoEntities,
    session::get_session_id_from_request_headers,
};
use crate::{
    modules::{
        auth::session::SessionId,
        common::{
            error_codes::{INVALID_SESSION, NO_SID_COOKIE, ORGANIZATION_BLOCKED},
            responses::{internal_error_response_with_msg, SimpleError},
        },
    },
    server::controller::AppState,
};
use anyhow::Error;
use axum::{extract::State, response::Response, Extension};
use http::StatusCode;
use tower::Service;

/// Simple extractor for routes that are only allowed for regular users
#[derive(Clone)]
pub struct RequestUser(pub dto::UserDto);

/// The logged in user password, this is exposed as a struct to be used
/// as a AxumExtension to endpoints that need to check the user password
#[derive(Clone)]
pub struct RequestUserPassword(pub String);

fn handle_fetch_user_result(
    user_fetch_result: Result<Option<UserDtoEntities>, Error>,
) -> Result<UserDtoEntities, (http::StatusCode, SimpleError)> {
    if let Ok(maybe_user) = user_fetch_result {
        return match maybe_user {
            Some(entities) => {
                if let Some(org) = entities.2.clone() {
                    if org.blocked {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            SimpleError::from(ORGANIZATION_BLOCKED),
                        ));
                    }
                }

                Ok(entities)
            }
            None => Err((StatusCode::UNAUTHORIZED, SimpleError::from(INVALID_SESSION))),
        };
    }

    Err(internal_error_response_with_msg(
        "failed to fetch user session",
    ))
}

/// middleware for routes that require a normal user, this queries the DB to get the request user by his session ID cookie,
/// so use it only within routes that need the user data, adds the following extensions:
///
/// - `RequestUser`
/// - `RequestUserPassword`
/// - `SessionId`
pub async fn require_user<B>(
    State(state): State<AppState>,
    mut req: http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<Response, (StatusCode, SimpleError)> {
    let mut headers = req.headers().clone();

    if let Some(session_id) = get_session_id_from_request_headers(&mut headers) {
        let session_token = SessionId::from(session_id);

        let user_fetch_result = state
            .auth_service
            .get_user_from_session_id(session_token)
            .await;

        let user_access_level_and_org = handle_fetch_user_result(user_fetch_result)?;

        let user_password = user_access_level_and_org.0.password.clone();

        let user = UserDto::from(user_access_level_and_org);

        req.extensions_mut().insert(session_token);
        req.extensions_mut().insert(RequestUser(user));
        req.extensions_mut()
            .insert(RequestUserPassword(user_password));

        return Ok(next.run(req).await);
    }

    Err((StatusCode::UNAUTHORIZED, SimpleError::from(NO_SID_COOKIE)))
}

// TODO: document me
pub async fn require_permissions<B>(
    required_permissions: Vec<String>,
    Extension(req_user): Extension<RequestUser>,
    State(state): State<AppState>,
    req: http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<Response, (StatusCode, SimpleError)> {
    if req_user.0.id == 4 {
        return Ok(next.run(req).await);
    }

    // TODO: document lacking permissions
    Err((
        StatusCode::UNAUTHORIZED,
        SimpleError::from("user lacks permissions"),
    ))
}

pub fn create_require_permissions<'a, B: 'a>(
    required_permissions: Vec<String>,
) -> impl Fn(
    Extension<RequestUser>,
    State<AppState>,
    http::Request<B>,
    axum::middleware::Next<B>,
)
    -> std::pin::Pin<Box<dyn Future<Output = Result<Response, (StatusCode, SimpleError)>> + 'a>> {
    move |u, s, r, n| {
        Box::pin(require_permissions(
            required_permissions.clone(),
            u,
            s,
            r,
            n,
        ))
    }
}
