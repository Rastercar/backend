use super::dto;
use crate::modules::common::{error_codes, responses::SimpleError};
use crate::server::controller::AppState;
use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use http::HeaderMap;
use validator::Validate;

pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/register-organization", post(register_organization))
        .route("/sign-in", post(sign_in))
}

// TODO: do not accept login requests from already logged in users by extracting request headers and denying
async fn sign_in(
    State(state): State<AppState>,
    Json(payload): Json<dto::SignIn>,
) -> Result<(http::StatusCode, HeaderMap, String), (StatusCode, SimpleError)> {
    // TODO: maybe i can extract the validation into a Union type such as Validated(Nig) where the
    // from request trait does the validation impl / middleware the auth middleware can be of help
    match payload.validate() {
        Ok(_) => {}
        Err(e) => return Err(((StatusCode::BAD_REQUEST), SimpleError::from(e))),
    }

    // TODO: discover how i can get the request cookies extracted by name
    // let session_token = req
    //     .headers()
    //     .get_all("Cookie")
    //     .iter()
    //     .filter_map(|cookie| {
    //         cookie
    //             .to_str()
    //             .ok()
    //             .and_then(|cookie| cookie.parse::<cookie::Cookie>().ok())
    //     })
    //     .find_map(|cookie| {
    //         (cookie.name() == USER_COOKIE_NAME).then(move || cookie.value().to_owned())
    //     })
    //     .and_then(|cookie_value| cookie_value.parse::<u128>().ok());

    let error_res = (
        StatusCode::INTERNAL_SERVER_ERROR,
        SimpleError::from("failed to create session"),
    );

    // TODO: get user ID from request

    // TODO: part of this fn might be used to login user after he signs up
    let session_token = state
        .auth_service
        .new_session(state.db_conn_pool, 1)
        .await
        .or(Err(error_res.clone()))?;

    let mut headers = HeaderMap::new();

    match session_token.into_set_cookie_header() {
        Ok(cookie) => {
            headers.insert("Set-Cookie", cookie);
            return Ok((http::StatusCode::OK, headers, String::from("login ok")));
        }
        Err(_) => Err(error_res),
    }
}

async fn register_organization(
    State(state): State<AppState>,
    Json(payload): Json<dto::RegisterOrganization>,
) -> Result<impl IntoResponse, (StatusCode, SimpleError)> {
    match payload.validate() {
        Ok(_) => {}
        Err(e) => return Err(((StatusCode::BAD_REQUEST), SimpleError::from(e))),
    }

    let internal_server_error_response =
        (StatusCode::INTERNAL_SERVER_ERROR, SimpleError::internal());

    let email_in_use = state
        .auth_service
        .check_email_in_use(payload.email.clone())
        .await
        .or(Err(internal_server_error_response.clone()))?;

    if email_in_use {
        return Err((
            StatusCode::BAD_REQUEST,
            SimpleError::from(error_codes::EMAIL_IN_USE),
        ));
    }

    let created_user = state
        .auth_service
        .register_user_and_organization(payload)
        .await
        .or(Err(internal_server_error_response.clone()))?;

    state
        .auth_service
        .login_for_user(created_user, true)
        .await
        .or(Err(internal_server_error_response))?;

    // TODO: !
    Ok(String::from("ok!"))
}

// TODO:
// #[derive(Clone)]
// pub(crate) struct User {
//     pub username: String,
// }

// #[derive(Clone)]
// pub(crate) struct AuthState(Option<(SessionToken, Option<User>, Pool<AsyncPgConnection>)>);

// impl AuthState {
// pub fn logged_in(&self) -> bool {
//     self.0.is_some()
// }

// pub async fn get_user(&mut self) -> Option<&User> {
//     let (session_token, store, database) = self.0.as_mut()?;

//     if store.is_none() {

//         // const QUERY: &str =
//         //     "SELECT id, username FROM users JOIN sessions ON user_id = id WHERE session_token = $1;";

//         // let user: Option<(i32, String)> = sqlx::query_as(QUERY)
//         //     .bind(&session_token.into_database_value())
//         //     .fetch_optional(&*database)
//         //     .await
//         //     .unwrap();

