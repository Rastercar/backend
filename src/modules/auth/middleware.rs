use super::{
    dto::{self, UserDto},
    session::get_session_id_from_request_headers,
};
use crate::{
    modules::{
        auth::session::SessionToken,
        common::{
            error_codes::{INVALID_SESSION, NO_SID_COOKIE, ORGANIZATION_BLOCKED},
            responses::{internal_error_response_with_msg, SimpleError},
        },
    },
    server::controller::AppState,
};
use anyhow::Error;
use axum::{extract::State, response::Response};
use http::StatusCode;

/// Simple extractor for routes that are only allowed for regular users
#[derive(Clone)]
pub struct RequestUser(pub dto::UserDto);

fn handle_fetch_user_result(
    user_fetch_result: Result<Option<UserDto>, Error>,
) -> Result<UserDto, (http::StatusCode, SimpleError)> {
    if let Ok(maybe_user) = user_fetch_result {
        return match maybe_user {
            Some(user) => {
                if let Some(org) = user.organization.clone() {
                    if org.blocked {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            SimpleError::from(ORGANIZATION_BLOCKED),
                        ));
                    }
                }

                Ok(user)
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
/// - `SessionToken`
pub async fn user_only_route<B>(
    State(state): State<AppState>,
    mut req: http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<Response, (StatusCode, SimpleError)> {
    let mut headers = req.headers().clone();

    if let Some(session_id) = get_session_id_from_request_headers(&mut headers) {
        let session_token = SessionToken::from(session_id);

        let user_fetch_result = state
            .auth_service
            .get_user_from_session_token(session_token)
            .await;

        let user = handle_fetch_user_result(user_fetch_result)?;

        req.extensions_mut().insert(session_token);
        req.extensions_mut().insert(RequestUser(user));

        return Ok(next.run(req).await);
    }

    Err((StatusCode::UNAUTHORIZED, SimpleError::from(NO_SID_COOKIE)))
}
