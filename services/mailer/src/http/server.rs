use crate::{
    config::app_config,
    http::routes::{check_aws_sns_arn_middleware, handle_ses_event, healthcheck},
    queue::MailerRabbitmq,
};
use axum::{
    middleware::{self},
    routing::{get, post},
    Router,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

#[derive(Clone)]
pub struct AppState {
    pub mailer_rmq: Arc<MailerRabbitmq>,
    pub aws_email_sns_subscription_arn: Option<String>,
}

pub async fn start(mailer_rmq: Arc<MailerRabbitmq>) {
    let cfg = app_config();

    let state = AppState {
        mailer_rmq,
        aws_email_sns_subscription_arn: cfg.aws_sns_tracking_subscription_arn.clone(),
    };

    let healthcheck_router = Router::new().route("/healthcheck", get(healthcheck));

    let protected_router = Router::new()
        .route("/ses-events", post(handle_ses_event))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            check_aws_sns_arn_middleware,
        ));

    let app = Router::new()
        .merge(healthcheck_router)
        .merge(protected_router)
        .with_state(state);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0)), cfg.http_port);
    println!("[WEB] listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to get address {}", addr));

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to serve app on address {}", addr))
}
