use actix::Addr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::ws::server::WsServer;

// App State
pub struct AppState {
    pub rooms: Mutex<HashMap<String, Room>>,
    pub ws_server: Addr<WsServer>,
}

// User model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: String,
    pub display_name: String,
}

impl User {
    pub fn copy(&self) -> Self {
        Self {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
        }
    }
}

// Participant in a room
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Participant {
    pub user: User,
    pub camera_on: bool,
    pub mic_on: bool,
    pub connected: bool,
}

impl Participant {
    pub fn copy(&self) -> Self {
        Self {
            user: self.user.copy(),
            camera_on: self.camera_on,
            mic_on: self.mic_on,
            connected: self.connected,
        }
    }
}


// Room model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    pub id: String,
    pub creator_id: String,
    pub participants: HashMap<String, Participant>,
    pub pending_requests: HashMap<String, User>,
}

impl Room {
    pub fn copy(&self) -> Self {
        Self {
            id: self.id.clone(),
            creator_id: self.creator_id.clone(),
            participants: self.participants.iter()
                .map(|(k, v)| (k.clone(), v.copy()))
                .collect(),
            pending_requests: self.pending_requests.iter()
                .map(|(k, v)| (k.clone(), v.copy()))
                .collect(),
        }
    }
}

// API Request/Response structures

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub room_id: String,
    pub creator_id: String,
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub id: String,
    pub creator_id: String,
    pub participants: Vec<Participant>,
    pub pending_requests: Vec<User>,
}

#[derive(Serialize)]
pub struct GeneralMessageResponse {
    pub message: String,
    pub room_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
    pub user_id: String,
    pub display_name: String,
}

// #[derive(Deserialize)]
// pub struct ApproveOrDenyRequest {
//     pub room_id: String,
//     pub user_id_to_act_on: String,
// }

#[derive(Deserialize)]
pub struct MediaStateUpdateRequest {
    pub room_id: String,
    pub user_id: String,
    pub camera_on: Option<bool>,
    pub mic_on: Option<bool>,
}

#[derive(Serialize)]
pub struct MediaStateUpdateResponse {
    pub room_id: String,
    pub message: String,
    pub user_id: String,
    pub camera_on: bool,
    pub mic_on: bool,
}

#[derive(Deserialize)]
pub struct DetailedRoomInfoResponse {
    pub room_id: String,
}

#[derive(Deserialize)]
pub struct LeaveRoomRequest {
    pub room_id: String,
    pub user_id: String,
}