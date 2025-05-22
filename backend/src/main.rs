mod models;
mod messages;
mod ws;
mod api;

use actix::{Actor, Addr};
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create WsServer actor
    let ws_server = ws::server::WsServer::new();
    
    // Start the server to get the address
    let ws_server_addr = ws_server.start();
    
    // Initialize app state with the server address
    let app_state = web::Data::new(models::AppState {
        rooms: Mutex::new(HashMap::new()),
        ws_server: ws_server_addr.clone(),
    });
    
    // Add a new message type in messages.rs for setting app state
    // Then send it to the running actor instead of calling set_app_state directly
    ws_server_addr.do_send(messages::SetAppState {
        app_state: app_state.clone(),
    });
    
    println!("Starting server at http://127.0.0.1:3030");
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // Room management endpoints
            .service(api::create_room_handler)
            .service(api::get_room_info_handler)
            // User management endpoints
            .service(api::request_join_room_handler)
            // Media endpoints
            .service(api::update_media_status_handler)
            .service(api::leave_room_handler)
            // WebSocket endpoint
            .service(ws::websocket_route)
    })
    .bind("127.0.0.1:3030")?
    .run()
    .await
}