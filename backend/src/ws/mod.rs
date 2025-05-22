
pub mod server;
pub mod connection;

use actix_web::{web, HttpRequest, HttpResponse, get, Error};
use actix_web_actors::ws;

use crate::models::AppState;
use self::connection::WsConnection;

// WebSocket connection handler
#[get("/ws/{room_id}/{user_id}")]
pub async fn websocket_route(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (room_id, user_id) = path.into_inner();
    let rooms_lock = state.rooms.lock().unwrap();
    
    // Verify room exists and user has access
    if let Some(room) = rooms_lock.get(&room_id) {
        if room.participants.contains_key(&user_id) || room.creator_id == user_id {
            // Allow WebSocket connection
            let ws = WsConnection::new(
                room_id.clone(), 
                user_id.clone(),
                state.ws_server.clone()
            );
            
            return ws::start(ws, &req, stream);
        }
    }
    
    Ok(HttpResponse::Forbidden().finish())
}