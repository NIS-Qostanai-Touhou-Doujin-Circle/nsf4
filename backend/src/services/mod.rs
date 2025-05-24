use sqlx::{Pool, MySql}; // Changed from Postgres to MySql
use std::sync::Arc;
use crate::config::Config;
use crate::models::{Video, Feed};
use crate::database;
// For periodic screenshot capture and base64 encoding
use tokio::process::Command;
use tokio::time::Duration;
use std::io::{Error, ErrorKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::task::JoinHandle;
use std::collections::HashMap;
use std::sync::Mutex;

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

pub struct AppState {
    pub db: Pool<MySql>, // Changed from Postgres to MySql
    pub config: Config,
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
    let output = Command::new("ffmpeg")
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
    url: String,
    title: String,
) -> Result<Video, sqlx::Error> {
    // Log entering service
    tracing::info!(url = %url, title = %title, "services::add_drone called");
    // First add the drone to the database
    let video = database::add_video(&state.db, url.clone(), title.clone()).await?;
    tracing::info!(video_id = %video.id, "services::add_drone database::add_video succeeded");
    // Then set up the RTMP relay
    let destination_url = state.config.media_server_url.clone() + "/" + &video.id;
    // Start relaying RTMP stream
    let relay_added = crate::rtmp::add_rtmp_relay(video.id.clone(), url, destination_url);
    tracing::info!(video_id = %video.id, relay_added = %relay_added, "services::add_drone rtmp::add_rtmp_relay result");
    
    // Spawn periodic thumbnail capture task
    {
        let video_id = video.id.clone();
        let source_url = video.url.clone();
        let db_pool = state.db.clone();
        let interval_secs = state.config.screenshot_interval_seconds;
        let quality = state.config.screenshot_quality;
        
        // Create the thumbnail update task
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                match capture_screenshot(&source_url, quality).await {
                    Ok(jpg) => {
                        let b64 = STANDARD.encode(&jpg);
                        let data_uri = format!("data:image/jpeg;base64,{}", b64);
                        if let Err(e) = database::update_thumbnail(&db_pool, &video_id, &data_uri).await {
                            tracing::error!(error = %e, video_id = %video_id, "Failed updating thumbnail");
                        } else {
                            tracing::debug!(video_id = %video_id, "Thumbnail updated successfully");
                        }
                    }
                    Err(e) => tracing::warn!(video_id = %video_id, error = %e, "Screenshot capture failed"),
                }
            }
        });
        
        // Register the task with the task manager
        if let Ok(mut task_manager) = THUMBNAIL_TASKS.lock() {
            task_manager.add_task(video.id.clone(), task);
        } else {
            tracing::error!(video_id = %video.id, "Failed to register thumbnail task");
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
