use crate::modules::organization::{
    repository::{new_organization_repository, OrganizationRepository},
    routes::create_organization_router,
};
use axum::{routing::get, Router};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

#[derive(Clone)]
pub struct AppState {
    pub db_conn_pool: Pool<AsyncPgConnection>,
    pub organization_repository: OrganizationRepository,
}

pub fn create_axum_app(db_conn_pool: Pool<AsyncPgConnection>) -> Router {
    let state = AppState {
        db_conn_pool: db_conn_pool.clone(),
        organization_repository: new_organization_repository(db_conn_pool),
    };

    Router::new()
        .route("/healthcheck", get(|| async { "ok" }))
        .nest("/organization", create_organization_router())
        .with_state(state)
}
