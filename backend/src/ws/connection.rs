use actix::{Actor, Addr, Handler, Running, StreamHandler};
use actix::AsyncContext;  // Add this for run_interval and address methods
use actix::ActorContext;  // Add this for stop method
use actix_web_actors::ws;
use std::time::{Duration, Instant};

use crate::messages::{Disconnect, WsMessage, SendMessage};
use crate::ws::server::WsServer;
use crate::messages::Connect;

// Constants for WebSocket
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// WebSocket connection structure
pub struct WsConnection {
    room_id: String,
    user_id: String,
    last_heartbeat: Instant,
    ws_server_addr: Addr<WsServer>,
}

impl WsConnection {
    pub fn new(room_id: String, user_id: String, ws_server: Addr<WsServer>) -> Self {
        Self {
            room_id,
            user_id,
            last_heartbeat: Instant::now(),
            ws_server_addr: ws_server,
        }
    }
    
    // Periodic heartbeat check
    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check for timeout
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                println!("WebSocket Client heartbeat failed, disconnecting!");
                // Notify server about disconnection
                act.ws_server_addr.do_send(Disconnect {
                    room_id: act.room_id.clone(),
                    user_id: act.user_id.clone(),
                });
                ctx.stop();
                return;
            }
            
            ctx.ping(b"");
        });
    }
}

// WebSocket actor implementation
impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);

        // Register connection with WsServer
        let addr = ctx.address();
        self.ws_server_addr.do_send(Connect {
            room_id: self.room_id.clone(),
            user_id: self.user_id.clone(),
            addr,
        });
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // Send disconnect message
        self.ws_server_addr.do_send(Disconnect {
            room_id: self.room_id.clone(),
            user_id: self.user_id.clone(),
        });
        Running::Stop
    }
}

// Handle WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                // Handle client JSON messages
                if let Ok(ws_message) = serde_json::from_str::<WsMessage>(&text) {
                    match &ws_message {
                        WsMessage::WebRTC { message } => {
                            // Forward WebRTC signals to the target user
                            use crate::messages::{SendMessage, WebRtcMessage};
                            
                            let target_user_id = match message {
                                WebRtcMessage::Offer { to_user_id, .. } |
                                WebRtcMessage::Answer { to_user_id, .. } |
                                WebRtcMessage::IceCandidate { to_user_id, .. } => Some(to_user_id.clone()),
                            };
                            
                            if let Some(to_user_id) = target_user_id {
                                self.ws_server_addr.do_send(SendMessage {
                                    room_id: self.room_id.clone(),
                                    sender_id: self.user_id.clone(), // Added sender_id
                                    target_user_id: Some(to_user_id),
                                    message: WsMessage::WebRTC { message: message.clone() },
                                });
                            }
                        }
                        WsMessage::Ping => {
                            self.last_heartbeat = Instant::now();
                            ctx.text(serde_json::to_string(&WsMessage::Pong).unwrap());
                        }
                        // Handle approval/denial from room creator
                        WsMessage::ApproveJoinRequest { user_id } => {
                            println!("User {} attempting to approve join request for {}", self.user_id, user_id);
                            
                            // Forward to server for processing
                            self.ws_server_addr.do_send(SendMessage {
                                room_id: self.room_id.clone(),
                                sender_id: self.user_id.clone(), // This connection's user is the sender
                                target_user_id: None, // Server will process this based on sender_id's rights
                                message: WsMessage::ApproveJoinRequest { 
                                    user_id: user_id.clone() 
                                },
                            });
                        }
                        WsMessage::DenyJoinRequest { user_id } => {
                            println!("User {} attempting to deny join request for {}", self.user_id, user_id);
                            
                            // Forward to server for processing
                            self.ws_server_addr.do_send(SendMessage {
                                room_id: self.room_id.clone(),
                                sender_id: self.user_id.clone(), // This connection's user is the sender
                                target_user_id: None, // Server will process this based on sender_id's rights
                                message: WsMessage::DenyJoinRequest { 
                                    user_id: user_id.clone() 
                                },
                            });
                        }
                        _ => {
                            println!("Received unhandled WebSocket message: {:?}", ws_message);
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

// Handle WsMessage sent from server to client
impl Handler<WsMessage> for WsConnection {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        // Convert WsMessage to JSON and send over WebSocket
        if let Ok(text) = serde_json::to_string(&msg) {
            ctx.text(text);
        } else {
            println!("Failed to serialize WebSocket message");
        }
    }
}