use crate::models::AppState;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, get, post};
use actix_cors::Cors;
use log::info;
use serde_json::json;

pub async fn start_http_server(app_state: AppState) -> std::io::Result<()> {
    let state = web::Data::new(app_state.clone());
    
    info!("Starting HTTP API server on port {}", app_state.config.http_port);
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
            
        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .service(get_stream_list)
            .service(get_stream_info)
            .service(health_check)
    })
    .bind(format!("0.0.0.0:{}", app_state.config.http_port))?
    .run()
    .await
}

#[get("/streams")]
async fn get_stream_list(app_state: web::Data<AppState>) -> impl Responder {
    let streams = app_state.stream_manager.lock().unwrap();
    
    let rtmp_streams: Vec<_> = streams.rtmp_streams.values().cloned().collect();
    let rtsp_streams: Vec<_> = streams.rtsp_streams.values().cloned().collect();
    
    HttpResponse::Ok().json(json!({
        "rtmp_streams": rtmp_streams,
        "rtsp_streams": rtsp_streams,
    }))
}

#[get("/streams/{id}")]
async fn get_stream_info(app_state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let streams = app_state.stream_manager.lock().unwrap();
    
    // Try to find in RTMP streams first
    if let Some(stream) = streams.rtmp_streams.get(&id) {
        return HttpResponse::Ok().json(json!({
            "stream_type": "rtmp",
            "stream": stream
        }));
    }
    
    // Then check RTSP streams
    if let Some(stream) = streams.rtsp_streams.get(&id) {
        return HttpResponse::Ok().json(json!({
            "stream_type": "rtsp",
            "stream": stream
        }));
    }
    
    HttpResponse::NotFound().json(json!({"error": "Stream not found"}))
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}
