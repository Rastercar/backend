use crate::rabbitmq::RmqMessage;
use serde::Serialize;
use strum::Display;

/// all protocols at least partially supported by this service
#[derive(Display)]
#[strum(serialize_all = "snake_case")]
pub enum Protocol {
    H02,
}

#[derive(Display)]
#[strum(serialize_all = "snake_case")]
pub enum TrackerEvent {
    Location,
    Heartbeat,
}

/// The result of decoding a tracker packet.
pub struct Decoded<T: Serialize> {
    pub event_type: TrackerEvent,

    /// imei of the tracker who sent the packet
    pub imei: String,

    /// the tracker protocol decoded content
    pub data: T,

    /// bytes to send in response to the tracker
    pub response: Option<Box<[u8]>>,

    /// protocol used to decode the packet
    pub protocol: Protocol,
}

impl<T: Serialize> Decoded<T> {
    pub fn get_routing_key(&self) -> String {
        format!("{}.{}.{}", self.protocol, self.event_type, self.imei)
    }
}

impl<T: Serialize> TryFrom<Decoded<T>> for RmqMessage {
    type Error = String;

    fn try_from(v: Decoded<T>) -> Result<Self, Self::Error> {
        let body = serde_json::to_string(&v.data)
            .map_err(|e| format!("failed to parse event of type: {} - {}", v.event_type, e))?;

        let routing_key = v.get_routing_key();

        Ok(RmqMessage { body, routing_key })
    }
}
