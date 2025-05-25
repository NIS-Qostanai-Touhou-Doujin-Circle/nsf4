use sqlx::{Pool, MySql, query, query_as};
use tracing::info;
use chrono::Utc;
use crate::models::{Video};
use base64::{engine::general_purpose, Engine as _};
use uuid::Uuid; // Added for generating ID

pub async fn get_videos(pool: &Pool<MySql>) -> Result<Vec<Video>, sqlx::Error> {
    info!("database::get_videos called");
    // Using dynamic query instead of macro to avoid compile-time DB connection requirement
    let videos = query_as::<_, Video>(
        r#"
        SELECT id, title, thumbnail, created_at, rtmp_url, ws_url
        FROM videos
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    let count = videos.len();
    info!(count = count, "database::get_videos succeeded");
    Ok(videos)
}

pub async fn get_video_analytics_by_id(
    pool: &Pool<MySql>,
    video_id: String,
) -> Result<Vec<(String, i32)>, sqlx::Error> {
    info!(video_id = &video_id, "database::get_video_analytics_by_id called");
    // Using dynamic query instead of macro to avoid compile-time DB connection requirement
    let analytics = query_as::<_, (String, i32)>(
        r#"
        SELECT created_at, bitrate
        FROM video_analytics
        WHERE video_id = ?
        ORDER BY created_at DESC
        "#
    )
    .bind(&video_id)
    .fetch_all(pool)
    .await?;

    info!(video_id = &video_id, count = analytics.len(), "database::get_video_analytics_by_id succeeded");
    Ok(analytics)
}

pub async fn get_videos_count(pool: &Pool<MySql>) -> Result<usize, sqlx::Error> {
    info!("database::get_videos_count called");
    // Using dynamic query instead of macro to avoid compile-time DB connection requirement
    let count: (i64,) = query_as(
        r#"
        SELECT COUNT(*) FROM videos
        "#
    )
    .fetch_one(pool)
    .await?;

    let count = count.0 as usize;
    info!(count = count, "database::get_videos_count succeeded");
    Ok(count)
}

// Extracts the first frame of the video at source_url as a base64-encoded PNG
async fn extract_thumbnail(source_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Use ffmpeg to capture the first frame image to stdout
    let output = tokio::process::Command::new("ffmpeg")
        .args(&[
            "-i",
            source_url,
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-vcodec",
            "png",
            "pipe:1",
        ])
        .output()
        .await?;
    if !output.status.success() {
        return Err(format!("ffmpeg exited with status: {}", output.status).into());
    }
    // Use the standard general_purpose engine for base64 encoding
    let b64 = general_purpose::STANDARD.encode(&output.stdout);
    Ok(format!("data:image/png;base64,{}", b64))
}

pub async fn get_video_by_id(pool: &Pool<MySql>, id: String) -> Result<Option<Video>, sqlx::Error> {
    // Log and borrow id to avoid moving    info!(video_id = &id, "database::get_video_by_id called");    // Using dynamic query
    let video = query_as::<_, Video>(
        r#"
        SELECT id, title, thumbnail, created_at, rtmp_url, ws_url
        FROM videos
        WHERE id = ?
        "#
    )
    .bind(&id)
    .fetch_optional(pool)
    .await?;

    let found = video.is_some();
    info!(video_id = &id, found = found, "database::get_video_by_id succeeded");
    Ok(video)
}

pub async fn add_video(
    pool: &Pool<MySql>,
    id: String, // Changed: Accept ID as a parameter
    title: String,
    rtmp_url: String,
    ws_url: Option<String>,
) -> Result<Video, sqlx::Error> {
    // Removed: let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let created_at = now.to_rfc3339();
    // Extract thumbnail from the source URL
    let thumbnail = match extract_thumbnail(&rtmp_url).await {
        Ok(b64) => b64,
        Err(e) => {
            info!(error = %e, "Failed to extract thumbnail, using empty string");
            String::new()
        }
    };// Using dynamic query
    query(
        r#"
        INSERT INTO videos (id, title, thumbnail, created_at, rtmp_url, ws_url)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&rtmp_url)  // url field should be set to rtmp_url
    .bind(&title)
    .bind(&thumbnail)
    .bind(&created_at)
    .bind(&rtmp_url)
    .bind(&ws_url)
    .bind(&title) 
    .execute(pool)
    .await?;// Fetch the newly inserted record
    let video = query_as::<_, Video>(
        r#"
        SELECT id, title, thumbnail, created_at, rtmp_url, ws_url
        FROM videos
        WHERE id = ?
        "#
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    info!(video_id = %id, "database::add_video succeeded");
    Ok(video)
}

pub async fn delete_video(pool: &Pool<MySql>, id: String) -> Result<bool, sqlx::Error> {
    // Log and borrow id to avoid moving
    info!(video_id = &id, "database::delete_video called");
    // Using dynamic query
    let result = query(
        r#"
        DELETE FROM videos
        WHERE id = ?
        "#
    )
    .bind(&id)
    .execute(pool)
    .await?;

    let deleted = result.rows_affected() > 0;
    info!(video_id = &id, deleted = deleted, "database::delete_video succeeded");
    Ok(deleted)
}
/// Update the thumbnail data for a video
pub async fn update_thumbnail(
    pool: &Pool<MySql>,
    id: &str,
    thumbnail: &str,
) -> Result<(), sqlx::Error> {
    // Log the update attempt
    info!(video_id = id, "database::update_thumbnail called");
    
    // Calculate size in KB for logging (might be useful for debugging large thumbnails)
    let size_kb = thumbnail.len() / 1024;
    
    // Update thumbnail field with base64 image data
    query(
        "UPDATE videos SET thumbnail = ? WHERE id = ?"
    )
    .bind(thumbnail)
    .bind(id)
    .execute(pool)
    .await?;
    
    info!(video_id = id, size_kb = size_kb, "database::update_thumbnail succeeded");
    Ok(())
}

pub async fn add_video_analytics(
    pool: &Pool<MySql>,
    video_id: String,
    bitrate: i32, // in kbit/s
    // resolution: String, // Placeholder for future implementation
    // frame_rate: i32,    // Placeholder for future implementation
    // error_rate: f32,    // Placeholder for future implementation
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    // Placeholder values for fields not yet parsed from ffmpeg
    let resolution = "N/A".to_string();
    let frame_rate = 0;
    let error_rate = 0.0;

    info!(video_id = %video_id, bitrate = %bitrate, "database::add_video_analytics called");

    query(
        r#"
        INSERT INTO video_analytics (id, video_id, created_at, bitrate, resolution, frame_rate, error_rate)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(id)
    .bind(video_id)
    .bind(created_at)
    .bind(bitrate)
    .bind(resolution)
    .bind(frame_rate)
    .bind(error_rate)
    .execute(pool)
    .await?;

    Ok(())
}