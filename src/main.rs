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

#[tokio::main]
async fn main() {
    let cfg = app_config();

    println!("[DB] running migrations");
    database::db::run_migrations(&cfg.db_url);

    let db_connection_pool = database::db::get_connection_pool(&cfg.db_url).await;
    let rmq_connection_pool = rabbitmq::get_connection_pool(&cfg.rmq_uri);

    let s3 = S3::new().await;

    cronjobs::start_clear_sessions_cronjob(db_connection_pool.clone());

    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let db_connection_pool_shutdown_ref = db_connection_pool.clone();
    let rmq_connection_pool_shutdown_ref = rmq_connection_pool.clone();

    tokio::spawn(async move {
        for sig in signals.forever() {
            if !cfg.is_development {
                println!("\n[APP] received signal: {}, shutting down", sig);

                println!("[APP] closing rabbitmq connections");
                rmq_connection_pool_shutdown_ref.close();

                println!("[APP] closing postgres connections");
                db_connection_pool_shutdown_ref.close();
            }

            std::process::exit(sig)
        }
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(
            server::controller::new(db_connection_pool, rmq_connection_pool, s3)
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
}
