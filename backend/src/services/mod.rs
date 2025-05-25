use sqlx::{Pool, MySql}; // Changed from Postgres to MySql
use std::sync::Arc;
use crate::config::Config;
use crate::models::{Video, Feed};
use crate::database;
use crate::redis::{RedisClient, RedisGpsData};
use tokio::time::Duration;
use std::io::{Error, ErrorKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::task::{JoinHandle, AbortHandle};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tokio::sync::broadcast;
pub mod drone_client;
use uuid::Uuid; // Add this if not already present

/// Connection manager to track active drone WebSocket connections
#[derive(Debug)]
struct DroneConnectionManager {
    active_connections: HashMap<String, AbortHandle>,
}

impl DroneConnectionManager {
    fn new() -> Self {
        DroneConnectionManager {
            active_connections: HashMap::new(),
        }
    }
    
    fn add_connection(&mut self, drone_id: String, abort_handle: AbortHandle) {
        // If there's an existing connection, abort it first
        if let Some(old_handle) = self.active_connections.remove(&drone_id) {
            old_handle.abort();
            tracing::info!(drone_id = %drone_id, "Aborted existing drone connection");
        }
        self.active_connections.insert(drone_id.clone(), abort_handle);
        tracing::info!(drone_id = %drone_id, "Added new drone connection");
    }
    
    fn remove_connection(&mut self, drone_id: &str) -> bool {
        if let Some(handle) = self.active_connections.remove(drone_id) {
            handle.abort();
            tracing::info!(drone_id = %drone_id, "Removed and aborted drone connection");
            true
        } else {
            false
        }
    }
    
    fn is_connected(&self, drone_id: &str) -> bool {
        self.active_connections.contains_key(drone_id)
    }
    
    fn get_active_connections(&self) -> HashSet<String> {
        self.active_connections.keys().cloned().collect()
    }
}

// Global drone connection manager
lazy_static::lazy_static! {
    static ref DRONE_CONNECTIONS: Mutex<DroneConnectionManager> = Mutex::new(DroneConnectionManager::new());
}

/// Task manager to keep track of running thumbnail update tasks
struct ThumbnailTaskManager {
    tasks: HashMap<String, JoinHandle<()>>,
}

impl ThumbnailTaskManager {
    fn new() -> Self {
        ThumbnailTaskManager {
            tasks: HashMap::new(),
        }
    }
    
    fn add_task(&mut self, video_id: String, handle: JoinHandle<()>) {
        // If there's an existing task, abort it first
        if let Some(task) = self.tasks.remove(&video_id) {
            task.abort();
            tracing::info!(video_id = %video_id, "Aborted existing thumbnail task");
        }
        self.tasks.insert(video_id.clone(), handle);
        tracing::info!(video_id = %video_id, "Added new thumbnail task");
    }
    
    fn remove_task(&mut self, video_id: &str) -> bool {
        if let Some(task) = self.tasks.remove(video_id) {
            task.abort();
            tracing::info!(video_id = %video_id, "Removed and aborted thumbnail task");
            true
        } else {
            false
        }
    }
}

// Global thumbnail task manager
lazy_static::lazy_static! {
    static ref THUMBNAIL_TASKS: Mutex<ThumbnailTaskManager> = Mutex::new(ThumbnailTaskManager::new());
}


// Global channel for GPS updates - можно получать последние обновления GPS для всех дронов
lazy_static::lazy_static! {
    pub static ref GPS_UPDATES: broadcast::Sender<RedisGpsData> = {
        let (sender, _) = broadcast::channel(100); // Буфер на 100 сообщений
        sender
    };
}

pub struct AppState {
    pub db: Pool<MySql>, // Changed from Postgres to MySql
    pub config: Config,
    pub redis: RedisClient,
}

pub async fn get_feed(state: Arc<AppState>) -> Result<Feed, sqlx::Error> {
    // Log entering service
    tracing::info!("services::get_feed called");
    let videos = database::get_videos(&state.db).await?;
    tracing::info!(count = videos.len(), "services::get_feed succeeded");
    Ok(Feed { videos })
}

/// Capture a single screenshot (JPG) from the RTMP stream using ffmpeg
async fn capture_screenshot(source_url: &str, quality: u32) -> Result<Vec<u8>, Error> {
    // Use ffmpeg to capture one frame as JPG to stdout with specified quality
    // JPG is more size-efficient than PNG for thumbnails
    let output = tokio::process::Command::new("ffmpeg")
        .arg("-y")                   // Overwrite output files
        .arg("-i")
        .arg(source_url)
        .arg("-vframes")
        .arg("1")                    // Capture one frame
        .arg("-q:v")
        .arg(quality.to_string())    // Set quality (2-31, lower is better)
        .arg("-vf")
        .arg("scale=480:-1")         // Resize to 480px width while keeping aspect ratio
        .arg("-f")
        .arg("image2")               // Output format
        .arg("-c:v")
        .arg("mjpeg")                // Use JPEG codec
        .arg("pipe:1")               // Output to stdout
        .output()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(Error::new(
            ErrorKind::Other,
            format!("ffmpeg failed: {}", String::from_utf8_lossy(&output.stderr)),
        ))
    }
}

