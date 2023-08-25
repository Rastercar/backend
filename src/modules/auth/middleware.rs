use super::session::get_session_id_from_request_headers;
use crate::{
    database,
    modules::{auth::session::SessionToken, common::responses::SimpleError},
    server::controller::AppState,
};
use axum::{extract::State, response::Response};
use http::StatusCode;

#[derive(Clone)]
/// Simple extractor for routes that are only allowed for regular users
pub struct RequestUser(pub database::models::User);

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

        let user = state
            .auth_service
            .get_user_from_session_token(session_token)
            .await;

        req.extensions_mut().insert(session_token);

        match user {
            Ok(maybe_user) => match maybe_user {
                Some(user) => {
                    req.extensions_mut().insert(RequestUser(user));
                    Ok(next.run(req).await)
                }
                None => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    SimpleError::from("session not found or expired"),
                )),
            },
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to fetch user session"),
            )),
        }
    } else {
        Err((StatusCode::UNAUTHORIZED, SimpleError::from("sid not found")))
    }
}
