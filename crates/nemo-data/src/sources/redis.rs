//! Redis pub/sub data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Configuration for a Redis pub/sub data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// Redis connection URL.
    pub url: String,
    /// Channels to subscribe to.
    pub channels: Vec<String>,
}

impl Default for RedisSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: "redis://127.0.0.1:6379".to_string(),
            channels: Vec::new(),
        }
    }
}

/// Redis pub/sub data source.
pub struct RedisSource {
    config: RedisSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
}

impl RedisSource {
    /// Creates a new Redis source.
    pub fn new(config: RedisSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("Redis pub/sub from {}", config.url),
            value_type: SchemaType::Any,
        };

        Self {
            config,
            status: Arc::new(RwLock::new(SourceStatus::Disconnected)),
            sender,
            schema,
            task: None,
        }
    }
}

#[async_trait]
impl DataSource for RedisSource {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn schema(&self) -> &DataSchema {
        &self.schema
    }

    async fn start(&mut self) -> Result<(), DataSourceError> {
        if self.task.is_some() {
            return Err(DataSourceError::AlreadyRunning);
        }

        *self.status.write().await = SourceStatus::Connecting;

        let config = self.config.clone();
        let status = self.status.clone();
        let sender = self.sender.clone();

        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| DataSourceError::Connection(e.to_string()))?;

        let source_id = config.id.clone();
        let task = tokio::spawn(async move {
            let conn = match client.get_async_pubsub().await {
                Ok(conn) => conn,
                Err(e) => {
                    *status.write().await = SourceStatus::Error(e.to_string());
                    return;
                }
            };

            let mut pubsub = conn;

            for channel in &config.channels {
                if let Err(e) = pubsub.subscribe(channel).await {
                    *status.write().await = SourceStatus::Error(e.to_string());
                    return;
                }
            }

            *status.write().await = SourceStatus::Connected;

            let mut msg_stream = pubsub.on_message();

            loop {
                use futures::StreamExt;
                match msg_stream.next().await {
                    Some(msg) => {
                        let channel: String = msg.get_channel_name().to_string();
                        let payload: String = match msg.get_payload() {
                            Ok(p) => p,
                            Err(_) => continue,
                        };

                        // Try to parse as JSON, fall back to string
                        let payload_value =
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                                Value::from(json)
                            } else {
                                Value::String(payload)
                            };

                        let mut data = indexmap::IndexMap::new();
                        data.insert("channel".to_string(), Value::String(channel));
                        data.insert("payload".to_string(), payload_value);

                        let update = DataUpdate::full(&source_id, Value::Object(data));
                        let _ = sender.send(update);
                    }
                    None => {
                        *status.write().await = SourceStatus::Disconnected;
                        break;
                    }
                }
            }
        });

        self.task = Some(task);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DataSourceError> {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        *self.status.write().await = SourceStatus::Disconnected;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<DataUpdate> {
        self.sender.subscribe()
    }

    fn status(&self) -> SourceStatus {
        self.status
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(SourceStatus::Disconnected)
    }

    async fn refresh(&mut self) -> Result<(), DataSourceError> {
        // Redis pub/sub is streaming, refresh doesn't apply
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_default() {
        let config = RedisSourceConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:6379");
    }

    #[test]
    fn test_redis_source_creation() {
        let config = RedisSourceConfig {
            id: "test".into(),
            url: "redis://localhost:6379".into(),
            channels: vec!["events".into()],
        };
        let source = RedisSource::new(config);
        assert_eq!(source.id(), "test");
        assert_eq!(source.status(), SourceStatus::Disconnected);
    }
}
