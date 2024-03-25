//! DTOS for all events and operation inputs accepted by the mailer service

use super::validation::{email_vec, rfc_5322_email};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid;
use validator::Validate;

#[derive(Debug, Validate, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmailRecipient {
    /// recipient email address
    #[validate(email)]
    pub email: String,

    /// An array of email addresses to send the email to and the
    /// replacements to use on the email html for that email address, eg:
    ///
    /// ```
    /// { email: "jhon@gmail.com", replacements: { "name": "jhon" } }
    /// ```
    pub replacements: Option<HashMap<String, String>>,
}

impl EmailRecipient {
    pub fn has_replacements(&self) -> bool {
        match &self.replacements {
            Some(replacements) => !replacements.is_empty(),
            None => false,
        }
    }
}

#[derive(Debug, Default, Validate, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailIn {
    /// A unique identifier for the email sending request, this is so the client can store this on
    /// his side and use this identifier on future requests, such as getting metrics for this uuid
    pub uuid: Option<uuid::Uuid>,

    /// The RFC5322 email address to be used to send the email, if None the service default address is used
    #[validate(custom = "rfc_5322_email")]
    pub sender: Option<String>,

    /// List of recipients for the email
    #[validate]
    #[validate(length(min = 1))]
    pub to: Vec<EmailRecipient>,

    /// List of email addresses to show on the email reply-to options, only makes
    /// sense if at least one email address different than the sender is used
    #[validate(custom = "email_vec")]
    pub reply_to_addresses: Option<Vec<String>>,

    pub subject: String,

    pub body_html: Option<String>,

    /// Optional email text content: displayed on clients that do not support Html
    pub body_text: Option<String>,

    /// If tracking for email events such as clicks and opens should be enabled
    #[serde(default)]
    pub enable_tracking: bool,
}

impl SendEmailIn {
    pub fn with_body_html(mut self, html: &str) -> SendEmailIn {
        self.body_html = Some(String::from(html));
        self
    }

    pub fn with_to(mut self, recipients: Vec<EmailRecipient>) -> SendEmailIn {
        self.to = recipients;
        self
    }

    pub fn with_subject(mut self, subject: &str) -> SendEmailIn {
        self.subject = String::from(subject);
        self
    }
}