//         // if let Some((_id, username)) = user {
//         //     *store = Some(User { username });
//         // }
//     }

//     store.as_ref()
// }
// }

// pub(crate) async fn auth<B>(
//     mut req: http::Request<B>,
//     next: axum::middleware::Next<B>,
//     database: Pool<AsyncPgConnection>,
// ) -> axum::response::Response {
//     let session_token = req
//         .headers()
//         .get_all("Cookie")
//         .iter()
//         .filter_map(|cookie| {
//             cookie
//                 .to_str()
//                 .ok()
//                 .and_then(|cookie| cookie.parse::<cookie::Cookie>().ok())
//         })
//         .find_map(|cookie| {
//             (cookie.name() == USER_COOKIE_NAME).then(move || cookie.value().to_owned())
//         })
//         .and_then(|cookie_value| cookie_value.parse::<SessionToken>().ok());

//     req.extensions_mut()
//         .insert(AuthState(session_token.map(|v| (v, None, database))));

//     next.run(req).await
// }

// pub(crate) async fn signup(
//     database: Pool<AsyncPgConnection>,
//     random: Random,
//     username: &str,
//     password: &str,
// ) -> Result<SessionToken, SignupError> {
//     fn valid_username(username: &str) -> bool {
//         (1..20).contains(&username.len())
//             && username
//                 .chars()
//                 .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
//     }

//     if !valid_username(username) {
//         return Err(SignupError::InvalidUsername);
//     }

//     const INSERT_QUERY: &str =
//         "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING id;";

//     let salt = SaltString::generate(&mut OsRng);

//     // Hash password to PHC string ($pbkdf2-sha256$...)
//     let password_hash = Pbkdf2.hash_password(password.as_bytes(), &salt);

//     let hashed_password = if let Ok(password) = password_hash {
//         password.to_string()
//     } else {
//         return Err(SignupError::InvalidPassword);
//     };

//     let fetch_one = sqlx::query_as(INSERT_QUERY)
//         .bind(username)
//         .bind(hashed_password)
//         .fetch_one(database)
//         .await;

//     let user_id: i32 = match fetch_one {
//         Ok((user_id,)) => user_id,
//         Err(sqlx::Error::Database(database))
//             if database.constraint() == Some("users_username_key") =>
//         {
//             return Err(SignupError::UsernameExists);
//         }
//         Err(_err) => {
//             return Err(SignupError::InternalError);
//         }
//     };

//     Ok(new_session(database, random, user_id).await)
// }

// pub(crate) async fn login(
//     database: &Database,
//     random: Random,
//     username: &str,
//     password: &str,
// ) -> Result<SessionToken, LoginError> {
//     const LOGIN_QUERY: &str = "SELECT id, password FROM users WHERE users.username = $1;";

//     let row: Option<(i32, String)> = sqlx::query_as(LOGIN_QUERY)
//         .bind(username)
//         .fetch_optional(database)
//         .await
//         .unwrap();

//     let (user_id, hashed_password) = if let Some(row) = row {
//         row
//     } else {
//         return Err(LoginError::UserDoesNotExist);
//     };

//     // Verify password against PHC string
//     let parsed_hash = PasswordHash::new(&hashed_password).unwrap();
//     if let Err(_err) = Pbkdf2.verify_password(password.as_bytes(), &parsed_hash) {
//         return Err(LoginError::WrongPassword);
//     }

//     Ok(new_session(database, random, user_id).await)
// }

// pub(crate) async fn delete_user(auth_state: AuthState) {
//     const DELETE_QUERY: &str = "DELETE FROM users
//         WHERE users.id = (
//             SELECT user_id FROM sessions WHERE sessions.session_token = $1
//         );";

//     let auth_state = auth_state.0.unwrap();
//     let _res = sqlx::query(DELETE_QUERY)
//         .bind(&auth_state.0.into_database_value())
//         .execute(&auth_state.2)
//         .await
//         .unwrap();
// }