pub async fn add_drone(
    state: Arc<AppState>,
    title: String,
    rtmp_url: String,
    ws_url: Option<String>,
    drone_id_override: Option<String>, // Added: Optional ID override
) -> Result<Video, sqlx::Error> {
    let id_to_use = drone_id_override.unwrap_or_else(|| Uuid::new_v4().to_string()); // Use override or generate
    // Pass id_to_use to database::add_video
    let video = database::add_video(&state.db, id_to_use.clone(), title.clone(), rtmp_url.clone(), ws_url.clone()).await?;
    tracing::info!(video_id = %video.id, "services::add_drone database::add_video succeeded");
    // Then set up the RTMP relay
    let source_url = rtmp_url.clone();
    let destination_url = state.config.media_server_url.clone() + "/" + &video.id;
    // Start relaying RTMP stream
    let relay_added = crate::rtmp::add_rtmp_relay(video.id.clone(), source_url, destination_url);
    tracing::info!(video_id = %video.id, relay_added = %relay_added, "services::add_drone rtmp::add_rtmp_relay result");
      // Spawn periodic thumbnail capture task
    {
        let video_id_clone = video.id.clone();
        let rtmp_url_clone = rtmp_url.clone(); // Use the original rtmp_url for thumbnails
        let app_state_clone = state.clone();
        let task_handle = tokio::spawn(async move {
            // Initial delay to allow stream to stabilize
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                match capture_screenshot(&rtmp_url_clone, 5).await { // Use rtmp_url_clone
                    Ok(image_data) => {
                        let b64_image = STANDARD.encode(&image_data);
                        let thumbnail_data = format!("data:image/jpeg;base64,{}", b64_image);
                        if let Err(e) = database::update_thumbnail(&app_state_clone.db, &video_id_clone, &thumbnail_data).await {
                            tracing::error!(video_id = %video_id_clone, error = %e, "Failed to update thumbnail in DB");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(video_id = %video_id_clone, error = %e, "Failed to capture screenshot");
                    }
                }
                // Wait for 10 seconds before next capture
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
        // Store the task handle
        if let Ok(mut task_manager) = THUMBNAIL_TASKS.lock() {
            task_manager.add_task(video.id.clone(), task_handle);
        }
    }
    Ok(video)
}

pub async fn delete_drone(
    state: Arc<AppState>,
    id: String,
) -> Result<bool, sqlx::Error> {
    // Log entering service
    tracing::info!(drone_id = %id, "services::delete_drone called");
    
    // First stop the thumbnail update task
    let task_removed = if let Ok(mut task_manager) = THUMBNAIL_TASKS.lock() {
        task_manager.remove_task(&id)
    } else {
        tracing::error!("Failed to acquire task manager lock");
        false
    };
    tracing::info!(drone_id = %id, task_removed = %task_removed, "services::delete_drone thumbnail task removal result");
    
    // Then stop the RTMP relay
    let relay_removed = crate::rtmp::remove_rtmp_relay(&id);
    tracing::info!(drone_id = %id, relay_removed = %relay_removed, "services::delete_drone rtmp::remove_rtmp_relay result");
    
    // Finally delete from the database
    let deleted = database::delete_video(&state.db, id.clone()).await?;
    tracing::info!(drone_id = %id, deleted = %deleted, "services::delete_drone database::delete_video result");
    Ok(deleted)
}

pub async fn get_drone_by_id(
    state: Arc<AppState>,
    id: String,
) -> Result<Option<Video>, sqlx::Error> {
    tracing::info!(drone_id = %id, "services::get_drone_by_id called");
    let result = database::get_video_by_id(&state.db, id.clone()).await?;
    tracing::info!(drone_id = %id, found = result.is_some(), "services::get_drone_by_id database::get_video_by_id result");
    Ok(result)
}

pub async fn save_drone_gps_data(
    state: Arc<AppState>,
    drone_id: String,
    latitude: f64,
    longitude: f64,
    _altitude: f64, // Оставляем для совместимости, но не используем в Redis
) -> Result<RedisGpsData, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        drone_id = %drone_id,
        latitude = %latitude,
        longitude = %longitude,
        "services::save_drone_gps_data called (Redis version)"
    );
    
    // Проверяем, существует ли дрон
    let drone = database::get_video_by_id(&state.db, drone_id.clone()).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    if let Some(video) = drone {
        let gps_data = state.redis.save_gps_data(
            drone_id,
            longitude,
            latitude,
            video.title        ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Отправляем обновление всем подписчикам
        let _ = GPS_UPDATES.send(gps_data.clone());
        
        tracing::info!(
            gps_data_id = %gps_data.id,
            "services::save_drone_gps_data succeeded (Redis version)"
        );
        
        Ok(gps_data)
    } else {
        Err("Drone not found".into())
    }
}

pub async fn get_drone_gps_data(
    state: Arc<AppState>,
    drone_id: String,
) -> Result<Option<RedisGpsData>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        drone_id = %drone_id,
        "services::get_drone_gps_data called (Redis version)"
    );
    
    let gps_data = state.redis.get_latest_gps_data(drone_id).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    tracing::info!(
        found = gps_data.is_some(),
        "services::get_drone_gps_data succeeded (Redis version)"
    );
    
    Ok(gps_data)
}

