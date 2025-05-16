use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use warp::ws::{Message, WebSocket};
use tokio::sync::mpsc;
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;
use uuid::Uuid;

pub struct SignalingState {
    pub rooms: HashMap<String, Vec<String>>,
    pub users: HashMap<String, mpsc::UnboundedSender<Message>>,
}

impl SignalingState {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            users: HashMap::new(),
        }
    }
    
    pub fn broadcast_to_room(&self, room_id: &str, sender_id: &str, message: &str) {
        if let Some(users) = self.rooms.get(room_id) {
            for user_id in users {
                // Don't send back to the sender
                if user_id != sender_id {
                    if let Some(tx) = self.users.get(user_id) {
                        tx.send(Message::text(message)).ok();
                    }
                }
            }
        }
    }

    pub fn join_room(&mut self, room_id: &str, user_id: &str) {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        if !room.contains(&user_id.to_string()) {
            room.push(user_id.to_string());
        }
    }

    pub fn leave_room(&mut self, room_id: &str, user_id: &str) {
        if let Some(users) = self.rooms.get_mut(room_id) {
            users.retain(|id| id != user_id);
            if users.is_empty() {
                self.rooms.remove(room_id);
            }
        }
    }

    pub fn get_room_users(&self, room_id: &str) -> Vec<String> {
        self.rooms.get(room_id).cloned().unwrap_or_default()
    }
}

pub async fn handle_websocket(ws: WebSocket, state: Arc<Mutex<SignalingState>>) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let user_id = Uuid::new_v4().to_string();

    {
        let mut state = state.lock().unwrap();
        state.users.insert(user_id.clone(), tx);
    }

    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            ws_tx.send(msg).await.ok();
        }
    });

    while let Some(result) = ws_rx.next().await {
        if let Ok(msg) = result {
            if let Ok(text) = msg.to_str() {
                let mut json_val = serde_json::from_str::<Value>(text).unwrap();
                // Add the sender ID to the message
                json_val["from"] = Value::String(user_id.clone());

                if let Ok(json_val) = serde_json::from_str::<Value>(text) {
                    if let Some(msg_type) = json_val.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "join" => {
                                if let Some(room) = json_val.get("room").and_then(|r| r.as_str()) {
                                    state.lock().unwrap().join_room(room, &user_id);
                                }
                            }
                            "offer" | "answer" | "candidate" => {
                                if let Some(room) = json_val.get("room").and_then(|r| r.as_str()) {
                                    // For room-specific messages
                                    let msg_text = serde_json::to_string(&json_val).unwrap();
                                    state.lock().unwrap().broadcast_to_room(room, &user_id, &msg_text);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    let mut state = state.lock().unwrap();
    state.users.remove(&user_id);
    for (_, users) in state.rooms.iter_mut() {
        users.retain(|id| id != &user_id);
    }
}