use crate::modules::common::responses::SimpleError;
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest},
    Json,
};
use http::{Request, StatusCode};
use validator::Validate;

/// Wrapper struct that extracts the request body as json exactly as `axum::Json<T>`
/// but also requires T to impl `Validate`, if validation fails a bad request and simple
/// error is returned
#[derive(Clone, Copy, Debug)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for ValidatedJson<T>
where
    Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
    T: Validate,
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(payload) => match payload.validate() {
                Ok(_) => Ok(ValidatedJson(payload.0)),
                Err(e) => Err(((StatusCode::BAD_REQUEST), SimpleError::from(e))),
            },
            Err(rejection) => Err((rejection.status(), SimpleError::from(rejection.to_string()))),
        }
    }
}
