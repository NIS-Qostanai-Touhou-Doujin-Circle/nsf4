use serde_json::from_str;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use serde_json::json;
use std::time::Duration;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Настраиваемые параметры
    let args: Vec<String> = env::args().collect();
    let port = if args.len() > 1 { &args[1] } else { "9002" };
    let drone_id = if args.len() > 2 { &args[2] } else { "drone-sim-1" };
    let addr = format!("0.0.0.0:{}", port);
    
    // Базовые координаты (можно изменить для разных локаций)
    let base_latitude = 55.751244; // Москва
    let base_longitude = 37.618423;
    
    println!("Drone simulator starting on {} with ID {}", addr, drone_id);
    let listener = TcpListener::bind(&addr).await?;
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        println!("Incoming connection from: {}", addr);
        
        let drone_id = drone_id.to_string();
        
        tokio::spawn(async move {
            // Принимаем WebSocket-соединение
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    let (mut tx, mut rx) = ws_stream.split();
                    println!("WebSocket connection established with: {}", addr);                    // Создаем генератор случайных чисел (thread-safe)
                    let mut rng = StdRng::from_os_rng();
                    
                    // Отправляем приветственное сообщение
                    let welcome_msg = json!({
                        "type": "info",
                        "message": "Connected to drone simulator",
                        "drone_id": drone_id
                    });
                    
                    if let Err(e) = tx.send(Message::Text(welcome_msg.to_string().into())).await {
                        println!("Error sending welcome message: {}", e);
                    }
                    
                    // Цикл обработки сообщений
                    let mut interval = tokio::time::interval(Duration::from_secs(1));
                    let mut current_latitude = base_latitude;
                    let mut current_longitude = base_longitude;
                    let mut current_altitude = 100.0;
                    
                    loop {
                        tokio::select! {
                            // Отправляем обновления GPS каждую секунду
                            _ = interval.tick() => {
                                // Имитируем небольшое перемещение дрона
                                current_latitude += rng.random_range(-0.0001..0.0001);
                                current_longitude += rng.random_range(-0.0001..0.0001);
                                current_altitude += rng.random_range(-1.0..1.0);
                                
                                let gps_update = json!({
                                    "type": "gps",
                                    "drone_id": drone_id,
                                    "latitude": current_latitude,
                                    "longitude": current_longitude,
                                    "altitude": current_altitude,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                });
                                
                                if let Err(e) = tx.send(Message::Text(gps_update.to_string().into())).await {
                                    println!("Error sending GPS update: {}", e);
                                    break;
                                }
                            }
                            
                            // Обрабатываем входящие сообщения
                            message = rx.next() => {
                                match message {
                                    Some(Ok(Message::Text(text))) => {
                                        println!("Received message: {}", text);
                                        // Здесь можно добавить обработку команд для дрона
                                        match from_str::<serde_json::Value>(&text) {
                                            Ok(json) => {
                                                if let Some(message_type) = json.get("type").and_then(|v| v.as_str()) {
                                                    match message_type {
                                                        "init" => {
                                                            println!("Initialization request received");
                                                            // Отправляем подтверждение
                                                            let ack = json!({
                                                                "type": "init_ack",
                                                                "status": "ok",
                                                                "drone_id": drone_id,
                                                            });
                                                            if let Err(e) = tx.send(Message::Text(ack.to_string().into())).await {
                                                                println!("Error sending init_ack: {}", e);
                                                            }
                                                        },
                                                        "gps_ack" => {
                                                            // Подтверждение получения GPS данных, ничего не делаем
                                                        },
                                                        _ => println!("Unknown message type: {}", message_type),
                                                    }
                                                }
                                            },
                                            Err(e) => println!("Error parsing message: {}", e),
                                        }
                                    },
                                    Some(Ok(Message::Close(_))) => {
                                        println!("Client disconnected");
                                        break;
                                    },
                                    Some(Err(e)) => {
                                        println!("Error receiving message: {}", e);
                                        break;
                                    },
                                    _ => {},
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("Error during WebSocket handshake: {}", e),
            }
        });
    }

    Ok(())
}
