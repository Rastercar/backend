use super::{heartbeat::HeartbeatMsg, location::LocationMsg, utils};
use crate::protocols::common::Decoded;
use std::str::{self, from_utf8};

mod msg_ids {
    pub const LOCATION: &str = "V1";
    pub const HEARTBEAT: &str = "HTBT";
}

/// All possible message types decodable from the H02 tracker protocol
pub enum Message {
    Heartbeat(Decoded<HeartbeatMsg>),
    Location(Decoded<LocationMsg>),
}

pub fn decode(packets: &[u8]) -> Result<Message, String> {
    let packets = from_utf8(packets)
        .or(Err("failed to read packets as utf8"))?
        .to_string();

    let message_frame = utils::get_message_frame(packets)?;

    let parts: Vec<&str> = message_frame.split(',').filter(|x| !x.is_empty()).collect();

    if parts.len() < 2 {
        return Err("cannot get message type to decode packets to".to_string());
    }

    let message_type = parts[1];

    match message_type {
        msg_ids::HEARTBEAT => Ok(Message::Heartbeat(parts.try_into()?)),
        msg_ids::LOCATION => Ok(Message::Location(parts.try_into()?)),
        _ => Err("unknown message type".to_string()),
    }
}
