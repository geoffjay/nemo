//! Built-in data source implementations.

mod file;
mod http;
mod mqtt;
mod nats;
mod redis;
mod timer;
mod websocket;

pub use self::file::{FileFormat, FileSource, FileSourceConfig};
pub use self::http::{HttpSource, HttpSourceConfig};
pub use self::mqtt::{MqttSource, MqttSourceConfig};
pub use self::nats::{NatsSource, NatsSourceConfig};
pub use self::redis::{RedisSource, RedisSourceConfig};
pub use self::timer::{TimerSource, TimerSourceConfig};
pub use self::websocket::{WebSocketSource, WebSocketSourceConfig};

use crate::source::DataSource;
use nemo_config::Value;

/// Creates a DataSource from a type name and HCL configuration.
///
/// Returns `None` for unknown source types or missing required fields.
pub fn create_source(
    name: &str,
    source_type: &str,
    config: &Value,
) -> Option<Box<dyn DataSource>> {
    match source_type {
        "timer" => {
            let interval_secs = config
                .get("interval")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(1);
            let immediate = config
                .get("immediate")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let cfg = TimerSourceConfig {
                id: name.to_string(),
                interval: std::time::Duration::from_secs(interval_secs as u64),
                immediate,
                payload: config.get("payload").cloned(),
            };
            Some(Box::new(TimerSource::new(cfg)))
        }
        "http" => {
            let url = config.get("url").and_then(|v| v.as_str())?.to_string();
            let interval = config
                .get("interval")
                .and_then(|v| v.as_i64())
                .map(|secs| std::time::Duration::from_secs(secs as u64));

            let cfg = HttpSourceConfig {
                id: name.to_string(),
                url,
                interval,
                ..Default::default()
            };
            Some(Box::new(HttpSource::new(cfg)))
        }
        "websocket" => {
            let url = config.get("url").and_then(|v| v.as_str())?.to_string();
            let cfg = WebSocketSourceConfig {
                id: name.to_string(),
                url,
                ..Default::default()
            };
            Some(Box::new(WebSocketSource::new(cfg)))
        }
        "mqtt" => {
            let host = config
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("localhost")
                .to_string();
            let port = config.get("port").and_then(|v| v.as_i64()).unwrap_or(1883) as u16;
            let topics: Vec<String> = config
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let qos = config.get("qos").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
            let client_id = config
                .get("client_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let cfg = MqttSourceConfig {
                id: name.to_string(),
                host,
                port,
                topics,
                qos,
                client_id,
            };
            Some(Box::new(MqttSource::new(cfg)))
        }
        "redis" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("redis://127.0.0.1:6379")
                .to_string();
            let channels: Vec<String> = config
                .get("channels")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let cfg = RedisSourceConfig {
                id: name.to_string(),
                url,
                channels,
            };
            Some(Box::new(RedisSource::new(cfg)))
        }
        "nats" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("nats://127.0.0.1:4222")
                .to_string();
            let subjects: Vec<String> = config
                .get("subjects")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let cfg = NatsSourceConfig {
                id: name.to_string(),
                url,
                subjects,
            };
            Some(Box::new(NatsSource::new(cfg)))
        }
        "file" => {
            let path = config.get("path").and_then(|v| v.as_str())?.to_string();
            let watch = config
                .get("watch")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let format = match config
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("raw")
            {
                "json" => FileFormat::Json,
                "lines" => FileFormat::Lines,
                _ => FileFormat::Raw,
            };

            let cfg = FileSourceConfig {
                id: name.to_string(),
                path: std::path::PathBuf::from(path),
                format,
                watch,
                ..Default::default()
            };
            Some(Box::new(FileSource::new(cfg)))
        }
        _ => None,
    }
}
