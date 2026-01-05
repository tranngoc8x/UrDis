use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "enableSSL")]
    pub enable_ssl: bool,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RedisValue {
    String(String),
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(HashMap<String, String>),
    None,
}

impl RedisConfig {
    fn to_url(&self) -> String {
        let protocol = if self.enable_ssl { "rediss" } else { "redis" };
        let auth = match (&self.username, &self.password) {
            (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => format!("{}:{}@", u, p),
            (None, Some(p)) if !p.is_empty() => format!(":{}@", p),
            (Some(u), None) if !u.is_empty() => format!("{}@", u),
            _ => "".to_string(),
        };
        format!("{}://{}{}:{}", protocol, auth, self.host, self.port)
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, String> {
        let client = redis::Client::open(self.to_url())
            .map_err(|e| format!("Failed to create Redis client: {}", e))?;
        client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Connection failed: {}", e))
    }
}

#[tauri::command]
async fn connect_redis(config: RedisConfig) -> Result<String, String> {
    let mut con = config.get_connection().await?;
    let response: String = redis::cmd("PING")
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to execute PING: {}", e))?;

    if response == "PONG" {
        Ok("Successfully connected to Redis server!".to_string())
    } else {
        Ok(format!("Unexpected response: {}", response))
    }
}

#[derive(Serialize)]
pub struct RedisKeyInfo {
    pub name: String,
    pub key_type: String,
}

#[tauri::command]
async fn list_keys(
    config: RedisConfig,
    db: i64,
    cursor: u64,
    pattern: String,
) -> Result<(u64, Vec<RedisKeyInfo>), String> {
    let mut con = config.get_connection().await?;

    // Select DB
    let _: () = redis::cmd("SELECT")
        .arg(db)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to select DB {}: {}", db, e))?;

    let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
        .arg(cursor)
        .arg("MATCH")
        .arg(if pattern.is_empty() { "*" } else { &pattern })
        .arg("COUNT")
        .arg(300)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("SCAN error: {}", e))?;

    let mut keys = Vec::new();
    if !batch.is_empty() {
        // Use pipeline to get types for all keys in batch
        let mut pipe = redis::pipe();
        for key in &batch {
            pipe.cmd("TYPE").arg(key);
        }

        let types: Vec<String> = pipe
            .query_async(&mut con)
            .await
            .map_err(|e| format!("Pipeline TYPE error: {}", e))?;

        for (name, key_type) in batch.into_iter().zip(types.into_iter()) {
            keys.push(RedisKeyInfo { name, key_type });
        }
    }

    Ok((next_cursor, keys))
}

#[tauri::command]
async fn get_key_value(config: RedisConfig, key: String, db: i64) -> Result<RedisValue, String> {
    let mut con = config.get_connection().await?;

    // Select DB
    let _: () = redis::cmd("SELECT")
        .arg(db)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to select DB {}: {}", db, e))?;

    let key_type: String = redis::cmd("TYPE")
        .arg(&key)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to get key type: {}", e))?;

    match key_type.as_str() {
        "string" => {
            let val: String = con.get(&key).await.map_err(|e| e.to_string())?;
            Ok(RedisValue::String(val))
        }
        "list" => {
            let val: Vec<String> = con.lrange(&key, 0, -1).await.map_err(|e| e.to_string())?;
            Ok(RedisValue::List(val))
        }
        "set" => {
            let val: Vec<String> = con.smembers(&key).await.map_err(|e| e.to_string())?;
            Ok(RedisValue::Set(val))
        }
        "zset" => {
            let val: Vec<(String, f64)> = con
                .zrange_withscores(&key, 0, -1)
                .await
                .map_err(|e| e.to_string())?;
            Ok(RedisValue::ZSet(val))
        }
        "hash" => {
            let val: HashMap<String, String> =
                con.hgetall(&key).await.map_err(|e| e.to_string())?;
            Ok(RedisValue::Hash(val))
        }
        _ => Ok(RedisValue::None),
    }
}

#[tauri::command]
async fn get_db_sizes(config: RedisConfig) -> Result<Vec<i64>, String> {
    let mut con = config.get_connection().await?;
    let mut sizes: Vec<i64> = Vec::with_capacity(16);

    for db in 0..16 {
        // Select DB
        let _: () = redis::cmd("SELECT")
            .arg(db)
            .query_async(&mut con)
            .await
            .map_err(|e| format!("Failed to select DB {}: {}", db, e))?;

        // Get DBSIZE
        let size: i64 = redis::cmd("DBSIZE")
            .query_async(&mut con)
            .await
            .map_err(|e| format!("Failed to get size for DB {}: {}", db, e))?;

        sizes.push(size);
    }

    // Reset to DB 0
    let _: () = redis::cmd("SELECT")
        .arg(0)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to reset to DB 0: {}", e))?;

    Ok(sizes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            connect_redis,
            list_keys,
            get_key_value,
            get_db_sizes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
