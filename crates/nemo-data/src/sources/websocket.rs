//! WebSocket streaming data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Reconnect configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// Whether reconnection is enabled.
    pub enabled: bool,
    /// Maximum reconnection attempts.
    pub max_attempts: u32,
    /// Initial delay between attempts.
    pub delay: Duration,
    /// Maximum delay between attempts.
    pub max_delay: Duration,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 10,
            delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Heartbeat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Interval between heartbeats.
    pub interval: Duration,
    /// Message to send.
    pub message: String,
}

/// Configuration for a WebSocket data source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSocketSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// WebSocket URL.
    pub url: String,
    /// Subprotocols.
    #[serde(default)]
    pub protocols: Vec<String>,
    /// Headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Reconnect configuration.
    #[serde(default)]
    pub reconnect: ReconnectConfig,
    /// Heartbeat configuration.
    pub heartbeat: Option<HeartbeatConfig>,
}

/// WebSocket streaming data source.
pub struct WebSocketSource {
    config: WebSocketSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
}

impl WebSocketSource {
    /// Creates a new WebSocket source.
    pub fn new(config: WebSocketSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("WebSocket stream from {}", config.url),
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
impl DataSource for WebSocketSource {
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

        let task = tokio::spawn(async move {
            let mut attempt = 0;
            let mut delay = config.reconnect.delay;

            loop {
                match connect_async(&config.url).await {
                    Ok((ws_stream, _)) => {
                        *status.write().await = SourceStatus::Connected;
                        attempt = 0;
                        delay = config.reconnect.delay;

                        let (mut write, mut read) = ws_stream.split();

                        // Channel for sending messages (heartbeats and pong responses)
                        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Message>(10);

                        // Spawn write task to handle all outgoing messages
                        let write_task = tokio::spawn(async move {
                            while let Some(msg) = write_rx.recv().await {
                                if write.send(msg).await.is_err() {
                                    break;
                                }
                            }
                        });

                        // Optional heartbeat task
                        let heartbeat_task = if let Some(hb) = &config.heartbeat {
                            let msg = hb.message.clone();
                            let interval = hb.interval;
                            let hb_tx = write_tx.clone();
                            
                            Some(tokio::spawn(async move {
                                let mut timer = tokio::time::interval(interval);
                                loop {
                                    timer.tick().await;
                                    let message = Message::Text(msg.clone());
                                    if hb_tx.send(message).await.is_err() {
                                        break;
                                    }
                                }
                            }))
                        } else {
                            None
                        };

                        // Read messages
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(&text)
                                    {
                                        let update =
                                            DataUpdate::full(&config.id, Value::from(json));
                                        let _ = sender.send(update);
                                    } else {
                                        let update =
                                            DataUpdate::full(&config.id, Value::String(text));
                                        let _ = sender.send(update);
                                    }
                                }
                                Ok(Message::Binary(data)) => {
                                    if let Ok(json) =
                                        serde_json::from_slice::<serde_json::Value>(&data)
                                    {
                                        let update =
                                            DataUpdate::full(&config.id, Value::from(json));
                                        let _ = sender.send(update);
                                    }
                                }
                                Ok(Message::Ping(data)) => {
                                    let _ = write_tx.send(Message::Pong(data)).await;
                                }
                                Ok(Message::Close(_)) => {
                                    break;
                                }
                                Err(_) => {
                                    break;
                                }
                                _ => {}
                            }
                        }

                        // Cleanup tasks
                        drop(write_tx);
                        write_task.abort();
                        if let Some(task) = heartbeat_task {
                            task.abort();
                        }

                        *status.write().await = SourceStatus::Disconnected;
                    }
                    Err(e) => {
                        *status.write().await = SourceStatus::Error(e.to_string());
                    }
                }

                // Reconnection logic
                if !config.reconnect.enabled {
                    break;
                }

                attempt += 1;
                if attempt > config.reconnect.max_attempts {
                    *status.write().await =
                        SourceStatus::Error("Max reconnection attempts reached".into());
                    break;
                }

                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, config.reconnect.max_delay);
                *status.write().await = SourceStatus::Connecting;
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
        // WebSocket is streaming, refresh doesn't apply
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketSourceConfig::default();
        assert!(config.reconnect.enabled);
    }

    #[test]
    fn test_websocket_source_creation() {
        let config = WebSocketSourceConfig {
            id: "test".into(),
            url: "wss://example.com/stream".into(),
            ..Default::default()
        };
        let source = WebSocketSource::new(config);
        assert_eq!(source.id(), "test");
    }
}
