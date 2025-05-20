mod signaling;
mod webrtc_handler;

use std::sync::{Arc, Mutex};
use warp::Filter;
use signaling::{SignalingState, handle_websocket};
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Запуск сигнального сервера...");

    let state = Arc::new(Mutex::new(SignalingState::new()));
    let state_filter = warp::any().map(move || state.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["Content-Type"]);

    let signaling = warp::path("signaling")
        .and(warp::ws())
        .and(state_filter.clone())
        .map(|ws: warp::ws::Ws, state| {
            info!("Новое WebSocket подключение инициировано"); // Лог перед апгрейдом
            ws.on_upgrade(move |socket| {
                info!("WebSocket соединение установлено"); // Лог после успешного апгрейда
                handle_websocket(socket, state)
            })
        })
        .with(cors.clone());

    let central = warp::path("central")
        .and(warp::ws())
        .and(state_filter)
        .map(|ws: warp::ws::Ws, state| {
            info!("Новое центральное WebSocket подключение инициировано");
            ws.on_upgrade(move |socket| {
                info!("Центральное WebSocket соединение установлено");
                handle_websocket(socket, state)
            })
        })
        .with(cors);

    warp::serve(signaling.or(central)).run(([0, 0, 0, 0], 3030)).await;
}
