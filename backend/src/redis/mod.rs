use redis::{Client, RedisResult, AsyncCommands};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisGpsData {
    pub id: String,
    pub video_id: String,
    pub longitude: f64,
    pub latitude: f64,
    pub title: String,
    pub created_at: String,
}

pub struct RedisClient {
    client: Client,
    ttl_seconds: u64,
}

impl RedisClient {
    pub fn new(redis_url: &str, ttl_seconds: u64) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(RedisClient {
            client,
            ttl_seconds,
        })
    }

    /// Сохранить GPS данные в Redis с автоматическим истечением срока действия
    pub async fn save_gps_data(
        &self,
        video_id: String,
        longitude: f64,
        latitude: f64,
        title: String,
    ) -> RedisResult<RedisGpsData> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let created_at = now.to_rfc3339();
        
        let gps_data = RedisGpsData {
            id: id.clone(),
            video_id: video_id.clone(),
            longitude,
            latitude,
            title,
            created_at,
        };
        
        let json_data = serde_json::to_string(&gps_data)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;
        // Сохраняем с TTL
        let key = format!("gps:{}:{}", video_id, id);
        let _: () = conn.set_ex(&key, &json_data, self.ttl_seconds).await?;
        
        // Также сохраняем в индекс по video_id для быстрого поиска
        let index_key = format!("gps_index:{}", video_id);
        let _: () = conn.zadd(&index_key, &key, now.timestamp()).await?;
        conn.expire(&index_key, self.ttl_seconds as i64).await?;
        
        info!(
            gps_data_id = %id,
            video_id = %video_id,
            ttl = self.ttl_seconds,
            "GPS data saved to Redis"
        );
        
        Ok(gps_data)
    }

    /// Получить последние GPS данные для конкретного дрона
    pub async fn get_latest_gps_data(&self, video_id: String) -> RedisResult<Option<RedisGpsData>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        
        let index_key = format!("gps_index:{}", video_id);
        
        // Получаем последнюю запись из sorted set
        let results: Vec<String> = conn.zrevrange(&index_key, 0, 0).await?;
        
        if results.is_empty() {
            return Ok(None);
        }
        
        let key = &results[0];
        let json_data: Option<String> = conn.get(key).await?;
        
        match json_data {
            Some(data) => {
                let gps_data: RedisGpsData = serde_json::from_str(&data)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON deserialization failed", e.to_string())))?;
                Ok(Some(gps_data))
            }
            None => Ok(None),
        }
    }

    /// Получить все последние GPS данные для всех дронов
    pub async fn get_all_latest_gps_data(&self) -> RedisResult<Vec<RedisGpsData>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        
        // Получаем все ключи индексов
        let index_keys: Vec<String> = conn.keys("gps_index:*").await?;
        let mut all_gps_data = Vec::new();
        
        for index_key in index_keys {
            // Получаем последнюю запись для каждого дрона
            let results: Vec<String> = conn.zrevrange(&index_key, 0, 0).await?;
            
            if !results.is_empty() {
                let key = &results[0];
                let json_data: Option<String> = conn.get(key).await?;
                
                if let Some(data) = json_data {
                    if let Ok(gps_data) = serde_json::from_str::<RedisGpsData>(&data) {
                        all_gps_data.push(gps_data);
                    }
                }
            }
        }
        
        // Сортируем по времени создания (по убыванию)
        all_gps_data.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        info!(count = all_gps_data.len(), "Retrieved all latest GPS data from Redis");
        Ok(all_gps_data)
    }

    pub async fn ping(&self) -> RedisResult<()> {
        // Проверяем соединение с Redis
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.ping().await?;
        Ok(())
    }
}
