use crate::modules::common::responses::SimpleError;
use axum::{async_trait, extract::FromRequestParts};
use cookie::{time, Cookie};
use http::{request::Parts, HeaderMap, HeaderValue};
use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

pub const SESSION_ID_COOKIE_NAME: &str = "sid";

#[derive(Clone, Copy, Debug)]
pub struct SessionToken(u128);

impl SessionToken {
    /// Creates a random session token from a random number generator
    pub fn generate_new(rng: &mut ChaCha8Rng) -> Self {
        let mut u128_pool = [0u8; 16];

        rng.fill_bytes(&mut u128_pool);

        Self(u128::from_le_bytes(u128_pool))
    }

    /// converts the token into a session cookie
    pub fn into_cookie<'a>(self) -> Cookie<'a> {
        let mut cookie = Cookie::new(SESSION_ID_COOKIE_NAME, self.0.to_string());

        cookie.set_path("/");
        cookie.set_max_age(time::Duration::days(1));

        cookie
    }

    /// converts the token into a session cookie and parses it into a header value to be sent as a "Set-Cookie" header
    ///
    /// reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
    pub fn into_set_cookie_header(self) -> HeaderValue {
        let cookie = self.into_cookie();

        // unwrap here since a cookie constructed from the cookie crate should always
        // be converted to a valid cookie string and therefore a valid header value
        cookie.to_string().parse::<HeaderValue>().unwrap()
    }

    pub fn into_database_value(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

fn get_session_id_from_request_headers(headers: &mut HeaderMap) -> Option<u128> {
    headers
        .get_all("Cookie")
        .iter()
        .filter_map(|cookie_header| {
            cookie_header
                .to_str()
                .ok()
                .and_then(|cookie_header| cookie_header.parse::<cookie::Cookie>().ok())
        })
        .find_map(|cookie| {
            (cookie.name() == SESSION_ID_COOKIE_NAME).then(move || cookie.value().to_owned())
        })
        .and_then(|cookie_str| cookie_str.parse::<u128>().ok())
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionToken
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe_session_id = get_session_id_from_request_headers(&mut parts.headers);

        match maybe_session_id {
            None => Err((
                http::StatusCode::UNAUTHORIZED,
                SimpleError::from("cannot find session cookie"),
            )),
            Some(session_id) => Ok(SessionToken(session_id)),
        }
    }
}

/// Simple struct to extract the session token into a `Option<SessionToken>`
pub struct OptionalSessionToken(pub Option<SessionToken>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalSessionToken
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe_session_id = get_session_id_from_request_headers(&mut parts.headers);

        match maybe_session_id {
            None => Ok(OptionalSessionToken(None)),
            Some(session_id) => Ok(OptionalSessionToken(Some(SessionToken(session_id)))),
        }
    }
}
