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
use axum::{
    body::{self, BoxBody, Bytes, HttpBody},
    extract::State,
    response::{IntoResponse, Response},
    BoxError,
};
use futures_util::future::BoxFuture;
use http::Request;
use http::StatusCode;
use std::convert::Infallible;
use std::task::Context;
use std::task::Poll;
use tower::{Layer, Service};

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

/// check if every permission on `permissions` is present in the user access level
pub fn user_contains_permissions(user: &RequestUser, permissions: &Vec<String>) -> bool {
    let user_permissions: Vec<String> = user
        .0
        .access_level
        .permissions
        .iter()
        .filter_map(|e| e.to_owned())
        .collect();

    permissions
        .iter()
        .all(|item| user_permissions.contains(item))
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
    required_permissions: Vec<String>,
}

impl AclLayer {
    pub fn new(required_permissions: Vec<String>) -> Self {
        AclLayer {
            required_permissions,
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
    required_permissions: Vec<String>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AclMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    Infallible: From<<S as Service<Request<ReqBody>>>::Error>,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let maybe_not_ready_inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, maybe_not_ready_inner);

        if let Some(req_user) = req.extensions().get::<RequestUser>() {
            let has_permissions = user_contains_permissions(req_user, &self.required_permissions);

            return Box::pin(async move {
                if has_permissions {
                    Ok(inner.call(req).await?.map(body::boxed))
                } else {
                    Ok((
                        StatusCode::UNAUTHORIZED,
                        SimpleError::from("user lacks permissions"),
                    )
                        .into_response())
                }
            });
        }

        Box::pin(async move {
            // this should be a internal error and not a UNAUTHORIZED response
            // since the request user should be available on the extensions.
            Ok(internal_error_response_with_msg("cannot check user permissions").into_response())
        })
    }
}
