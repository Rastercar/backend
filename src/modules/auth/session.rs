use crate::{config, modules::common::responses::SimpleError};
use axum::{async_trait, extract::FromRequestParts};
use cookie::{
    time::{self, OffsetDateTime},
    Cookie, SameSite,
};
use http::{request::Parts, HeaderMap, HeaderValue};
use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

pub const SESSION_ID_COOKIE_NAME: &str = "sid";
pub const SESSION_DAYS_DURATION: i64 = 5;

fn cookie_to_header_value(cookie: Cookie) -> HeaderValue {
    // unwrap here since a cookie constructed from the cookie crate should always
    // be converted to a valid cookie string and therefore a valid header value
    cookie.to_string().parse::<HeaderValue>().unwrap()
}

#[derive(Clone, Copy, Debug)]
pub struct SessionToken(u128);

impl SessionToken {
    /// converts the token into a session cookie
    fn into_cookie<'a>(self) -> Cookie<'a> {
        let mut cookie = Cookie::new(SESSION_ID_COOKIE_NAME, self.0.to_string());

        cookie.set_path("/");
        cookie.set_secure(!*config::ENV_DEVELOPMENT);
        cookie.set_same_site(SameSite::Strict);
        cookie.set_max_age(time::Duration::days(SESSION_DAYS_DURATION));

        cookie
    }

    pub fn get_id(&self) -> u128 {
        self.0
    }

    /// Creates a random session token from a random number generator
    pub fn generate_new(rng: &mut ChaCha8Rng) -> Self {
        let mut u128_pool = [0u8; 16];

        rng.fill_bytes(&mut u128_pool);

        Self(u128::from_le_bytes(u128_pool))
    }

    /// converts the token into a session cookie and parses it into a header value to be sent as a "Set-Cookie" header
    ///
    /// reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
    pub fn into_set_cookie_header(self) -> HeaderValue {
        cookie_to_header_value(self.into_cookie())
    }

    /// converts the token into a session cookie and parses it into a header value to be sent as a "Set-Cookie" header
    /// with a already expired date, this will cause the client browser to delete the cookie and thus end the session
    /// on the client side
    pub fn into_delete_cookie_header(self) -> HeaderValue {
        let mut cookie = self.into_cookie();

        cookie.set_max_age(None);
        cookie.set_expires(OffsetDateTime::now_utc());

        cookie_to_header_value(cookie)
    }

    /// Converts the session id into a vec of bytes to be stored as binary
    pub fn into_database_value(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

pub fn get_session_id_from_request_headers(headers: &mut HeaderMap) -> Option<u128> {
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

impl From<u128> for SessionToken {
    fn from(v: u128) -> Self {
        SessionToken(v)
    }
}

/// Simple struct to extract the session token from the request cookies into a `Option<SessionToken>`,
/// useful for endpoints where you might handle requests with or without sessions
pub struct OptionalSessionToken(Option<SessionToken>);

impl OptionalSessionToken {
    pub fn get_value(&self) -> Option<SessionToken> {
        self.0
    }
}

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
