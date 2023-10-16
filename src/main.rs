mod config;
mod database;
mod modules;
mod rabbitmq;
mod scheduled;
mod server;
mod services;

use crate::services::s3::S3;
use config::app_config;
use scheduled::cronjobs;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // see the project readme for more info on how tracing is configured
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cfg = app_config();

    info!("[DB] running migrations");
    database::db::run_migrations(&cfg.db_url);

    let db_conn_pool = database::db::get_connection_pool(&cfg.db_url).await;
    let rmq_conn_pool = rabbitmq::get_connection_pool(&cfg.rmq_uri);

    cronjobs::start_clear_sessions_cronjob(db_conn_pool.clone());

    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let db_conn_pool_shutdown_ref = db_conn_pool.clone();
    let rmq_conn_pool_shutdown_ref = rmq_conn_pool.clone();

    tokio::spawn(async move {
        for sig in signals.forever() {
            if !cfg.is_development {
                info!("[APP] received signal: {}, shutting down", sig);

                info!("[APP] closing rabbitmq connections");
                rmq_conn_pool_shutdown_ref.close();

                info!("[APP] closing postgres connections");
                db_conn_pool_shutdown_ref.close();
            }

            std::process::exit(sig)
        }
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    info!("[WEB] soon listening on {}", addr);

    let s3 = S3::new().await;

    let server = server::controller::new(db_conn_pool, rmq_conn_pool, s3)
        .into_make_service_with_connect_info::<SocketAddr>();

    axum::Server::bind(&addr).serve(server).await.unwrap();
}
