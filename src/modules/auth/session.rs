use crate::{config::app_config, modules::common::responses::SimpleError};
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

/// a u128 that identifies a user session stored on the `sessions` database table
#[derive(Clone, Copy, Debug)]
pub struct SessionId(u128);

impl SessionId {
    pub fn get_id(&self) -> u128 {
        self.0
    }

    /// Creates a random session token from a random number generator
    pub fn generate_new(rng: &mut ChaCha8Rng) -> Self {
        let mut u128_pool = [0u8; 16];

        rng.fill_bytes(&mut u128_pool);

        Self(u128::from_le_bytes(u128_pool))
    }

    /// Creates a session id from a database value created by `into_database_value`
    ///
    /// returns `None` on error
    pub fn from_database_value(bytes: Vec<u8>) -> Option<Self> {
        if let Some(ipv6) = <[u8; 16]>::try_from(bytes.as_slice()).ok() {
            return Some(SessionId(u128::from_le_bytes(ipv6)));
        }

        None
    }

    fn cookie_to_header_value(self, cookie: Cookie) -> HeaderValue {
        // unwrap here since a cookie constructed from the cookie crate should always
        // be converted to a valid cookie string and therefore a valid header value
        cookie.to_string().parse::<HeaderValue>().unwrap()
    }

    /// converts the token into a session cookie
    fn into_cookie<'a>(self) -> Cookie<'a> {
        let mut cookie = Cookie::new(SESSION_ID_COOKIE_NAME, self.0.to_string());

        cookie.set_path("/");
        cookie.set_secure(!app_config().is_development);
        cookie.set_same_site(SameSite::Strict);
        cookie.set_max_age(time::Duration::days(SESSION_DAYS_DURATION));

        cookie
    }

    /// Converts the session id into a vec of bytes to be stored as binary
    pub fn into_database_value(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }

    /// converts the token into a session cookie and parses it into a header value to be sent as a "Set-Cookie" header
    ///
    /// reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
    pub fn into_set_cookie_header(self) -> HeaderValue {
        self.cookie_to_header_value(self.into_cookie())
    }

    /// converts the token into a session cookie and parses it into a header value to be sent as a "Set-Cookie" header
    /// with a already expired date, this will cause the client browser to delete the cookie and thus end the session
    /// on the client side
    pub fn into_delete_cookie_header(self) -> HeaderValue {
        let mut cookie = self.into_cookie();

        cookie.set_max_age(None);
        cookie.set_expires(OffsetDateTime::now_utc());

        self.cookie_to_header_value(cookie)
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
        .and_then(|sid_cookie| sid_cookie.parse::<u128>().ok())
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionId
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
            Some(session_id) => Ok(SessionId(session_id)),
        }
    }
}

impl From<u128> for SessionId {
    fn from(v: u128) -> Self {
        SessionId(v)
    }
}

/// Simple struct to extract the session token from the request cookies into a `Option<SessionId>`,
/// useful for endpoints where you might handle requests with or without sessions
pub struct OptionalSessionId(Option<SessionId>);

impl OptionalSessionId {
    pub fn get_value(&self) -> Option<SessionId> {
        self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalSessionId
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, SimpleError);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe_session_id = get_session_id_from_request_headers(&mut parts.headers);

        match maybe_session_id {
            None => Ok(OptionalSessionId(None)),
            Some(session_id) => Ok(OptionalSessionId(Some(SessionId(session_id)))),
        }
    }
}
