use actix_web::{web, HttpResponse, Responder, post};
use std::collections::HashMap;

use crate::models::*;
use crate::messages::{SendMessage, WsMessage};

// Create a new room
#[post("/rooms/create")]
pub async fn create_room_handler(
    state: web::Data<AppState>, 
    req_body: web::Json<CreateRoomRequest>,
) -> impl Responder {
    let mut rooms_guard = state.rooms.lock().expect("Failed to lock rooms mutex");
    let room_id = req_body.room_id.clone();
    let creator_id = req_body.creator_id.clone();
    let creator_user = User { id: creator_id.clone(), display_name: "Creator".to_string() };
    println!("Creating room with ID: {}", room_id);

    if rooms_guard.contains_key(&room_id) {
        println!("Room with ID {} already exists", room_id);
        return HttpResponse::Conflict().json(GeneralMessageResponse {
            message: "Room with this ID already exists".to_string(),
            room_id: Some(room_id),
            user_id: None,
        });
    }

    let creator_participant = Participant {
        user: creator_user.clone(),
        camera_on: false,
        mic_on: false,
        connected: false,
    };

    let mut participants = HashMap::new();
    participants.insert(creator_id.clone(), creator_participant.clone());

    let new_room = Room {
        id: room_id.clone(),
        creator_id: creator_id.clone(),
        participants,
        pending_requests: HashMap::new(),
    };
    
    rooms_guard.insert(room_id.clone(), new_room.clone());

    HttpResponse::Created().json(RoomResponse {
        id: new_room.id,
        creator_id: new_room.creator_id,
        participants: new_room.participants.values().cloned().collect(),
        pending_requests: new_room.pending_requests.values().cloned().collect(),
    })
}

// Request to join a room
#[post("/rooms/join")]
pub async fn request_join_room_handler(
    state: web::Data<AppState>, 
    req_body: web::Json<JoinRoomRequest>,
) -> impl Responder {
    let room_id = req_body.room_id.clone();
    let mut rooms_guard = state.rooms.lock().unwrap();
    let user_id_to_join = req_body.user_id.clone();
    let requesting_user = User { 
        id: user_id_to_join.clone(), 
        display_name: req_body.display_name.clone() 
    };
    
    println!("{} requests to join room: {}", user_id_to_join, room_id);
    
    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.participants.contains_key(&user_id_to_join) {
                return HttpResponse::Ok().json(GeneralMessageResponse {
                    message: "User already in room".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_join),
                });
            }
            if room.pending_requests.contains_key(&user_id_to_join) {
                 return HttpResponse::Ok().json(GeneralMessageResponse {
                    message: "Join request already pending".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_join),
                });
            }

            room.pending_requests.insert(user_id_to_join.clone(), requesting_user.clone());

            // Notify room creator via WebSocket
            state.ws_server.do_send(SendMessage {
                room_id: room_id.clone(),
                sender_id: requesting_user.id.clone(),
                target_user_id: Some(room.creator_id.clone()),
                message: WsMessage::JoinRequest { 
                    user_id: user_id_to_join.clone(), 
                    display_name: requesting_user.display_name 
                },
            });

            HttpResponse::Ok().json(GeneralMessageResponse {
                message: "Join request sent. Waiting for approval.".to_string(),
                room_id: Some(room_id),
                user_id: Some(user_id_to_join),
            })
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// Update media status (camera/mic)
#[post("/rooms/media_status")]
pub async fn update_media_status_handler(
    state: web::Data<AppState>, 
    req_body: web::Json<MediaStateUpdateRequest>
) -> impl Responder {
    let room_id = req_body.room_id.clone();
    let mut rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if let Some(participant) = room.participants.get_mut(&req_body.user_id) {
                if let Some(cam_status) = req_body.camera_on {
                    participant.camera_on = cam_status;
                }
                if let Some(mic_status) = req_body.mic_on {
                    participant.mic_on = mic_status;
                }

                // Notify all room participants about media status change
                state.ws_server.do_send(SendMessage {
                    room_id: room_id.clone(),
                    sender_id: req_body.user_id.clone(),
                    target_user_id: None, // All users in room
                    message: WsMessage::MediaStatus { 
                        user_id: req_body.user_id.clone(),
                        camera_on: participant.camera_on,
                        mic_on: participant.mic_on,
                    },
                });

                println!("User {} in room {} updated media status: cam={}, mic={}",
                    req_body.user_id, room_id, participant.camera_on, participant.mic_on);

                HttpResponse::Ok().json(MediaStateUpdateResponse {
                    room_id: room_id.clone(),
                    message: "Media status updated".to_string(),
                    user_id: req_body.user_id.clone(),
                    camera_on: participant.camera_on,
                    mic_on: participant.mic_on,
                })
            } else {
                HttpResponse::NotFound().json(GeneralMessageResponse {
                    message: "Participant not found in room".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// Leave room
#[post("/rooms/leave")]
pub async fn leave_room_handler(
    state: web::Data<AppState>, 
    req_body: web::Json<LeaveRoomRequest>,
) -> impl Responder {
    let room_id = req_body.room_id.clone();
    let mut rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.participants.remove(&req_body.user_id).is_some() {
                // Notify all room participants about user leaving
                state.ws_server.do_send(SendMessage {
                    room_id: room_id.clone(),
                    sender_id: req_body.user_id.clone(),
                    target_user_id: None, // All users in room
                    message: WsMessage::Disconnect { 
                        user_id: req_body.user_id.clone() 
                    },
                });
                
                println!("User {} left room {}", req_body.user_id, room_id);

                let room_is_now_empty = room.participants.is_empty();
                let message = if room_is_now_empty {
                    println!("Room {} is now empty.", room_id);
                    format!("User {} left room. Room is now empty.", req_body.user_id)
                } else {
                    format!("User {} left room.", req_body.user_id)
                };

                HttpResponse::Ok().json(GeneralMessageResponse {
                    message,
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            } else {
                HttpResponse::BadRequest().json(GeneralMessageResponse {
                    message: format!("User {} not found in room.", req_body.user_id),
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// Get room information
#[post("/rooms/details")]
pub async fn get_room_info_handler(
    state: web::Data<AppState>, 
    req_body: web::Json<DetailedRoomInfoResponse>,
) -> impl Responder {
    let room_id = req_body.room_id.clone();
    let rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get(&room_id) {
        Some(room) => HttpResponse::Ok().json(RoomResponse {
            id: room.id.clone(),
            creator_id: room.creator_id.clone(),
            participants: room.participants.values().cloned().collect(),
            pending_requests: room.pending_requests.values().cloned().collect(),
        }),
        None => HttpResponse::NotFound().json(GeneralMessageResponse{
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}