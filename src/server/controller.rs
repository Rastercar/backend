use super::open_api;
use crate::modules::{
    auth::routes::create_auth_router,
    auth::service::{new_auth_service, AuthService},
};
use axum::{routing::get, Router};
use axum_client_ip::SecureClientIpSource;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use http::StatusCode;
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub db_conn_pool: Pool<AsyncPgConnection>,
}

/// Creates the main axum router/controller to be served over https
pub fn new(db_conn_pool: Pool<AsyncPgConnection>) -> Router {
    let rng = ChaCha8Rng::seed_from_u64(OsRng.next_u64());

    let state = AppState {
        db_conn_pool: db_conn_pool.clone(),
        auth_service: new_auth_service(db_conn_pool.clone(), rng),
    };

    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(open_api::create_openapi_router())
        .nest("/auth", create_auth_router(state.clone()))
        .layer(SecureClientIpSource::ConnectInfo.into_extension())
        .with_state(state)
}

#[utoipa::path(
    get,
    tag = "meta",
    path = "/healthcheck",
    responses((status = OK)),
)]
pub async fn healthcheck() -> StatusCode {
    StatusCode::OK
}
