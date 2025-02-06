use axum::{http::StatusCode, routing::get, Router};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub async fn start_server(port: u16) {
    let app = Router::new().route("/healthcheck", get(healthcheck));

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    println!("[WEB] listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to get address {}", addr));

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap_or_else(|_| panic!("[WEB] failed to serve app on address {}", addr))
}

/// just returns a ok response to say the service is healthy
async fn healthcheck() -> (StatusCode, String) {
    (StatusCode::OK, String::from("ok"))
}
