use cookie::{time, Cookie};
use http::HeaderValue;
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
    pub fn into_set_cookie_header(self) -> Result<HeaderValue, http::header::InvalidHeaderValue> {
        let cookie = self.into_cookie();
        cookie.to_string().parse::<HeaderValue>()
    }

    pub fn into_database_value(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}
