use serde_json::from_str;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use serde_json::json;
use std::time::{Duration, Instant};
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::prelude::*;
use std::env;
use tokio::time::sleep;

// Physics-based drone motion simulator with smooth acceleration and jerk
#[derive(Clone)]
struct DronePhysics {
    // Base center for circular flight (degrees)
    base_lat: f64,
    base_lng: f64,
    // Circular flight state
    angle: f64,           // radians
    angular_speed: f64,   // radians per second
    radius: f64,          // degrees offset
    // Current position (degrees)
    lat: f64,
    lng: f64,
    alt: f64,
    // Velocity (degrees/second)
    vel_lat: f64,
    vel_lng: f64,
    vel_alt: f64,
    // Target waypoint (current position)
    target_lat: f64,
    target_lng: f64,
    target_alt: f64,
}

impl DronePhysics {
    fn new(init_lat: f64, init_lng: f64, init_alt: f64) -> Self {
        // Circular flight settings
        let radius = 0.0005;       // ~50m offset
        let angular_speed = 0.2;   // rad/s (~31s per circle)
        let angle = 0.0f64;
        // Compute initial position
        let lat = init_lat + radius * angle.cos();
        let lng = init_lng + radius * angle.sin();
        Self {
            base_lat: init_lat,
            base_lng: init_lng,
            angle,
            angular_speed,
            radius,
            lat,
            lng,
            alt: init_alt,
            vel_lat: 0.0,
            vel_lng: 0.0,
            vel_alt: 0.0,
            target_lat: lat,
            target_lng: lng,
            target_alt: init_alt,
        }
    }
    
    fn update(&mut self, dt: f64, _rng: &mut StdRng) {
        // Advance angle for circular flight
        self.angle = (self.angle + self.angular_speed * dt) % (2.0 * std::f64::consts::PI);
        // Update position
        self.lat = self.base_lat + self.radius * self.angle.cos();
        self.lng = self.base_lng + self.radius * self.angle.sin();
        // Velocity components
        self.vel_lat = -self.radius * self.angular_speed * self.angle.sin();
        self.vel_lng =  self.radius * self.angular_speed * self.angle.cos();
        self.vel_alt = 0.0;
        
        // Altitude remains constant, vel_alt is 0
        // self.alt remains self.alt;
        // self.vel_alt remains 0.0;

        // Update target to current position for reporting purposes
        self.target_lat = self.lat;
        self.target_lng = self.lng;
        self.target_alt = self.alt;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Настраиваемые параметры
    let args: Vec<String> = env::args().collect();
    let port = if args.len() > 1 { &args[1] } else { "9002" };
    let drone_id = if args.len() > 2 { &args[2] } else { "drone-sim-1" };
    let addr = format!("0.0.0.0:{}", port);
    
    // Базовые координаты (обновлены согласно запросу)
    // Init point: Lat, Long
    let base_latitude = 53.218282;
    let base_longitude = 63.658686;
    
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
                    println!("WebSocket connection established with: {}", addr);
                    
                    // Создаем генератор случайных чисел и физику дрона
                    let mut rng = StdRng::from_os_rng();
                    let mut drone_physics = DronePhysics::new(base_latitude, base_longitude, 120.0);
                    
                    // Отправляем приветственное сообщение
                    let welcome_msg = json!({
                        "type": "info",
                        "message": "Connected to drone simulator",
                        "drone_id": drone_id
                    });
                    
                    if let Err(e) = tx.send(Message::Text(welcome_msg.to_string().into())).await {
                        println!("Error sending welcome message: {}", e);
                    }
                    
                    // Physics update loop at 100 Hz (10ms interval) to match TS
                    let mut phy_interval = tokio::time::interval(Duration::from_millis(10)); 
                    let mut last_time = Instant::now();
                    // target_change_timer and arelated logic removed
                    
                    // Initial target setting removed, drone will just wander
                    
                    loop {
                        tokio::select! {
                            // Physics tick for simulation
                            _ = phy_interval.tick() => {
                                let now = Instant::now();
                                let dt = now.duration_since(last_time).as_secs_f64();
                                last_time = now;
                                
                                // Physics update
                                drone_physics.update(dt, &mut rng); // Pass rng
                                
                                // Target changing logic removed
                                
                                // Send GPS update on every physics tick (10ms)
                                let gps_update = json!({
                                    "type": "gps",
                                    "drone_id": drone_id,
                                    "latitude": drone_physics.lat,
                                    "longitude": drone_physics.lng,
                                    "altitude": drone_physics.alt,
                                    "velocity": {
                                        "lat": drone_physics.vel_lat,
                                        "lng": drone_physics.vel_lng,
                                        "alt": drone_physics.vel_alt // This will be 0.0
                                    },
                                    "acceleration": { // Accelerations are effectively zero in this model
                                        "lat": 0.0,
                                        "lng": 0.0,
                                        "alt": 0.0
                                    },
                                    "target": { // Target is now current position
                                        "lat": drone_physics.target_lat,
                                        "lng": drone_physics.target_lng,
                                        "alt": drone_physics.target_alt
                                    },
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                });
                                if let Err(e) = tx.send(Message::Text(gps_update.to_string().into())).await {
                                    println!("Error sending GPS update: {}", e);
                                    break; // Exit loop on send error
                                }
                            }
                            
                            // Обрабатываем входящие сообщения
                            message = rx.next() => {
                                sleep(Duration::from_millis(100)).await;
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
                                                        "set_target" => {
                                                            // Allow external target setting
                                                            if let (Some(lat), Some(lng)) = (
                                                                json.get("latitude").and_then(|v| v.as_f64()),
                                                                json.get("longitude").and_then(|v| v.as_f64())
                                                            ) {
                                                                drone_physics.target_lat = lat;
                                                                drone_physics.target_lng = lng;
                                                                if let Some(alt) = json.get("altitude").and_then(|v| v.as_f64()) {
                                                                    drone_physics.target_alt = alt;
                                                                }
                                                                // target_change_timer = 0.0; // This timer is removed
                                                                println!("Target set via command to: ({}, {}, {}) (Note: TS-style trajectory may override this behavior)", 
                                                                    drone_physics.target_lat, 
                                                                    drone_physics.target_lng, 
                                                                    drone_physics.target_alt);
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
