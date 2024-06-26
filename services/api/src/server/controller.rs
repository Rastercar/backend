use super::open_api;
use crate::{
    config::app_config,
    modules::{
        access_level,
        auth::{self, service::AuthService},
        organization, sim_card, tracker,
        tracking::{self},
        user, vehicle,
    },
    rabbitmq::Rmq,
    services::{mailer::service::MailerService, s3::S3},
    utils::string::StringExt,
};
use axum::{body::Body, routing::get, Router};
use axum_client_ip::SecureClientIpSource;
use http::{header, HeaderValue, Method, Request, StatusCode};
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level, Span};

/// The main application state, this is cloned for every HTTP / WS
/// request and thus its fields should contain types that are cheap
/// to clone.
#[derive(Clone)]
pub struct AppState {
    pub s3: S3,
    pub db: DatabaseConnection,
    pub auth_service: AuthService,
    pub mailer_service: MailerService,
}

/// Creates the main axum router/controller to be served over https
pub fn new(db: DatabaseConnection, s3: S3, rmq: Arc<Rmq>) -> Router {
    let rng = ChaCha8Rng::seed_from_u64(OsRng.next_u64());

    let positions_consumer_rmq = rmq.clone();

    let state = AppState {
        s3,
        db: db.clone(),
        auth_service: AuthService::new(db.clone(), rng),
        mailer_service: MailerService::new(rmq),
    };

    let (socket_io_layer, socket_io) = socketioxide::SocketIo::builder()
        .with_state(state.clone())
        .build_layer();

    socket_io.ns("/tracking", tracking::routes::on_connect);

    tracking::background::start_positions_consumer(positions_consumer_rmq, socket_io, db);

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

    let tracing_layer = TraceLayer::new_for_http()
        .on_request(|request: &Request<Body>, _span: &Span| {
            info!("{} {}", request.method(), request.uri().path())
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    let global_middlewares = ServiceBuilder::new()
        .layer(ip_extractor_layer)
        .layer(tracing_layer)
        .layer(cors)
        .layer(socket_io_layer);

    Router::new()
        .merge(open_api::create_openapi_router())
        .route("/healthcheck", get(healthcheck))
        .nest("/auth", auth::routes::create_router(state.clone()))
        .nest("/user", user::routes::create_router(state.clone()))
        .nest("/vehicle", vehicle::routes::create_router(state.clone()))
        .nest("/sim-card", sim_card::routes::create_router(state.clone()))
        .nest("/tracker", tracker::routes::create_router(state.clone()))
        .nest("/tracking", tracking::routes::create_router(state.clone()))
        .nest(
            "/access-level",
            access_level::routes::create_router(state.clone()),
        )
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
