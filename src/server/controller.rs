use super::open_api;
use crate::{
    config::app_config,
    modules::{
        auth::routes::create_auth_router,
        auth::{
            middleware::RequestUser,
            service::{new_auth_service, AuthService},
        },
        common::responses::SimpleError,
    },
    services::{
        mailer::service::MailerService,
        s3::{S3Key, S3},
    },
};
use axum::{extract::State, routing::get, Extension, Router};
use axum_client_ip::SecureClientIpSource;
use deadpool_lapin::Pool as RmqPool;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use http::{header, HeaderValue, Method, StatusCode};
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub s3: S3,
    pub auth_service: AuthService,
    pub mailer_service: MailerService,
    pub db_conn_pool: Pool<AsyncPgConnection>,
}

impl AppState {
    pub async fn get_db_conn(
        &self,
    ) -> Result<
        deadpool::managed::Object<AsyncDieselConnectionManager<AsyncPgConnection>>,
        (StatusCode, SimpleError),
    > {
        Ok(self.db_conn_pool.get().await.or(Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            SimpleError::internal(),
        )))?)
    }
}

/// Creates the main axum router/controller to be served over https
pub fn new(db_conn_pool: Pool<AsyncPgConnection>, rmq_conn_pool: RmqPool, s3: S3) -> Router {
    let rng = ChaCha8Rng::seed_from_u64(OsRng.next_u64());

    let state = AppState {
        s3,
        db_conn_pool: db_conn_pool.clone(),
        mailer_service: MailerService::new(rmq_conn_pool),
        auth_service: new_auth_service(db_conn_pool.clone(), rng),
    };

    let frontend_origin = app_config()
        .frontend_url
        .to_string()
        .parse::<HeaderValue>()
        .expect("failed to parse CORS allowed origins");

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::PUT,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_origin(frontend_origin)
        .allow_credentials(true)
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE]);

    Router::new()
        // TODO: remove mock route
        .route("/upload", post(upload))
        .route("/healthcheck", get(healthcheck))
        .merge(open_api::create_openapi_router())
        .nest("/auth", create_auth_router(state.clone()))
        .layer(SecureClientIpSource::ConnectInfo.into_extension())
        .layer(cors)
        .with_state(state)
}

#[utoipa::path(
    get,
    tag = "meta",
    path = "/healthcheck",
    responses((status = OK)),
)]
pub async fn healthcheck() -> StatusCode {
    StatusCode::OK
}

use axum::{extract::Multipart, routing::post};

async fn upload(
    State(state): State<AppState>,
    req_user: Extension<RequestUser>,
    mut multipart: Multipart,
) -> Result<String, (StatusCode, SimpleError)> {
    // TODO: it would be very cool to have a field extractor or a typed extractor, also remove unwrap
    while let Some(file) = multipart.next_field().await.unwrap() {
        // this is the name which is sent in form data from frontend or whoever called the api, i am
        // using it as category, we can get the filename from file data
        let category = file.name().unwrap().to_string();

        // name of the file with extension
        let filename = file.file_name().unwrap().to_string();

        // TODO: accept png, jpeg, webp
        let content_type = file.content_type().unwrap().to_string();

        // file data
        let data = file.bytes().await.unwrap();

        println!(
            "Length is {} bytes of category: {}, type: {}",
            data.len(),
            category,
            content_type
        );

        let user = req_user.0 .0;

        let folder = match user.organization {
            Some(org) => format!("organization/{}/user/{}", org.id, user.id),
            None => format!("user/{}", user.id),
        };

        let key = S3Key { folder, filename };

        return state
            .s3
            .upload(key, data)
            .await
            .map(|_| String::from("profile pic changed successfully"))
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    SimpleError::from("failed to upload new profile picture"),
                )
            });

        // TODO: update profile picture on the DB!
    }

    Ok(String::from("xd/??"))
}
