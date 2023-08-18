use crate::modules::{
    auth::routes::create_auth_router,
    auth::service::{new_auth_service, AuthService},
};
use axum::{routing::get, Router};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

#[derive(Clone)]
pub struct AppState {
    pub db_conn_pool: Pool<AsyncPgConnection>,
    pub auth_service: AuthService,
}

pub fn create_axum_app(db_conn_pool: Pool<AsyncPgConnection>) -> Router {
    let state = AppState {
        db_conn_pool: db_conn_pool.clone(),
        auth_service: new_auth_service(db_conn_pool.clone()),
    };

    Router::new()
        .route("/healthcheck", get(|| async { "ok" }))
        .nest("/auth", create_auth_router())
        .with_state(state)
}
