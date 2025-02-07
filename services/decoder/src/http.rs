use axum::{extract::Query, http::StatusCode, routing::get, Router};
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

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

pub async fn healthcheck(Query(params): Query<HashMap<String, String>>) -> (StatusCode, String) {
    if params.get("debug").map(|v| v == "true").unwrap_or(false) {
        let commit_sha = env::var("COMMIT_HASH").unwrap_or_else(|_| "unknown".to_string());

        return (StatusCode::OK, format!("OK, commit HASH: {}", commit_sha));
    }

    (StatusCode::OK, String::from("ok"))
}
