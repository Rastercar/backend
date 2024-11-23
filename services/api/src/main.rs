mod config;
mod cronjobs;
mod database;
mod modules;
mod rabbitmq;
mod server;
mod services;
mod tracer;
mod utils;

use crate::{modules::tracking::cache::TrackerIdCache, services::s3::S3};
use config::app_config;
use sea_orm::DatabaseConnection;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::{sync::RwLock, task};

#[tokio::main]
pub async fn main() {
    let cfg = app_config();

    tracer::init("rastercar_api", cfg.is_development).expect("failed to init tracer");

    let db = database::db::connect(&cfg.db_url).await;

    modules::globals::TRACKER_ID_CACHE
        .get_or_init(|| Arc::new(RwLock::new(TrackerIdCache::new(db.clone()))));

    // # disable since were nuking the api
    // database::db::run_migrations(&db).await;

    cronjobs::start_clear_sessions_cronjob(db.clone(), Duration::from_secs(5 * 60));

    let rmq = Arc::new(rabbitmq::Rmq::new(&cfg.rmq_uri).await);
    let rmq_reconnect_ref = rmq.clone();
    let rmq_shutdown_ref = rmq.clone();

    task::spawn(async move {
        rmq_reconnect_ref.start_reconnection_task().await;
    });

    let db_conn_pool_shutdown_ref = db.clone();

    listen_to_shutdown_signals(
        !cfg.is_development,
        rmq_shutdown_ref,
        db_conn_pool_shutdown_ref,
    );

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] soon listening on {}", addr);

    let s3 = S3::new().await;

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to get address {}", addr));

    let server =
        server::controller::new(db, s3, rmq).into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, server)
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to serve app on address {}", addr));
}

/// Listen to shutdown signals `SIGINT` and `SIGTERM`, on a signal gracefully shutdowns down the application
#[allow(clippy::never_loop)]
fn listen_to_shutdown_signals(
    gracefully_shutdown: bool,
    rmq: Arc<rabbitmq::Rmq>,
    db: DatabaseConnection,
) {
    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("failed to setup signals hook");

    tokio::spawn(async move {
        for sig in signals.forever() {
            if gracefully_shutdown {
                println!("[APP] received signal: {}, shutting down", sig);

                println!("[APP] closing rabbitmq connections");
                rmq.shutdown().await;

                println!("[APP] closing postgres connections");
                if let Err(e) = db.close().await {
                    println!("[DB] failed to close db connection: {e}")
                }

                shared::tracer::shutdown().await;
            }

            std::process::exit(sig)
        }
    });
}
