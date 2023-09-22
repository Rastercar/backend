use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    // Optional. Audience
    pub aud: String,
    // Optional. Issued at (as UTC timestamp)
    pub iat: usize,
    // Optional. Issuer
    pub iss: String,
    // Optional. Subject (whom token refers to)
    pub sub: String,
    // TODO: check validation
    // Required. (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub exp: usize,
}
