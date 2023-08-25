mod config;
mod database;
mod modules;
mod scheduled;
mod server;

use config::AppConfig;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    let cfg = AppConfig::from_env();

    println!("[DB] running migrations");
    database::db::run_migrations(&cfg.db_url);

    let db_connection_pool = database::db::get_connection_pool(&cfg.db_url).await;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] listening on {}", addr);

    scheduled::cronjobs::start_clear_sessions_cronjob(db_connection_pool.clone());

    axum::Server::bind(&addr)
        .serve(
            server::controller::new(db_connection_pool)
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
}
