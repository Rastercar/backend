use serde::Serialize;

use crate::protocols::common::{Decoded, Protocol, TrackerEvent};

#[derive(Serialize, Debug)]
pub struct HeartbeatMsg {
    /// 15 digit imei number
    pub imei: String,
}

impl TryFrom<Vec<&str>> for Decoded<HeartbeatMsg> {
    type Error = String;

    fn try_from(parts: Vec<&str>) -> Result<Self, Self::Error> {
        if parts.is_empty() {
            return Err("incomplete heartbeat message".to_string());
        }

        let imei = parts[0].to_string();

        Ok(Decoded {
            data: HeartbeatMsg { imei: imei.clone() },
            imei,
            response: None,
            protocol: Protocol::H02,
            event_type: TrackerEvent::Heartbeat,
        })
    }
}
