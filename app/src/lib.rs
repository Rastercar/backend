mod config;
mod cronjobs;
mod database;
mod modules;
mod rabbitmq;
mod server;
mod services;
mod utils;

use crate::services::s3::S3;
use config::app_config;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
pub async fn main() {
    // see the project readme for more info on how tracing is configured
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .with_target(false)
        .init();

    let cfg = app_config();

    let db = database::db::create_db_conn(&cfg.db_url).await;

    database::db::run_migrations(&db).await;

    cronjobs::start_clear_sessions_cronjob(db.clone(), Duration::from_secs(5 * 60));

    let rmq_conn_pool = rabbitmq::create_connection_pool(&cfg.rmq_uri);

    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let db_conn_pool_shutdown_ref = db.clone();
    let rmq_conn_pool_shutdown_ref = rmq_conn_pool.clone();

    tokio::spawn(async move {
        for sig in signals.forever() {
            if !cfg.is_development {
                info!("[APP] received signal: {}, shutting down", sig);

                info!("[APP] closing rabbitmq connections");
                rmq_conn_pool_shutdown_ref.close();

                info!("[APP] closing postgres connections");

                if let Err(_) = db_conn_pool_shutdown_ref.close().await {
                    error!("[DB] failed to close db connection")
                }
            }

            std::process::exit(sig)
        }
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    info!("[WEB] soon listening on {}", addr);

    let s3 = S3::new().await;

    let server = server::controller::new(db, s3, rmq_conn_pool)
        .into_make_service_with_connect_info::<SocketAddr>();

    axum::Server::bind(&addr).serve(server).await.unwrap();
}
