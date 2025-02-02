//! DTOS for AWS SNS and SES events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SnsNotification {
    #[serde(rename = "Type")]
    pub notification_type: String,

    #[serde(rename = "MessageId")]
    pub message_id: String,

    #[serde(rename = "TopicArn")]
    pub topic_arn: String,

    #[serde(rename = "Subject")]
    pub subject: Option<String>,

    /// JSON string of the ses event
    #[serde(rename = "Message")]
    pub message: String,

    #[serde(rename = "Timestamp")]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "SignatureVersion")]
    pub signature_version: String,

    #[serde(rename = "Signature")]
    pub signature: String,

    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,

    #[serde(rename = "SubscribeURL")]
    pub subscribe_url: Option<String>,

    #[serde(rename = "UnsubscribeURL")]
    pub unsubscribe_url: Option<String>,
}

/// spec: https://docs.aws.amazon.com/ses/latest/dg/event-publishing-retrieving-sns-contents.html#event-publishing-retrieving-sns-contents-top-level-json-object
///
/// examples: https://docs.aws.amazon.com/ses/latest/dg/event-publishing-retrieving-sns-examples.html#event-publishing-retrieving-sns-delivery
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesEvent {
    /// If you did not set up event publishing this field will be `None` and the value will be in `notification_type`
    pub event_type: Option<String>,

    /// If you did set up event publishing this field will be `None` and the value will be in `event_type`
    pub notification_type: Option<String>,

    pub mail: MailObj,

    pub bounce: Option<BounceObj>,

    pub complaint: Option<ComplaintObj>,

    pub delivery: Option<DeliveryObj>,

    pub reject: Option<RejectObj>,

    pub open: Option<OpenObj>,

    pub send: Option<SendObj>,

    pub click: Option<ClickObj>,

    pub failure: Option<FailureObj>,

    pub delivery_delay: Option<DeliveryDelayObj>,

    pub subscription: Option<SubscriptionObj>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailObj {
    pub timestamp: DateTime<Utc>,

    pub message_id: String,

    pub source_arn: Option<String>,

    pub sending_account_id: String,

    pub destination: Vec<String>,

    pub headers_truncated: bool,

    pub headers: Vec<Header>,

    pub common_headers: HashMap<String, serde_json::Value>,

    pub tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Header {
    pub name: String,

    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendObj {}

#[derive(Debug, Deserialize, Serialize)]
pub struct BounceObj {
    pub timestamp: DateTime<Utc>,

    #[serde(rename(serialize = "bounce_type", deserialize = "bounceType"))]
    pub bounce_type: String,

    #[serde(rename(serialize = "bounce_sub_type", deserialize = "bounceSubType"))]
    pub bounce_sub_type: String,

    #[serde(rename(serialize = "bounced_recipients", deserialize = "bouncedRecipients"))]
    pub bounced_recipients: Vec<BouncedRecipients>,

    #[serde(rename(serialize = "feedback_id", deserialize = "feedbackId"))]
    pub feedback_id: String,

    #[serde(rename(serialize = "reporting_mta", deserialize = "reportingMTA"))]
    pub reporting_mta: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BouncedRecipients {
    pub email_address: String,

    pub action: String,

    pub diagnostic_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplaintObj {
    pub complained_recipients: Vec<ComplainedRecipient>,

    pub timestamp: DateTime<Utc>,

    pub feedback_id: String,

    pub complaint_sub_type: String,

    pub user_agent: String,

    pub complaint_feedback_type: String,

    pub arrival_date: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplainedRecipient {
    pub email_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeliveryObj {
    pub timestamp: DateTime<Utc>,

    #[serde(rename(
        serialize = "processing_time_millis",
        deserialize = "processingTimeMillis"
    ))]
    pub processing_time_millis: i32,

    pub recipients: Vec<String>,

    #[serde(rename(serialize = "smtp_response", deserialize = "smtpResponse"))]
    pub smtp_response: String,

    #[serde(rename(serialize = "reporting_mta", deserialize = "reportingMTA"))]
    pub reporting_mta: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RejectObj {
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenObj {
    pub timestamp: DateTime<Utc>,

    pub ip_address: String,

    pub user_agent: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickObj {
    pub timestamp: DateTime<Utc>,

    pub ip_address: String,

    pub user_agent: String,

    pub link: String,

    pub link_tags: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureObj {
    pub template_name: String,

    pub error_message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDelayObj {
    pub delay_type: String,

    pub delayed_recipients: Vec<DelayedRecipient>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelayedRecipient {
    pub email_address: String,

    pub status: String,

    pub diagnostic_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionObj {
    pub contact_list: String,

    pub timestamp: DateTime<Utc>,

    pub source: String,

    pub new_topic_preferences: TopicPreference,

    pub old_topic_preferences: TopicPreference,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicPreference {
    pub unsubscribe_all: bool,

    pub topic_subscription_status: Vec<TopicSubscriptionStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicSubscriptionStatus {
    pub topic_name: String,

    pub subscription_status: String,
}
