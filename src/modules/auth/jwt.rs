use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

use crate::config::app_config;

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
    pub exp: usize,
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
