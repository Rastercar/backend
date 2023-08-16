use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::Validate;

lazy_static! {
    static ref REGEX_CONTAINS_NUMBER: Regex = Regex::new(r"[0-9]").unwrap();
    static ref REGEX_CONTAINS_UPPERCASE_CHARACTER: Regex = Regex::new(r"[A-Z]").unwrap();
    static ref REGEX_CONTAINS_LOWERCASE_CHARACTER: Regex = Regex::new(r"[a-z]").unwrap();
    static ref REGEX_CONTAINS_SYMBOLIC_CHARACTER: Regex = Regex::new(r"[#?!@$%^&*-]").unwrap();
}

#[derive(Deserialize, Serialize, Validate, Debug)]
#[serde(rename_all = "snake_case")]
pub struct RegisterUserDto {
    #[validate(length(min = 5, max = 60))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 5, max = 60))]
    #[validate(regex(
        path = "REGEX_CONTAINS_NUMBER",
        message = "password must contain a number"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_SYMBOLIC_CHARACTER",
        message = "password must contain a symbol in: #?!@$%^&*-"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_UPPERCASE_CHARACTER",
        message = "password must contain a uppercase character"
    ))]
    #[validate(regex(
        path = "REGEX_CONTAINS_LOWERCASE_CHARACTER",
        message = "password must contain a lowercase character"
    ))]
    pub password: String,

    pub refers_to_unregistered_user: Option<String>,
}
