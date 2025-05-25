use futures::{SinkExt, StreamExt, future::BoxFuture};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use std::sync::Arc;
use serde_json::{json, Value};
use tracing::{info, error};
use tokio::time::Duration;
use std::error::Error as StdError;

use crate::models::DroneGpsUpdate;
use super::AppState;

pub fn connect_to_drone(
    state: Arc<AppState>,
    drone_id: String,
    ws_url: String
) -> BoxFuture<'static, Result<(), Box<dyn StdError + Send + Sync>>> {
    Box::pin(async move {
    // Парсим URL
    let url = Url::parse(&ws_url).map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;
    info!(drone_id = %drone_id, url = %url, "Connecting to drone WebSocket");
    
    // Устанавливаем соединение
    let (ws_stream, _) = match connect_async(url.to_string()).await {
        Ok(conn) => conn,
        Err(e) => {
            error!(drone_id = %drone_id, error = %e, "Failed to connect to drone WebSocket");
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Failed to connect to drone: {}", e),
            )) as Box<dyn StdError + Send + Sync>);
        }
    };
    info!(drone_id = %drone_id, "Connected to drone WebSocket");
    
    let (mut write, mut read) = ws_stream.split();
    
    // Отправляем сообщение об аутентификации или инициализации 
    // (зависит от протокола дрона, здесь пример)
    let init_message = json!({
        "type": "init",
        "drone_id": drone_id,
    });
    
    write.send(Message::Text(init_message.to_string().into())).await
        .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;
    info!(drone_id = %drone_id, "Sent initialization message to drone");
    
    // Цикл обработки сообщений от дрона
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!(drone_id = %drone_id, message = %text, "Received message from drone");
                
                // Парсим JSON сообщение
                match serde_json::from_str::<Value>(&text) {
                    Ok(value) => {
                        // Проверяем тип сообщения
                        if let Some("gps") = value.get("type").and_then(|v| v.as_str()) {
                            // Извлекаем GPS данные
                            if let (
                                Some(lat), 
                                Some(lng), 
                                Some(alt)
                            ) = (
                                value.get("latitude").and_then(|v| v.as_f64()),
                                value.get("longitude").and_then(|v| v.as_f64()),
                                value.get("altitude").and_then(|v| v.as_f64())
                            ) {
                                // Создаем объект обновления GPS
                                let update = DroneGpsUpdate {
                                    drone_id: drone_id.clone(),
                                    latitude: lat,
                                    longitude: lng,
                                    altitude: alt,
                                    timestamp: value.get("timestamp").and_then(|v| v.as_str()).map(String::from),
                                    title: value.get("title").and_then(|v| v.as_str()).map(String::from),
                                };
                                
                                // Сохраняем данные в БД
                                match crate::services::save_drone_gps_data(
                                    state.clone(),
                                    update.drone_id.clone(),
                                    update.latitude,
                                    update.longitude,
                                    update.altitude
                                ).await {
                                    Ok(_) => {
                                        info!(
                                            drone_id = %update.drone_id,
                                            latitude = %update.latitude,
                                            longitude = %update.longitude,
                                            altitude = %update.altitude,
                                            "Saved drone GPS data"
                                        );
                                    },
                                    Err(e) => {
                                        error!(
                                            drone_id = %update.drone_id,
                                            error = %e,
                                            "Failed to save drone GPS data"
                                        );
                                    }
                                }
                                
                                // Отправляем подтверждение получения
                                let ack = json!({
                                    "type": "gps_ack",
                                    "status": "ok",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                });
                                if let Err(e) = write.send(Message::Text(ack.to_string().into())).await {
                                    error!(drone_id = %drone_id, error = %e, "Failed to send acknowledgment");
                                }
                            }
                        }
                    },
                    Err(e) => {
                        error!(drone_id = %drone_id, error = %e, "Failed to parse message from drone");
                    }                }
            },
            Ok(Message::Close(reason)) => {
                info!(drone_id = %drone_id, ?reason, "Drone connection closed");
                
                // Remove connection from manager when closed
                {
                    let mut connection_manager = super::DRONE_CONNECTIONS.lock().unwrap();
                    connection_manager.remove_connection(&drone_id);
                }
                
                break;
            },
            Ok(_) => {}, // Игнорируем другие типы сообщений
            Err(e) => {
                error!(drone_id = %drone_id, error = %e, "Error reading from drone WebSocket");
                
                // Remove current connection from manager before reconnecting
                {
                    let mut connection_manager = super::DRONE_CONNECTIONS.lock().unwrap();
                    connection_manager.remove_connection(&drone_id);
                }
                
                tokio::time::sleep(Duration::from_secs(5)).await;
                return connect_to_drone(state, drone_id, ws_url).await;
            }        }
    }
    
    Ok(())
    })
}

// Функция для запуска клиента для всех дронов
pub async fn start_drone_clients(state: Arc<AppState>) -> Result<(), Box<dyn StdError + Send + Sync>> {
    // Получаем список всех дронов
    let drones = match crate::database::get_videos(&state.db).await {
        Ok(drones) => drones,
        Err(e) => {
            error!(error = %e, "Failed to get drones list");
            return Err(Box::new(e) as Box<dyn StdError + Send + Sync>);        }
    };
    
    // Запускаем клиент для каждого дрона
    for drone in drones {
        // Используем сохраненный ws_url если он есть, иначе пропускаем дрон
        if let Some(ws_url) = drone.ws_url.as_ref().filter(|url| !url.trim().is_empty()) {
            let state_clone = state.clone();
            let drone_id = drone.id.clone();
            let drone_id_for_task = drone_id.clone();
            let ws_url_clone = ws_url.clone();
            
            info!(drone_id = %drone.id, ws_url = %ws_url, "Started WebSocket client for drone");
            
            let connection_task = tokio::spawn(async move {
                match connect_to_drone(state_clone, drone_id_for_task.clone(), ws_url_clone).await {
                    Ok(_) => info!(drone_id = %drone_id_for_task, "Drone client finished successfully"),
                    Err(e) => error!(drone_id = %drone_id_for_task, error = %e, "Drone client error"),
                }
                
                // Remove from connection manager when task finishes
                {
                    let mut connection_manager = super::DRONE_CONNECTIONS.lock().unwrap();
                    connection_manager.remove_connection(&drone_id_for_task);
                }
            });
            
            // Register the connection in the manager
            {
                let mut connection_manager = super::DRONE_CONNECTIONS.lock().unwrap();
                connection_manager.add_connection(drone_id.clone(), connection_task.abort_handle());
            }
        } else {
            info!(drone_id = %drone.id, "No WebSocket URL configured for drone, skipping WebSocket connection");
        }
    }
    
    Ok(())
}
