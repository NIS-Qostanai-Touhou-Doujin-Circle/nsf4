// use actix::{Actor, Addr};
use actix_web::{App, HttpServer};
use actix_cors::Cors;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://127.0.0.1:3031");
    
     HttpServer::new(move || {
        let _cors = Cors::default()
            .allow_any_origin() // For development; restrict in production!
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            // .wrap(cors) // Add the CORS middleware here
            // .app_data(app_state.clone())
            // // Room management endpoints
            // .service(api::create_room_handler)
            // .service(api::get_room_info_handler)
            // // User management endpoints
            // .service(api::request_join_room_handler)
            // // Media endpoints
            // .service(api::update_media_status_handler)
            // .service(api::leave_room_handler)
            // // WebSocket endpoint
            // .service(ws::websocket_route)
    })
    .bind("127.0.0.1:3031")?
    .run()
    .await
}