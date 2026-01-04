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

#[tauri::command]
async fn list_keys(config: RedisConfig) -> Result<Vec<String>, String> {
    let mut con = config.get_connection().await?;
    let keys: Vec<String> = con
        .keys("*")
        .await
        .map_err(|e| format!("Failed to list keys: {}", e))?;
    Ok(keys)
}

#[tauri::command]
async fn get_key_value(config: RedisConfig, key: String) -> Result<RedisValue, String> {
    let mut con = config.get_connection().await?;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            connect_redis,
            list_keys,
            get_key_value
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
