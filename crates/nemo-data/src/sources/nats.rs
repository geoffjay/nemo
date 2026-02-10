//! NATS streaming data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Configuration for a NATS data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// NATS server URL(s).
    pub url: String,
    /// Subjects to subscribe to.
    pub subjects: Vec<String>,
}

impl Default for NatsSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: "nats://127.0.0.1:4222".to_string(),
            subjects: Vec::new(),
        }
    }
}

/// NATS streaming data source.
pub struct NatsSource {
    config: NatsSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
}

impl NatsSource {
    /// Creates a new NATS source.
    pub fn new(config: NatsSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("NATS stream from {}", config.url),
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
impl DataSource for NatsSource {
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

        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| DataSourceError::Connection(e.to_string()))?;

        *self.status.write().await = SourceStatus::Connected;

        let source_id = config.id.clone();
        let task = tokio::spawn(async move {
            use futures::StreamExt;

            // Subscribe to all configured subjects and merge into a single stream
            let mut subscriptions = Vec::new();
            for subject in &config.subjects {
                match client.subscribe(subject.clone()).await {
                    Ok(sub) => subscriptions.push((subject.clone(), sub)),
                    Err(e) => {
                        *status.write().await = SourceStatus::Error(e.to_string());
                        return;
                    }
                }
            }

            // Process messages from all subscriptions concurrently
            let mut handles = Vec::new();
            for (subject, mut sub) in subscriptions {
                let sender = sender.clone();
                let source_id = source_id.clone();
                let status = status.clone();

                handles.push(tokio::spawn(async move {
                    while let Some(msg) = sub.next().await {
                        let payload_str = String::from_utf8_lossy(&msg.payload).to_string();

                        // Try to parse as JSON, fall back to string
                        let payload_value = if let Ok(json) =
                            serde_json::from_str::<serde_json::Value>(&payload_str)
                        {
                            Value::from(json)
                        } else {
                            Value::String(payload_str)
                        };

                        let mut data = indexmap::IndexMap::new();
                        data.insert(
                            "subject".to_string(),
                            Value::String(msg.subject.to_string()),
                        );
                        data.insert("payload".to_string(), payload_value);

                        let update = DataUpdate::full(&source_id, Value::Object(data));
                        let _ = sender.send(update);
                    }

                    *status.write().await = SourceStatus::Disconnected;
                }));
            }

            // Wait for all subscription handlers
            for handle in handles {
                let _ = handle.await;
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
        // NATS is streaming, refresh doesn't apply
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nats_config_default() {
        let config = NatsSourceConfig::default();
        assert_eq!(config.url, "nats://127.0.0.1:4222");
    }

    #[test]
    fn test_nats_source_creation() {
        let config = NatsSourceConfig {
            id: "test".into(),
            url: "nats://localhost:4222".into(),
            subjects: vec!["events.>".into()],
        };
        let source = NatsSource::new(config);
        assert_eq!(source.id(), "test");
        assert_eq!(source.status(), SourceStatus::Disconnected);
    }
}
