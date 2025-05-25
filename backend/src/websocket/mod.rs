use axum::{
    extract::{ws::{Message, WebSocket}, Extension, WebSocketUpgrade, Path},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::select;
use serde_json::json;

use crate::services::{AppState, GPS_UPDATES};
use crate::models::{WebSocketMessage, DroneGpsUpdate, ws_message_types};

// Создаем роутер для WebSocket
pub fn router() -> Router {
    Router::new()
        .route("/ws", get(handler_all_drones))
        .route("/ws/{drone_id}", get(handler_single_drone))
}

// Handler для WebSocket подключения, который будет отдавать данные по всем дронам
pub async fn handler_all_drones(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("WebSocket upgrade requested for all drones endpoint");
    ws.on_upgrade(|socket| handle_all_drones_socket(socket, state))
}

// Handler для WebSocket подключения к конкретному дрону
pub async fn handler_single_drone(
    ws: WebSocketUpgrade,
    Path(drone_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!(drone_id = %drone_id, "WebSocket upgrade requested for specific drone");
    ws.on_upgrade(move |socket| handle_single_drone_socket(socket, state, drone_id))
}

// Обработка соединения для всех дронов
async fn handle_all_drones_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("WebSocket connection established for all drones endpoint");
    let (mut sender, mut receiver) = socket.split();    // Получаем актуальные GPS-данные по всем дронам
    let all_drones_gps = match crate::services::get_all_drones_gps_data(state.clone()).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to get all drones GPS data");
            let _ = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::ERROR.to_string(),
                data: json!({ "error": format!("Failed to get drones data: {}", e) }),
            }).unwrap().into())).await;
            return;
        }
    };

    // Отправляем текущие данные по всем дронам
    if !all_drones_gps.is_empty() {
        if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
            message_type: ws_message_types::GPS_DATA.to_string(),
            data: json!(all_drones_gps),
        }).unwrap().into())).await {
            tracing::error!(error = ?e, "Failed to send initial GPS data");
            return;
        }
    }

    // Подписываемся на обновления GPS
    let mut gps_receiver = GPS_UPDATES.subscribe();

    // Асинхронно обрабатываем сообщения от клиента и обновления GPS
    loop {
        select! {
            // Обрабатываем входящие сообщения от клиента
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match handle_client_message(text.to_string(), state.clone(), &mut sender).await {
                            Err(e) => {
                                tracing::error!(error = ?e, "Failed to handle client message");
                                if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                                    message_type: ws_message_types::ERROR.to_string(),
                                    data: json!({ "error": format!("Failed to process message: {}", e) }),
                                }).unwrap().into())).await {
                                    tracing::error!(error = ?e, "Failed to send error message");
                                    break;
                                }
                            },
                            Ok(should_break) if should_break => break,
                            _ => {}
                        }
                    },
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebSocket close message received");
                        break;
                    },
                    Some(Err(e)) => {
                        tracing::error!(error = ?e, "WebSocket receive error");
                        break;
                    },
                    _ => {}
                }
            },
            // Обрабатываем обновления GPS
            gps_result = gps_receiver.recv() => {
                match gps_result {
                    Ok(gps_update) => {
                        if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                            message_type: ws_message_types::GPS_UPDATE.to_string(),
                            data: json!(gps_update),
                        }).unwrap().into())).await {
                            tracing::error!(error = ?e, "Failed to send GPS update");
                            break;
                        }
                    },
                    Err(e) => {
                        tracing::error!(error = ?e, "Failed to receive GPS update");
                        // Переподписываемся при ошибке
                        gps_receiver = GPS_UPDATES.subscribe();
                    }
                }
            }
        }
    }

    tracing::info!("WebSocket connection closed for all drones endpoint");
}