pub async fn get_all_drones_gps_data(
    state: Arc<AppState>,
) -> Result<Vec<RedisGpsData>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("services::get_all_drones_gps_data called (Redis version)");
    
    let all_gps_data = state.redis.get_all_latest_gps_data().await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    tracing::info!(
        count = all_gps_data.len(),
        "services::get_all_drones_gps_data succeeded (Redis version)"
    );
      Ok(all_gps_data)
}

/// Revive a drone connection if it's not already active
pub async fn revive_drone_connection(
    state: Arc<AppState>,
    drone_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(drone_id = %drone_id, "services::revive_drone_connection called");
    
    // Check if drone is already connected
    {
        let connection_manager = DRONE_CONNECTIONS.lock().unwrap();
        if connection_manager.is_connected(&drone_id) {
            tracing::info!(drone_id = %drone_id, "Drone connection already active");
            return Ok(());
        }
    }
    
    // Get drone information from database
    let drone = database::get_video_by_id(&state.db, drone_id.clone()).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    let drone = match drone {
        Some(d) => d,
        None => {
            tracing::error!(drone_id = %drone_id, "Drone not found in database");
            return Err("Drone not found".into());
        }
    };
    
    // Check if drone has a WebSocket URL
    let ws_url = match drone.ws_url.as_ref().filter(|url| !url.trim().is_empty()) {
        Some(url) => url.clone(),
        None => {
            tracing::warn!(drone_id = %drone_id, "No WebSocket URL configured for drone");
            return Ok(()); // Not an error, just no connection to establish
        }
    };
    
    tracing::info!(drone_id = %drone_id, ws_url = %ws_url, "Starting drone connection revival");
    
    // Start the connection in a background task
    let state_clone = state.clone();
    let drone_id_clone = drone_id.clone();
    let ws_url_clone = ws_url.clone();
    
    let connection_task = tokio::spawn(async move {
        match drone_client::connect_to_drone(state_clone, drone_id_clone.clone(), ws_url_clone).await {
            Ok(_) => tracing::info!(drone_id = %drone_id_clone, "Drone connection revival completed"),
            Err(e) => tracing::error!(drone_id = %drone_id_clone, error = %e, "Drone connection revival failed"),
        }
    });
    
    // Track the connection
    {
        let mut connection_manager = DRONE_CONNECTIONS.lock().unwrap();
        connection_manager.add_connection(drone_id.clone(), connection_task.abort_handle());
    }
    
    tracing::info!(drone_id = %drone_id, "Drone connection revival initiated");
    Ok(())
}

/// Get current connection status for a drone
pub fn get_drone_connection_status(drone_id: &str) -> bool {
    let connection_manager = DRONE_CONNECTIONS.lock().unwrap();
    connection_manager.is_connected(drone_id)
}

/// Get all active drone connections
pub fn get_active_drone_connections() -> HashSet<String> {
    let connection_manager = DRONE_CONNECTIONS.lock().unwrap();
    connection_manager.get_active_connections()
}
