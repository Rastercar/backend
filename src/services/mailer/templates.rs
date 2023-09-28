use std::collections::HashMap;

pub struct ResetPasswordReplacements {
    pub username: String,
    pub reset_password_link: String,
}

impl Into<HashMap<String, String>> for ResetPasswordReplacements {
    fn into(self) -> HashMap<String, String> {
        HashMap::from([
            (String::from("username"), self.username),
            (String::from("resetPasswordLink"), self.reset_password_link),
        ])
    }
}
