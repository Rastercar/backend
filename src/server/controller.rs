use super::open_api;
use crate::{
    config::app_config,
    modules::{
        auth::{
            self,
            service::{new_auth_service, AuthService},
        },
        common::responses::SimpleError,
        organization,
        user::{self},
    },
    services::{mailer::service::MailerService, s3::S3},
    utils::string::StringExt,
};
use axum::{body::Body, routing::get, Router};
use axum_client_ip::SecureClientIpSource;
use deadpool_lapin::Pool as RmqPool;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use http::{header, HeaderValue, Method, Request, StatusCode};
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{Level, Span};

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

    // URL.to_string for some reason adds a trailing slash
    // we need to remove it to avoid cors errors
    let mut frontend_origin = app_config().frontend_url.to_string();
    frontend_origin.pop_if_is('/');

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::PUT,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_origin(
            frontend_origin
                .parse::<HeaderValue>()
                .expect("failed to parse CORS allowed origins"),
        )
        .allow_credentials(true)
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE]);

    // extracts the client IP from the request, this is harder than it sounds and should be
    // done by a lib to deal with edge cases such as extracting the original IP from a header
    // set by cloudflare or other load balancers.
    let ip_extractor_layer = SecureClientIpSource::ConnectInfo.into_extension();

    // [PROD-TODO] decide on useful values here
    let tracing_layer = TraceLayer::new_for_http()
        .on_request(|request: &Request<Body>, _span: &Span| {
            tracing::info!("request: {} {}", request.method(), request.uri().path())
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    let global_middlewares = ServiceBuilder::new()
        .layer(ip_extractor_layer)
        .layer(tracing_layer)
        .layer(cors);

    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(open_api::create_openapi_router())
        .nest("/auth", auth::routes::create_router(state.clone()))
        .nest("/user", user::routes::create_router(state.clone()))
        .nest(
            "/organization",
            organization::routes::create_router(state.clone()),
        )
        .layer(global_middlewares)
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
