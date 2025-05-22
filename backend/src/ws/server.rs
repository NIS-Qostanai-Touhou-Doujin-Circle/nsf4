use actix::{Actor, Context, Addr, Handler};
use std::collections::HashMap;
use crate::messages::SetAppState;
use crate::messages::{Connect, Disconnect, SendMessage, WsMessage};
use crate::ws::connection::WsConnection;
use crate::models::{AppState, Participant, User, Room};

// WebSocket server for managing connections
pub struct WsServer {
    sessions: HashMap<(String, String), Addr<WsConnection>>, // (room_id, user_id) -> connection
    app_state: Option<actix_web::web::Data<AppState>>, // Optional reference to AppState
}

impl WsServer {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            app_state: None,
        }
    }
    
    // Set app state reference
    pub fn set_app_state(&mut self, app_state: actix_web::web::Data<AppState>) {
        self.app_state = Some(app_state);
    }
    
    // Helper method to process approval
    fn process_approval(&self, room_id: &str, creator_id: &str, user_id: &str) {
        if let Some(state) = &self.app_state {
            let mut rooms_guard = state.rooms.lock().unwrap();
            
            if let Some(room) = rooms_guard.get_mut(room_id) {
                // Verify sender is room creator
                if room.creator_id != creator_id {
                    println!("User {} is not the creator of room {}", creator_id, room_id);
                    return;
                }
                
                if let Some(user_to_add) = room.pending_requests.remove(user_id) {
                    let new_participant = Participant {
                        user: user_to_add.clone(),
                        camera_on: false,
                        mic_on: false,
                        connected: false,
                    };
                    room.participants.insert(user_id.to_string(), new_participant);

                    // Notify the approved user
                    self.send_to_specific_user(room_id, user_id, WsMessage::JoinApproved { 
                        user_id: user_id.to_string() 
                    });
                    
                    // Notify all room participants about the new user
                    self.send_to_room(room_id, WsMessage::Connect { 
                        user_id: user_id.to_string(),
                        display_name: user_to_add.display_name,
                    });
                    
                    println!("User {} approved to join room {} by creator {}", 
                        user_id, room_id, creator_id);
                }
            }
        }
    }

    // Helper method to process denial
    fn process_denial(&self, room_id: &str, creator_id: &str, user_id: &str) {
        if let Some(state) = &self.app_state {
            let mut rooms_guard = state.rooms.lock().unwrap();
            
            if let Some(room) = rooms_guard.get_mut(room_id) {
                // Verify sender is room creator
                if room.creator_id != creator_id {
                    println!("User {} is not the creator of room {}", creator_id, room_id);
                    return;
                }
                
                if room.pending_requests.remove(user_id).is_some() {
                    // Notify denied user
                    self.send_to_specific_user(room_id, user_id, WsMessage::JoinDenied { 
                        user_id: user_id.to_string() 
                    });
                    
                    println!("User {} denied from joining room {} by creator {}", 
                        user_id, room_id, creator_id);
                }
            }
        }
    }
    
    // Helper to send message to specific user
    fn send_to_specific_user(&self, room_id: &str, user_id: &str, message: WsMessage) {
        if let Some(addr) = self.sessions.get(&(room_id.to_string(), user_id.to_string())) {
            addr.do_send(message);
        }
    }
    
    // Helper to send message to all users in room
    fn send_to_room(&self, room_id: &str, message: WsMessage) {
        for ((r, _), addr) in self.sessions.iter().filter(|((r, _), _)| r == room_id) {
            addr.do_send(message.clone());
        }
    }
}

// Implement Clone for WsServer
impl Clone for WsServer {
    fn clone(&self) -> Self {
        WsServer {
            sessions: self.sessions.clone(),
            app_state: self.app_state.clone(),
        }
    }
}

// Actor implementation for WsServer
impl Actor for WsServer {
    type Context = Context<Self>;
}

impl Handler<SetAppState> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: SetAppState, _: &mut Context<Self>) {
        self.app_state = Some(msg.app_state);
    }
}

// Handle Connect messages
impl Handler<Connect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        println!("User {} connected to room {}", msg.user_id, msg.room_id);
        self.sessions.insert((msg.room_id, msg.user_id), msg.addr);
    }
}

// Handle Disconnect messages
impl Handler<Disconnect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("User {} disconnected from room {}", msg.user_id, msg.room_id);
        self.sessions.remove(&(msg.room_id, msg.user_id));
    }
}

// Handle SendMessage messages
impl Handler<SendMessage> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: SendMessage, _: &mut Context<Self>) {
        match &msg.message {
            // Handle approval/denial messages
            WsMessage::ApproveJoinRequest { user_id } => {
                // msg.sender_id is the user who sent the ApproveJoinRequest command
                self.process_approval(&msg.room_id, &msg.sender_id, user_id);
            },
            WsMessage::DenyJoinRequest { user_id } => {
                // msg.sender_id is the user who sent the DenyJoinRequest command
                self.process_denial(&msg.room_id, &msg.sender_id, user_id);
            },
            _ => {
                // Normal message routing
                match msg.target_user_id {
                    Some(user_id) => {
                        // Send message to specific user
                        if let Some(addr) = self.sessions.get(&(msg.room_id.clone(), user_id.clone())) {
                            addr.do_send(msg.message);
                        }
                    }
                    None => {
                        // Send message to all users in the room
                        for ((room_id, _), addr) in self.sessions.iter().filter(|((r, _), _)| r == &msg.room_id) {
                            addr.do_send(msg.message.clone());
                        }
                    }
                }
            }
        }
    }
}