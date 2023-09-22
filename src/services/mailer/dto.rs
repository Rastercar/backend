use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmailRecipient {
    /// recipient email address
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
    pub fn from(email: &str) -> EmailRecipient {
        EmailRecipient {
            email: String::from(email),
            replacements: None,
        }
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailIn {
    /// A unique identifier for the email sending request, this is so the client can store this on
    /// his side and use this identifier on future requests, such as getting metrics for this uuid
    pub uuid: Option<uuid::Uuid>,

    /// The RFC5322 email address to be used to send the email, if None the service default address is used
    pub sender: Option<String>,

    /// List of recipients for the email
    pub to: Vec<EmailRecipient>,

    /// List of email addresses to show on the email reply-to options, only makes
    /// sense if at least one email address different than the sender is used
    pub reply_to_addresses: Option<Vec<String>>,

    /// Email subject
    pub subject: String,

    /// Email HTML content
    pub body_html: Option<String>,

    /// Optional email text content: displayed on clients that do not support Html
    pub body_text: Option<String>,

    /// If tracking for email events such as clicks and opens should be enabled
    pub enable_tracking: bool,
}

impl SendEmailIn {
    pub fn with_sender(mut self, sender: &str) -> SendEmailIn {
        self.sender = Some(String::from(sender));
        self
    }

    pub fn with_body_html(mut self, html: &str) -> SendEmailIn {
        self.body_html = Some(String::from(html));
        self
    }

    pub fn with_to_from_emails(mut self, recipients: Vec<&str>) -> SendEmailIn {
        self.to = recipients
            .into_iter()
            .map(|email_address| EmailRecipient::from(email_address))
            .collect();
        self
    }

    pub fn with_subject(mut self, subject: &str) -> SendEmailIn {
        self.subject = String::from(subject);
        self
    }
}
