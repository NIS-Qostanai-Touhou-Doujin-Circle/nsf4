use actix::prelude::*;
use serde::{Deserialize, Serialize};

// WebRTC message types
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum WebRtcMessage {
    Offer {
        sdp: String,
        from_user_id: String,
        to_user_id: String,
    },
    Answer {
        sdp: String,
        from_user_id: String,
        to_user_id: String,
    },
    IceCandidate {
        candidate: String,
        from_user_id: String,
        to_user_id: String,
    },
}

// WebSocket message types
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event")]
pub enum WsMessage {
    // System messages
    Connect {
        user_id: String, 
        display_name: String
    },
    Disconnect {
        user_id: String
    },
    JoinRequest {
        user_id: String, 
        display_name: String
    },
    JoinApproved {
        user_id: String
    },
    JoinDenied {
        user_id: String
    },
    
    // New message types for WebSocket approval/denial
    ApproveJoinRequest {
        user_id: String,     // The user being approved
    },
    DenyJoinRequest {
        user_id: String,     // The user being denied
    },
    
    // Media status
    MediaStatus {
        user_id: String,
        camera_on: bool,
        mic_on: bool
    },
    
    // WebRTC signaling
    WebRTC {
        message: WebRtcMessage
    },
    
    // Error message
    Error {
        message: String
    },
    
    // Ping-pong for connection check
    Ping,
    Pong,
}

// Implement Message trait for WsMessage
impl Message for WsMessage {
    type Result = ();
}

// WebSocket connection registration
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub room_id: String,
    pub user_id: String,
    pub addr: Addr<crate::ws::connection::WsConnection>,
}

// Send message to WebSocket clients
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendMessage {
    pub room_id: String,
    // If target_user_id = None, message is sent to all users in room
    pub sender_id: String, // Added sender_id
    pub target_user_id: Option<String>,
    pub message: WsMessage,
}

// WebSocket disconnection
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub room_id: String,
    pub user_id: String,
}

pub struct SetAppState {
    pub app_state: actix_web::web::Data<crate::models::AppState>,
}

impl actix::Message for SetAppState {
    type Result = ();
}