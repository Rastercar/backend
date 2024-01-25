use crate::config::app_config;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    // Audience
    pub aud: String,
    // Issued at (as UTC timestamp)
    pub iat: usize,
    // Issuer
    pub iss: String,
    // Subject (whom token refers to)
    pub sub: String,
    // Expiration time (as UTC timestamp, validate_exp defaults to true in validation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<usize>,
}

impl Default for Claims {
    fn default() -> Claims {
        let now = Utc::now();

        Claims {
            // [PROD-TODO] set this as the rastercar url
            aud: String::from("rastercar users"),
            iat: now.timestamp() as usize,
            iss: String::from("rastercar API"),
            sub: String::from("rastercar API token"),
            exp: None,
        }
    }
}

impl Claims {
    /// sets the claims `iat` (issued at) to the current time, and the `exp` to now + duration
    pub fn set_expiration_in(&mut self, duration: Duration) -> &Self {
        let now = Utc::now();

        self.exp = Some((now + duration).timestamp() as usize);
        self.iat = now.timestamp() as usize;

        self
    }
}

pub fn encode(claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(app_config().jwt_secret.as_ref()),
    )
}

pub fn decode(jwt: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        jwt,
        &DecodingKey::from_secret(app_config().jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
}
