//! Structs containing the needed replacements for email templates

use std::collections::HashMap;

pub struct RecoverPasswordReplacements {
    pub username: String,
    pub reset_password_link: String,
}

impl From<RecoverPasswordReplacements> for HashMap<String, String> {
    fn from(val: RecoverPasswordReplacements) -> Self {
        HashMap::from([
            (String::from("username"), val.username),
            (String::from("resetPasswordLink"), val.reset_password_link),
        ])
    }
}

pub struct ConfirmEmailReplacements {
    pub title: String,
    pub confirmation_link: String,
}

impl From<ConfirmEmailReplacements> for HashMap<String, String> {
    fn from(val: ConfirmEmailReplacements) -> Self {
        HashMap::from([
            (String::from("title"), val.title),
            (String::from("confirmationLink"), val.confirmation_link),
        ])
    }
}
