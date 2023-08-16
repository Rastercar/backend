mod config;
mod database;
mod http;
mod modules;

use config::AppConfig;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    let cfg = AppConfig::from_env();

    database::db::run_migrations(&cfg.db_url);

    let db_connection_pool = database::db::get_connection_pool(&cfg.db_url).await;
    let app = http::controller::create_axum_app(db_connection_pool);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