// Обработка соединения для конкретного дрона
async fn handle_single_drone_socket(socket: WebSocket, state: Arc<AppState>, drone_id: String) {
    tracing::info!(drone_id = %drone_id, "WebSocket connection established for specific drone");
    let (mut sender, mut receiver) = socket.split();
    
    // Проверяем, существует ли запрашиваемый дрон
    let drone = match crate::services::get_drone_by_id(state.clone(), drone_id.clone()).await {
        Ok(Some(drone)) => drone,
        Ok(None) => {
            tracing::error!(drone_id = %drone_id, "Drone not found");
            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::ERROR.to_string(),
                data: json!({ "error": format!("Drone with ID {} not found", drone_id) }),
            }).unwrap().into())).await {
                tracing::error!(error = ?e, "Failed to send error message");
            }
            return;
        },
        Err(e) => {
            tracing::error!(error = ?e, drone_id = %drone_id, "Failed to query drone");
            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::ERROR.to_string(),
                data: json!({ "error": format!("Failed to query drone: {}", e) }),
            }).unwrap().into())).await {
                tracing::error!(error = ?e, "Failed to send error message");
            }
            return;
        }    };    // Получаем последние GPS-данные для этого дрона
    let _drone_gps = match crate::services::get_drone_gps_data(state.clone(), drone_id.clone()).await {
        Ok(Some(gps)) => {
            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::GPS_DATA.to_string(),
                data: json!(gps),
            }).unwrap().into())).await {
                tracing::error!(error = ?e, "Failed to send initial GPS data");
                return;
            }
            Some(gps)
        },
        Ok(None) => {
            // Нет GPS-данных, но это нормально - просто отправляем информацию о дроне
            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::GPS_DATA.to_string(),
                data: json!({"drone": drone, "gps": null}),
            }).unwrap().into())).await {
                tracing::error!(error = ?e, "Failed to send drone info");
                return;
            }
            None
        },
        Err(e) => {
            tracing::error!(error = ?e, "Failed to get drone GPS data");
            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::ERROR.to_string(),
                data: json!({ "error": format!("Failed to get drone GPS data: {}", e) }),
            }).unwrap().into())).await {
                tracing::error!(error = ?e, "Failed to send error message");
            }
            return;
        }
    };

    // Подписываемся на обновления GPS
    let mut gps_receiver = GPS_UPDATES.subscribe();

    // Асинхронно обрабатываем сообщения от клиента и обновления GPS
    loop {
        select! {
            // Обрабатываем входящие сообщения от клиента
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match handle_client_message(text.to_string(), state.clone(), &mut sender).await {
                            Err(e) => {
                                tracing::error!(error = ?e, "Failed to handle client message");
                                if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                                    message_type: ws_message_types::ERROR.to_string(),
                                    data: json!({ "error": format!("Failed to process message: {}", e) }),
                                }).unwrap().into())).await {
                                    tracing::error!(error = ?e, "Failed to send error message");
                                    break;
                                }
                            },
                            Ok(should_break) if should_break => break,
                            _ => {}
                        }
                    },
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebSocket close message received");
                        break;
                    },
                    Some(Err(e)) => {
                        tracing::error!(error = ?e, "WebSocket receive error");
                        break;
                    },
                    _ => {}
                }
            },
            // Обрабатываем обновления GPS, но только для нашего дрона
            gps_result = gps_receiver.recv() => {
                match gps_result {
                    Ok(gps_update) => {
                        // Отправляем только обновления для нашего дрона
                        if gps_update.video_id == drone_id {
                            if let Err(e) = sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                                message_type: ws_message_types::GPS_UPDATE.to_string(),
                                data: json!(gps_update),
                            }).unwrap().into())).await {
                                tracing::error!(error = ?e, "Failed to send GPS update");
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!(error = ?e, "Failed to receive GPS update");
                        // Переподписываемся при ошибке
                        gps_receiver = GPS_UPDATES.subscribe();
                    }
                }
            }
        }
    }

    tracing::info!(drone_id = %drone_id, "WebSocket connection closed for specific drone");
}

// Handler for processing WebSocket client messages
async fn handle_client_message(
    text: String, 
    state: Arc<AppState>, 
    sender: &mut futures::stream::SplitSink<WebSocket, Message>
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!(message = %text, "WebSocket message received");
    
    match serde_json::from_str::<WebSocketMessage>(&text) {
        Ok(msg) => {
            match msg.message_type.as_str() {
                // Сообщение от дрона с обновлением GPS
                "gps_update" => {
                    let update = serde_json::from_value::<DroneGpsUpdate>(msg.data)?;
                    tracing::info!(
                        drone_id = %update.drone_id,
                        latitude = %update.latitude,
                        longitude = %update.longitude,
                        altitude = %update.altitude,
                        "GPS update received"
                    );
                    
                    // Сохраняем GPS данные в БД
                    let _ = crate::services::save_drone_gps_data(
                        state,
                        update.drone_id,
                        update.latitude,
                        update.longitude,
                        update.altitude
                    ).await?;
                    
                    // Подтверждаем обработку
                    sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                        message_type: "gps_update_ack".to_string(),
                        data: json!({ "status": "ok" }),
                    })?.into())).await?;
                }
                // Запрос на получение данных GPS
                "gps_request" => {
                    // Если в запросе указан drone_id, вернем данные только для этого дрона
                    if let Some(drone_id) = msg.data.get("drone_id").and_then(|v| v.as_str()) {
                        let gps_data = crate::services::get_drone_gps_data(state, drone_id.to_string()).await?;
                        sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                            message_type: ws_message_types::GPS_DATA.to_string(),
                            data: json!(gps_data),
                        })?.into())).await?;
                    } else {
                        // Иначе возвращаем данные по всем дронам
                        let all_gps_data = crate::services::get_all_drones_gps_data(state).await?;
                        sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                            message_type: ws_message_types::GPS_DATA.to_string(),
                            data: json!(all_gps_data),
                        })?.into())).await?;
                    }
                }
                _ => {
                    tracing::warn!(message_type = %msg.message_type, "Unknown message type");
                    sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                        message_type: ws_message_types::ERROR.to_string(),
                        data: json!({ "error": format!("Unknown message type: {}", msg.message_type) }),
                    })?.into())).await?;
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Invalid JSON message received");
            sender.send(Message::Text(serde_json::to_string(&WebSocketMessage {
                message_type: ws_message_types::ERROR.to_string(),
                data: json!({ "error": "Invalid JSON message" }),
            })?.into())).await?;
        }
    }
    
    Ok(false) // Не прерывать соединение
}
