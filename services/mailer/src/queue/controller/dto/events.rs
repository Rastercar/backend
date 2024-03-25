//! DTOS for all the events that are fired by this service

use super::ses;
use crate::queue::Routable;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::dto::mailer::SendEmailIn;
use uuid::Uuid;

#[derive(strum_macros::Display, Deserialize, Serialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum EmailRequestStatus {
    STARTED,
    REJECTED,
}

/// Root object of the AWS SES email event JSON, see:
///
/// https://docs.aws.amazon.com/ses/latest/dg/event-publishing-retrieving-sns-contents.html#event-publishing-retrieving-sns-contents-top-level-json-object
#[allow(non_camel_case_types)]
#[derive(Deserialize, Serialize)]
pub enum Email {
    open(ses::OpenObj),
    send(ses::SendObj),
    click(ses::ClickObj),
    bounce(ses::BounceObj),
    reject(ses::RejectObj),
    failure(ses::FailureObj),
    delivery(ses::DeliveryObj),
    complaint(ses::ComplaintObj),
    subscription(ses::SubscriptionObj),
    delivery_delay(ses::DeliveryDelayObj),
}

/// A event to signify the status of a email sending request.
///
/// note that a email sending 'request' might have any number of
/// recipients, and dispatch multiple emails.
///
/// - `STARTED` the emails will be sent soon
/// - `REJECTED` the emails wont be sent as the request is invalid
#[derive(Deserialize, Serialize)]
pub struct EmailSendingReceivedEvent {
    pub timestamp: DateTime<Utc>,

    pub status: EmailRequestStatus,

    pub request_uuid: Uuid,

    pub request: SendEmailIn,
}

impl EmailSendingReceivedEvent {
    pub fn started(request_uuid: uuid::Uuid, request: SendEmailIn) -> EmailSendingReceivedEvent {
        EmailSendingReceivedEvent {
            request,
            request_uuid,
            timestamp: Utc::now(),
            status: EmailRequestStatus::STARTED,
        }
    }

    pub fn rejected(request_uuid: uuid::Uuid, request: SendEmailIn) -> EmailSendingReceivedEvent {
        EmailSendingReceivedEvent {
            request,
            request_uuid,
            timestamp: Utc::now(),
            status: EmailRequestStatus::REJECTED,
        }
    }
}

impl Routable for EmailSendingReceivedEvent {
    fn routing_key(&self) -> String {
        match self.status {
            EmailRequestStatus::STARTED => format!("sending.{}.started", self.request_uuid),
            EmailRequestStatus::REJECTED => format!("sending.{}.rejected", self.request_uuid),
        }
    }
}

/// informs that all the emails for a request have been fired to the AWS servers, this does not mean
/// the emails have all been successfully fired, much less that they reached the recipients inboxes
#[derive(Deserialize, Serialize)]
pub struct EmailRequestFinishedEvent {
    pub timestamp: DateTime<Utc>,

    pub request_uuid: uuid::Uuid,
}

impl Routable for EmailRequestFinishedEvent {
    fn routing_key(&self) -> String {
        format!("sending.{}.finished", self.request_uuid)
    }
}

impl EmailRequestFinishedEvent {
    pub fn new(request_uuid: uuid::Uuid) -> EmailRequestFinishedEvent {
        EmailRequestFinishedEvent {
            timestamp: Utc::now(),
            request_uuid,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct EmailSendingErrorEvent {
    pub timestamp: DateTime<Utc>,

    pub request_uuid: Uuid,

    pub error: String,

    pub recipients: Vec<String>,
}

impl EmailSendingErrorEvent {
    pub fn new(
        error: String,
        request_uuid: uuid::Uuid,
        recipients: Vec<String>,
    ) -> EmailSendingErrorEvent {
        EmailSendingErrorEvent {
            error,
            timestamp: Utc::now(),
            request_uuid,
            recipients,
        }
    }
}

impl Routable for EmailSendingErrorEvent {
    fn routing_key(&self) -> String {
        format!("sending.{}.error", self.request_uuid)
    }
}

#[derive(Deserialize, Serialize)]
pub struct EmailEvent {
    /// uuid of the mail request that generated this event, extracted from the `mail` field
    pub request_uuid: String,

    /// snake case version of possible values on:
    ///
    /// https://docs.aws.amazon.com/ses/latest/dg/event-publishing-retrieving-sns-contents.html#event-publishing-retrieving-sns-contents-top-level-json-object
    pub event_type: String,

    /// mail object from
    pub mail: ses::MailObj,

    /// original / raw SES event
    pub original: Email,
}

impl Routable for EmailEvent {
    fn routing_key(&self) -> String {
        format!("email.{}.{}", self.request_uuid, self.event_type)
    }
}
