mod config;
mod cronjobs;
mod database;
mod modules;
mod rabbitmq;
mod server;
mod services;
mod tracer;
mod utils;

use crate::services::s3::S3;
use config::app_config;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::task;

#[tokio::main]
#[allow(clippy::never_loop)]
pub async fn main() {
    tracer::init("rastercar_api").expect("failed to init tracer");

    // TODO:
    // see the project readme for more info on how tracing is configured
    // tracing_subscriber::fmt()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .with_test_writer()
    //     .with_target(false)
    //     .init();

    let cfg = app_config();

    let db = database::db::connect(&cfg.db_url).await;

    database::db::run_migrations(&db).await;

    cronjobs::start_clear_sessions_cronjob(db.clone(), Duration::from_secs(5 * 60));

    let rmq = Arc::new(rabbitmq::Rmq::new(&cfg.rmq_uri).await);
    let rmq_reconnect_ref = rmq.clone();
    let rmq_shutdown_ref = rmq.clone();

    task::spawn(async move {
        rmq_reconnect_ref.start_reconnection_task().await;
    });

    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let db_conn_pool_shutdown_ref = db.clone();

    tokio::spawn(async move {
        for sig in signals.forever() {
            if !cfg.is_development {
                println!("[APP] received signal: {}, shutting down", sig);

                println!("[APP] closing rabbitmq connections");
                rmq_shutdown_ref.shutdown().await;

                println!("[APP] closing postgres connections");
                if let Err(e) = db_conn_pool_shutdown_ref.close().await {
                    println!("[DB] failed to close db connection: {e}")
                }

                tracer::shutdown().await;
            }

            std::process::exit(sig)
        }
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] soon listening on {}", addr);

    let s3 = S3::new().await;

    let server =
        server::controller::new(db, s3, rmq).into_make_service_with_connect_info::<SocketAddr>();

    axum::Server::bind(&addr).serve(server).await.unwrap();
}
