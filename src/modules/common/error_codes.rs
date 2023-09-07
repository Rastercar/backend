/// static error code for when a entity could not be created or updated
/// with a given email because its already in use by another entity
pub static EMAIL_IN_USE: &str = "EMAIL_IN_USE";

/// static error code for when a user could not be created or updated
/// with a given username because its already in use
pub static USERNAME_IN_USE: &str = "USERNAME_IN_USE";

/// a request to a endpoint was not authorized because it did not contain
/// the session id cookie in the request headers
pub static NO_SID_COOKIE: &str = "NO_SID_COOKIE";

/// a request to a endpoint was not authorized because
/// the session on the session id cookie is expired or does not exist
pub static INVALID_SESSION: &str = "INVALID_SESSION";
