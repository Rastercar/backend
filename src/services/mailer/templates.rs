//! Structs containing the needed replacements for email templates

use std::collections::HashMap;

pub struct RecoverPasswordReplacements {
    pub username: String,
    pub reset_password_link: String,
}

impl Into<HashMap<String, String>> for RecoverPasswordReplacements {
    fn into(self) -> HashMap<String, String> {
        HashMap::from([
            (String::from("username"), self.username),
            (String::from("resetPasswordLink"), self.reset_password_link),
        ])
    }
}

pub struct ConfirmEmailReplacements {
    pub title: String,
    pub confirmation_link: String,
}

impl Into<HashMap<String, String>> for ConfirmEmailReplacements {
    fn into(self) -> HashMap<String, String> {
        HashMap::from([
            (String::from("title"), self.title),
            (String::from("confirmationLink"), self.confirmation_link),
        ])
    }
}
