use crate::{
    modules::{auth::middleware::RequestUser, common::responses::SimpleError},
    server::controller::AppState,
};
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Query},
    Json,
};
use axum_typed_multipart::{BaseMultipart, TypedMultipartError};
use http::{request::Parts, Request, StatusCode};
use sea_orm::DatabaseConnection;
use serde::de::DeserializeOwned;
use validator::Validate;

/// Wrapper struct that extracts from the request query exactly `axum::Query<T>`
/// but also requires T to impl `Validate`, if validation fails a bad request code
/// and simple error is returned
#[derive(Clone, Copy)]
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Query::<T>::from_request_parts(parts, state).await {
            Ok(payload) => match payload.validate() {
                Ok(_) => Ok(ValidatedQuery(payload.0)),
                Err(e) => Err((StatusCode::BAD_REQUEST, SimpleError::from(e))),
            },
            Err(rejection) => Err((rejection.status(), SimpleError::from(rejection.to_string()))),
        }
    }
}

/// Wrapper struct that extracts the request body as json exactly as `axum::Json<T>`
/// but also requires T to impl `Validate`, if validation fails a bad request code
/// and simple error is returned
#[derive(Clone, Copy)]
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
                Err(e) => Err((StatusCode::BAD_REQUEST, SimpleError::from(e))),
            },
            Err(rejection) => Err((rejection.status(), SimpleError::from(rejection.to_string()))),
        }
    }
}

/// Wrapper struct that extracts the request body from `axum_typed_multipart::TryFromMultipart`
/// but also requires T to impl `Validate`, if validation fails a bad request code and simple
/// error is returned
#[derive(Clone, Copy)]
pub struct ValidatedMultipart<T>(pub T);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for ValidatedMultipart<T>
where
    BaseMultipart<T, TypedMultipartError>: FromRequest<S, B, Rejection = TypedMultipartError>,
    T: Validate,
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        match BaseMultipart::<T, TypedMultipartError>::from_request(req, state).await {
            Ok(payload) => match payload.data.validate() {
                Ok(_) => Ok(ValidatedMultipart(payload.data)),
                Err(e) => Err((StatusCode::BAD_REQUEST, SimpleError::from(e))),
            },
            Err(rejection) => Err((
                StatusCode::BAD_REQUEST,
                SimpleError::from(rejection.to_string()),
            )),
        }
    }
}

/// Extracts the organization id of the request user, failing with
/// `(StatusCode::BAD_REQUEST, SimpleError::from("route only accessible to organization bound users"))`
/// if the request user is not bound to a organization.
///
/// this requires the `RequestUser` extension to be available.
#[derive(Clone, Copy)]
pub struct OrganizationId(pub i32);

#[async_trait]
impl<S> FromRequestParts<S> for OrganizationId
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let err = (
            StatusCode::FORBIDDEN,
            SimpleError::from("endpoint only for org bound users"),
        );

        if let Some(req_user) = parts.extensions.get::<RequestUser>() {
            let org_id = req_user.get_org_id().ok_or(err)?;

            return Ok(OrganizationId(org_id));
        }

        Err(err)
    }
}

/// Helper to get a DB connection from the state
pub struct DbConnection(pub DatabaseConnection);

#[async_trait]
impl FromRequestParts<AppState> for DbConnection {
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(_: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(DbConnection(state.db.clone()))
    }
}
