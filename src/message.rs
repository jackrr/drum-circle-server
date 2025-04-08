use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::drum_circle::{CircleId, UserId};

type SDP = String;

#[derive(Serialize, Deserialize, Debug)]
pub struct SDPOffer {
    user_id: UserId,
    sdp: SDP,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WSPayload {
    pub name: String,
    pub member_id: Option<UserId>,
    pub circle_id: Option<CircleId>,
    pub members: Option<Vec<String>>,
    pub sdps: Option<Vec<SDPOffer>>,
    pub sdp: Option<SDP>,
}

// pub type ParsedPayload = Result<WSPayload>;

pub fn deserialize(s: &str) -> WSPayload {
    match serde_json::from_str(s) {
        Err(e) => panic!("Failed to parse {:?}", e),
        Ok(payload) => payload,
    }
}

pub fn serialize(payload: WSPayload) -> Message {
    match serde_json::to_string(&payload) {
        Err(e) => panic!("Failed to serialize {:?}", e),
        Ok(json) => Message::text(json),
    }
}
