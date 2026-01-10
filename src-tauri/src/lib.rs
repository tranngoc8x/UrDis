use base64::{engine::general_purpose, Engine as _};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use tokio::sync::Mutex;

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
    Binary(String), // Base64 encoded
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
}

#[derive(Default)]
pub struct ConnectionManager {
    state: Mutex<ConnectionState>,
}

#[derive(Default)]
struct ConnectionState {
    connections: HashMap<i64, redis::aio::MultiplexedConnection>,
    config: Option<RedisConfig>,
}

impl ConnectionManager {
    async fn get_connection(
        &self,
        new_config: &RedisConfig,
        db: i64,
    ) -> Result<redis::aio::MultiplexedConnection, String> {
        let mut state = self.state.lock().await;

        let config_changed = match &state.config {
            Some(existing) => {
                existing.host != new_config.host
                    || existing.port != new_config.port
                    || existing.username != new_config.username
                    || existing.password != new_config.password
                    || existing.enable_ssl != new_config.enable_ssl
            }
            None => true,
        };

        if config_changed {
            state.connections.clear();
            state.config = Some(new_config.clone());
        }

        if let Some(conn) = state.connections.get(&db) {
            return Ok(conn.clone());
        }

        // Create new connection for this DB
        let client = redis::Client::open(new_config.to_url())
            .map_err(|e| format!("Failed to create Redis client: {}", e))?;

        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        // SELECT the DB immediately for this connection
        let _: () = redis::cmd("SELECT")
            .arg(db)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Failed to select DB {}: {}", db, e))?;

        state.connections.insert(db, conn.clone());
        Ok(conn)
    }
}

