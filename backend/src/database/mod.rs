use sqlx::{Pool, MySql, query, query_as};
use uuid::Uuid;
use chrono::Utc;

use crate::models::Video;

pub async fn get_videos(pool: &Pool<MySql>) -> Result<Vec<Video>, sqlx::Error> {
    // Using dynamic query instead of macro to avoid compile-time DB connection requirement
    let videos = query_as::<_, Video>(
        r#"
        SELECT id, url, title, thumbnail, created_at as `createdAt`
        FROM videos
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(videos)
}

pub async fn add_video(
    pool: &Pool<MySql>,
    url: String,
    title: String,
) -> Result<Video, sqlx::Error> {
    // Use the proper UUID builder method with the v4 feature
    let id = Uuid::new_v4().to_string();    
    let now = Utc::now();
    let created_at = now.to_rfc3339();
    
    // Placeholder for thumbnail
    let thumbnail = "".to_string();

    // Using dynamic query
    query(
        r#"
        INSERT INTO videos (id, url, title, thumbnail, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&url)
    .bind(&title)
    .bind(&thumbnail)
    .bind(&created_at)
    .execute(pool)
    .await?;

    // Fetch the newly inserted record
    let video = query_as::<_, Video>(
        r#"
        SELECT id, url, title, thumbnail, created_at as `createdAt`
        FROM videos
        WHERE id = ?
        "#
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(video)
}

pub async fn delete_video(pool: &Pool<MySql>, id: String) -> Result<bool, sqlx::Error> {
    // Using dynamic query
    let result = query(
        r#"
        DELETE FROM videos
        WHERE id = ?
        "#
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
