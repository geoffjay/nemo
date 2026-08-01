//! Built-in data source implementations.

mod file;
mod http;
mod mqtt;
mod nats;
mod redis;
mod timer;
mod websocket;

pub use self::file::{FileFormat, FileSource, FileSourceConfig};
pub use self::http::{HttpMethod, HttpSource, HttpSourceConfig};
pub use self::mqtt::{MqttSource, MqttSourceConfig};
pub use self::nats::{NatsSource, NatsSourceConfig};
pub use self::redis::{RedisSource, RedisSourceConfig};
pub use self::timer::{TimerSource, TimerSourceConfig};
pub use self::websocket::{WebSocketSource, WebSocketSourceConfig};

use crate::source::DataSource;
use nemo_config::Value;
use std::collections::HashMap;

/// Parses the `headers` property of an HTTP source into a string map.
///
/// Accepts either a config object (`<headers Authorization="Bearer …" />`-style
/// nested attributes) or a JSON-string attribute
/// (`headers='{"Authorization":"Bearer …"}'`). Non-string values are stringified
/// so numeric header values are tolerated. Header values authored as
/// `${env.TOKEN}` / `${var.x}` are already resolved by the config resolver before
/// reaching here.
fn parse_http_headers(value: Option<&Value>) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    let obj = match value {
        Some(v) => {
            if let Some(obj) = v.as_object() {
                obj.clone()
            } else if let Some(s) = v.as_str() {
                // JSON-string form: parse and recurse on the parsed object.
                match serde_json::from_str::<serde_json::Value>(s) {
                    Ok(json) if json.is_object() => {
                        return parse_http_headers(Some(&Value::from(json)));
                    }
                    _ => return headers,
                }
            } else {
                return headers;
            }
        }
        None => return headers,
    };

    for (key, val) in obj {
        // Header values are strings; tolerate scalar JSON values by stringifying.
        let s = if let Some(s) = val.as_str() {
            s.to_string()
        } else if let Some(i) = val.as_i64() {
            i.to_string()
        } else if let Some(f) = val.as_f64() {
            f.to_string()
        } else if let Some(b) = val.as_bool() {
            b.to_string()
        } else {
            continue;
        };
        headers.insert(key, s);
    }
    headers
}

/// Creates a DataSource from a type name and XML configuration.
///
/// Returns `None` for unknown source types or missing required fields.
pub fn create_source(name: &str, source_type: &str, config: &Value) -> Option<Box<dyn DataSource>> {
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

            let method = match config
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET")
                .to_ascii_uppercase()
                .as_str()
            {
                "POST" => HttpMethod::Post,
                "PUT" => HttpMethod::Put,
                "PATCH" => HttpMethod::Patch,
                "DELETE" => HttpMethod::Delete,
                _ => HttpMethod::Get,
            };

            let headers = parse_http_headers(config.get("headers"));
            let body = config.get("body").cloned();

            let cfg = HttpSourceConfig {
                id: name.to_string(),
                url,
                method,
                headers,
                body,
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
