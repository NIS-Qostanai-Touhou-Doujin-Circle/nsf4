// use actix::{Actor, Addr};
use actix_web::{App, HttpServer};
use actix_cors::Cors;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://127.0.0.1:3030");
    
     HttpServer::new(move || {
        let _cors = Cors::default()
            .allow_any_origin() // For development; restrict in production!
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            // .wrap(cors) // Add the CORS middleware here
            // .app_data(app_state.clone())
    })
    .bind("127.0.0.1:3031")?
    .run()
    .await
}