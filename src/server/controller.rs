use super::open_api;
use crate::{
    config::app_config,
    modules::{
        auth::routes::create_auth_router,
        auth::service::{new_auth_service, AuthService},
        common::responses::SimpleError,
        user::routes::create_user_router,
    },
    services::{mailer::service::MailerService, s3::S3},
};
use axum::{routing::get, Router};
use axum_client_ip::SecureClientIpSource;
use deadpool_lapin::Pool as RmqPool;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use http::{header, HeaderValue, Method, StatusCode};
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub s3: S3,
    pub auth_service: AuthService,
    pub mailer_service: MailerService,
    pub db_conn_pool: Pool<AsyncPgConnection>,
}

impl AppState {
    pub async fn get_db_conn(
        &self,
    ) -> Result<
        deadpool::managed::Object<AsyncDieselConnectionManager<AsyncPgConnection>>,
        (StatusCode, SimpleError),
    > {
        Ok(self.db_conn_pool.get().await.or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::internal(),
        )))?)
    }
}

/// Creates the main axum router/controller to be served over https
pub fn new(db_conn_pool: Pool<AsyncPgConnection>, rmq_conn_pool: RmqPool, s3: S3) -> Router {
    let rng = ChaCha8Rng::seed_from_u64(OsRng.next_u64());

    let state = AppState {
        s3,
        db_conn_pool: db_conn_pool.clone(),
        mailer_service: MailerService::new(rmq_conn_pool),
        auth_service: new_auth_service(db_conn_pool.clone(), rng),
    };

    let frontend_origin = app_config()
        .frontend_url
        .to_string()
        .parse::<HeaderValue>()
        .expect("failed to parse CORS allowed origins");

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::PUT,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_origin(frontend_origin)
        .allow_credentials(true)
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE]);

    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(open_api::create_openapi_router())
        .nest("/auth", create_auth_router(state.clone()))
        .nest("/user", create_user_router(state.clone()))
        .layer(SecureClientIpSource::ConnectInfo.into_extension())
        .layer(cors)
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
