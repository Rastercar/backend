use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref REGEX_CONTAINS_NUMBER: Regex = Regex::new(r"[0-9]").unwrap();
    pub static ref REGEX_CONTAINS_UPPERCASE_CHARACTER: Regex = Regex::new(r"[A-Z]").unwrap();
    pub static ref REGEX_CONTAINS_LOWERCASE_CHARACTER: Regex = Regex::new(r"[a-z]").unwrap();
    pub static ref REGEX_CONTAINS_SYMBOLIC_CHARACTER: Regex = Regex::new(r"[#?!@$%^&*-]").unwrap();
    pub static ref REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES: Regex =
        Regex::new(r"^[a-z0-9_]+$").unwrap();
}
