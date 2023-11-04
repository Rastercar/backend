use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches:
    /// - mercosul vehicle plates (format: AAA-9A99)
    /// - brazilian vehicle plates (format: AAA-9999)
    pub static ref REGEX_IS_MERCOSUL_OR_BR_VEHICLE_PLATE: Regex =
        Regex::new(r"[a-z]{3}[0-9][a-z0-9][0-9]{2}").unwrap();
    //
    pub static ref REGEX_CONTAINS_NUMBER: Regex = Regex::new(r"[0-9]").unwrap();
    //
    pub static ref REGEX_CONTAINS_UPPERCASE_CHARACTER: Regex = Regex::new(r"[A-Z]").unwrap();
    //
    pub static ref REGEX_CONTAINS_LOWERCASE_CHARACTER: Regex = Regex::new(r"[a-z]").unwrap();
    //
    pub static ref REGEX_CONTAINS_SYMBOLIC_CHARACTER: Regex = Regex::new(r"[#?!@$%^&*-]").unwrap();
    //
    pub static ref REGEX_IS_LOWERCASE_ALPHANUMERIC_WITH_UNDERSCORES: Regex =
        Regex::new(r"^[a-z0-9_]+$").unwrap();
}
