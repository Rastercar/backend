use super::{
    dto::{self, UserDto},
    service::UserDtoEntities,
    session::get_session_id_from_request_headers,
};
use crate::{
    modules::{
        auth::session::SessionId,
        common::{
            error_codes::{
                INVALID_SESSION, MISSING_PERMISSIONS, NO_SID_COOKIE, ORGANIZATION_BLOCKED,
            },
            responses::{internal_error_msg, ApiError, SimpleError},
        },
    },
    server::controller::AppState,
};
use anyhow::Error;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use convert_case::{Case, Casing};
use futures_util::future::BoxFuture;
use http::Request;
use http::StatusCode;
use shared::constants::Permission;
use std::convert::Infallible;
use std::task::Context;
use std::task::Poll;
use tower::{Layer, Service};

/// Simple extractor for routes that are only allowed for regular users
#[derive(Clone)]
pub struct RequestUser(pub dto::UserDto);

impl RequestUser {
    /// Returns the ID the organization the user belongs to, if `None`
    /// the user is not bound to a org and is a admin user.
    pub fn get_org_id(&self) -> Option<i32> {
        self.0.organization.as_ref().map(|user| user.id)
    }

    /// get the missing permissions the user does not have
    pub fn get_missing_permissions(&self, required_permissions: &[Permission]) -> Vec<String> {
        required_permissions
            .iter()
            .map(|required_permission| {
                required_permission
                    .to_string()
                    .to_case(Case::ScreamingSnake)
            })
            .filter(|item| !self.0.access_level.permissions.contains(item))
            .collect()
    }
}

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

    Err(internal_error_msg("failed to fetch user session"))
}

/// middleware for routes that require a normal user, this queries the DB to get the request user by his session ID cookie,
/// so use it only within routes that need the user data, adds the following extensions:
///
/// - `SessionId`
/// - `RequestUser`
/// - `RequestUserPassword`
pub async fn require_user(
    State(state): State<AppState>,
    mut req: http::Request<axum::body::Body>,
    next: axum::middleware::Next,
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

/// A layer to be used as a middleware to authorize users.
///
/// this requires the `RequestUser` extension to be available for the route
/// its protecting, otherwise the request will always fail since there is no
/// user to check permissions against.
#[derive(Clone)]
pub struct AclLayer {
    /// list of permissions the role of the request user must have
    /// to allow the request to continue
    required_permissions: Vec<Permission>,
}

impl AclLayer {
    pub fn single(required_permission: Permission) -> Self {
        AclLayer {
            required_permissions: vec![required_permission],
        }
    }
}

impl<S> Layer<S> for AclLayer {
    type Service = AclMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AclMiddleware {
            inner,
            required_permissions: self.required_permissions.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AclMiddleware<S> {
    /// inner service to execute, normally the next middleware or the final route handler
    inner: S,
    required_permissions: Vec<Permission>,
}

impl<S> Service<Request<axum::body::Body>> for AclMiddleware<S>
where
    S: Service<
            Request<axum::body::Body>,
            Response = Response<axum::body::Body>,
            Error = Infallible,
        > + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Box<axum::body::Body>>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<axum::body::Body>) -> Self::Future {
        let maybe_not_ready_inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, maybe_not_ready_inner);

        if let Some(req_user) = req.extensions().get::<RequestUser>() {
            let missing_permissions = req_user.get_missing_permissions(&self.required_permissions);

            return Box::pin(async move {
                if missing_permissions.is_empty() {
                    return Ok(inner.call(req).await?.map(Box::new));
                }

                let err = ApiError {
                    error: String::from(MISSING_PERMISSIONS),
                    info: Some(missing_permissions),
                };

                Ok((StatusCode::FORBIDDEN, err).into_response().map(Box::new))
            });
        }

        Box::pin(async {
            let response = internal_error_msg("cannot check user permissions").into_response();
            Ok(response.map(Box::new))
        })
    }
}