#[tauri::command]
async fn connect_redis(
    config: RedisConfig,
    state: State<'_, ConnectionManager>,
) -> Result<String, String> {
    let mut con = state.get_connection(&config, 0).await?;
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
    current_count: usize,
    state: State<'_, ConnectionManager>,
) -> Result<(u64, Vec<RedisKeyInfo>), String> {
    let mut con = state.get_connection(&config, db).await?;

    // Dynamic COUNT only for pattern search (not for listing all keys)
    let has_pattern = !pattern.is_empty() && pattern != "*";

    let scan_count = if !has_pattern {
        // Listing all keys: use fixed COUNT
        1000
    } else if current_count == 0 {
        // First fetch with pattern
        1000
    } else if current_count < 10 {
        // Very few keys found, increase significantly
        10000
    } else if current_count < 50 {
        // Few keys found, increase moderately
        5000
    } else {
        // Enough keys found, keep stable
        3000
    };

    let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
        .arg(cursor)
        .arg("MATCH")
        .arg(if pattern.is_empty() { "*" } else { &pattern })
        .arg("COUNT")
        .arg(scan_count)
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

#[derive(Serialize)]
pub struct RedisKeyData {
    pub key_type: String,
    pub value: RedisValue,
    pub ttl: i64,
    pub memory: i64,
    pub encoding: String,
}

fn format_redis_bytes(bytes: Vec<u8>) -> (String, bool) {
    match String::from_utf8(bytes) {
        Ok(s) => (s, true),
        Err(e) => (general_purpose::STANDARD.encode(e.into_bytes()), false),
    }
}

// Updated formatters for other types
fn format_redis_bytes_list(bytes_list: Vec<Vec<u8>>) -> Vec<String> {
    bytes_list
        .into_iter()
        .map(|b| format_redis_bytes(b).0)
        .collect()
}

fn format_redis_bytes_zset(zset: Vec<(Vec<u8>, f64)>) -> Vec<(String, f64)> {
    zset.into_iter()
        .map(|(b, s)| (format_redis_bytes(b).0, s))
        .collect()
}

fn format_redis_bytes_hash(hash: HashMap<String, Vec<u8>>) -> HashMap<String, String> {
    hash.into_iter()
        .map(|(k, v)| (k, format_redis_bytes(v).0))
        .collect()
}

#[tauri::command]
async fn get_key_value(
    config: RedisConfig,
    key: String,
    db: i64,
    state: State<'_, ConnectionManager>,
) -> Result<RedisKeyData, String> {
    let mut con = state.get_connection(&config, db).await?;

    // 1. Get Key Type
    let key_type: String = redis::cmd("TYPE")
        .arg(&key)
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to get key type: {}", e))?;

    // 2. Get Metadata (TTL, Memory, Encoding) using pipeline
    let mut pipe = redis::pipe();
    pipe.cmd("TTL").arg(&key);
    pipe.cmd("MEMORY").arg("USAGE").arg(&key);
    pipe.cmd("OBJECT").arg("ENCODING").arg(&key);

    let metadata: (i64, Option<i64>, Option<String>) = pipe
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    let ttl = metadata.0;
    let memory = metadata.1.unwrap_or(0);
    let encoding = metadata.2.unwrap_or_else(|| "none".to_string());

    // 3. Get Value
    let value = match key_type.as_str() {
        "string" => {
            let val: Vec<u8> = con.get(&key).await.map_err(|e| e.to_string())?;
            let (s, is_utf8) = format_redis_bytes(val);
            if is_utf8 {
                RedisValue::String(s)
            } else {
                RedisValue::Binary(s)
            }
        }
        "list" => {
            let val: Vec<Vec<u8>> = con.lrange(&key, 0, -1).await.map_err(|e| e.to_string())?;
            RedisValue::List(format_redis_bytes_list(val))
        }
        "set" => {
            let val: Vec<Vec<u8>> = con.smembers(&key).await.map_err(|e| e.to_string())?;
            RedisValue::Set(format_redis_bytes_list(val))
        }
        "zset" => {
            let val: Vec<(Vec<u8>, f64)> = con
                .zrange_withscores(&key, 0, -1)
                .await
                .map_err(|e| e.to_string())?;
            RedisValue::ZSet(format_redis_bytes_zset(val))
        }
        "hash" => {
            let val: HashMap<String, Vec<u8>> =
                con.hgetall(&key).await.map_err(|e| e.to_string())?;
            RedisValue::Hash(format_redis_bytes_hash(val))
        }
        _ => RedisValue::None,
    };

    Ok(RedisKeyData {
        key_type,
        value,
        ttl,
        memory,
        encoding,
    })
}

#[tauri::command]
async fn get_batch_key_values(
    config: RedisConfig,
    keys: Vec<String>,
    db: i64,
    state: State<'_, ConnectionManager>,
) -> Result<Vec<RedisValue>, String> {
    let mut con = state.get_connection(&config, db).await?;

    if keys.is_empty() {
        return Ok(Vec::new());
    }

    // Pass 1: Get all types using pipeline
    let mut pipe = redis::pipe();
    for key in &keys {
        pipe.cmd("TYPE").arg(key);
    }
    let types: Vec<String> = pipe
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Pipeline Pass 1 (TYPE) failed: {}", e))?;

    // Pass 2: Get all values using pipeline
    let mut pipe = redis::pipe();
    for (key, key_type) in keys.iter().zip(types.iter()) {
        match key_type.as_str() {
            "string" => {
                pipe.cmd("GET").arg(key);
            }
            "list" => {
                pipe.cmd("LRANGE").arg(key).arg(0).arg(-1);
            }
            "set" => {
                pipe.cmd("SMEMBERS").arg(key);
            }
            "zset" => {
                pipe.cmd("ZRANGE").arg(key).arg(0).arg(-1).arg("WITHSCORES");
            }
            "hash" => {
                pipe.cmd("HGETALL").arg(key);
            }
            _ => {
                pipe.cmd("EXISTS").arg(key); // Dummy command to keep alignment
            }
        }
    }

    // Executing Pass 2
    let values: Vec<redis::Value> = pipe
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Pipeline Pass 2 (VALUE) failed: {}", e))?;

    // Map raw redis::Value back to our RedisValue enum
    let mut results = Vec::with_capacity(keys.len());
    for (val, key_type) in values.into_iter().zip(types.into_iter()) {
        let rv = match key_type.as_str() {
            "string" => val
                .as_sequence()
                .and_then(|s| s.get(0))
                .cloned()
                .or(Some(val.clone()))
                .and_then(|v| redis::from_redis_value::<Vec<u8>>(v).ok())
                .map(|v| {
                    let (s, is_utf8) = format_redis_bytes(v);
                    if is_utf8 {
                        RedisValue::String(s)
                    } else {
                        RedisValue::Binary(s)
                    }
                })
                .unwrap_or(RedisValue::None),
            "list" => redis::from_redis_value::<Vec<Vec<u8>>>(val)
                .map(|v| RedisValue::List(format_redis_bytes_list(v)))
                .unwrap_or(RedisValue::None),
            "set" => redis::from_redis_value::<Vec<Vec<u8>>>(val)
                .map(|v| RedisValue::Set(format_redis_bytes_list(v)))
                .unwrap_or(RedisValue::None),
            "zset" => redis::from_redis_value::<Vec<(Vec<u8>, f64)>>(val)
                .map(|v| RedisValue::ZSet(format_redis_bytes_zset(v)))
                .unwrap_or(RedisValue::None),
            "hash" => redis::from_redis_value::<HashMap<String, Vec<u8>>>(val)
                .map(|v| RedisValue::Hash(format_redis_bytes_hash(v)))
                .unwrap_or(RedisValue::None),
            _ => RedisValue::None,
        };
        results.push(rv);
    }

    Ok(results)
}

#[tauri::command]
async fn get_db_sizes(
    config: RedisConfig,
    state: State<'_, ConnectionManager>,
) -> Result<Vec<i64>, String> {
    let mut con = state.get_connection(&config, 0).await?;
    let mut sizes: Vec<i64> = vec![0; 16];

    let info: String = redis::cmd("INFO")
        .arg("keyspace")
        .query_async(&mut con)
        .await
        .map_err(|e| format!("Failed to get INFO keyspace: {}", e))?;

    // Parse INFO keyspace
    // format: # Keyspace\r\ndb0:keys=1,expires=0,avg_ttl=0\r\ndb1:keys=10,expires=0,avg_ttl=0
    for line in info.lines() {
        if line.starts_with("db") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let db_num_str = &parts[0][2..];
                if let Ok(db_idx) = db_num_str.parse::<usize>() {
                    if db_idx < 16 {
                        let metrics: Vec<&str> = parts[1].split(',').collect();
                        for metric in metrics {
                            if metric.trim().starts_with("keys=") {
                                if let Ok(count) = metric.trim()[5..].parse::<i64>() {
                                    sizes[db_idx] = count;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(sizes)
}

#[tauri::command]
async fn set_key_value(
    config: RedisConfig,
    key: String,
    value: String,
    db: i64,
    state: State<'_, ConnectionManager>,
) -> Result<(), String> {
    let mut con = state.get_connection(&config, db).await?;
    let _: () = con.set(key, value).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConnectionManager::default())
        .invoke_handler(tauri::generate_handler![
            connect_redis,
            list_keys,
            get_key_value,
            get_batch_key_values,
            get_db_sizes,
            set_key_value
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
