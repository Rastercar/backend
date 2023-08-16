use super::dto::RegisterUserDto;
use crate::http::controller::AppState;
use axum::{
    extract::{Json, State},
    routing::post,
    Router,
};
use validator::Validate;

pub fn create_organization_router() -> Router<AppState> {
    Router::new().route("/", post(create_organization))
}

// TODO: FINISH ME !
async fn create_organization(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUserDto>,
) -> Result<String, String> {
    payload.validate().map_err(|e| e.to_string())?;

    println!("{:#?}", payload);

    state
        .organization_repository
        .create_organization()
        .await
        .or(Err("!!!"))?;

    Ok(String::from("ok!"))
}
