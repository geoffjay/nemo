//! Timer data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use chrono::Utc;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Configuration for a timer data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// Interval between ticks.
    pub interval: Duration,
    /// Whether to emit immediately on start.
    #[serde(default = "default_immediate")]
    pub immediate: bool,
    /// Static payload to emit with each tick.
    pub payload: Option<Value>,
}

fn default_immediate() -> bool {
    true
}

impl Default for TimerSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            interval: Duration::from_secs(1),
            immediate: true,
            payload: None,
        }
    }
}

/// Timer data source.
pub struct TimerSource {
    config: TimerSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
    tick_count: Arc<RwLock<u64>>,
}

impl TimerSource {
    /// Creates a new timer source.
    pub fn new(config: TimerSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("Timer emitting every {:?}", config.interval),
            value_type: SchemaType::Object,
        };

        Self {
            config,
            status: Arc::new(RwLock::new(SourceStatus::Disconnected)),
            sender,
            schema,
            task: None,
            tick_count: Arc::new(RwLock::new(0)),
        }
    }

    fn create_tick_payload(&self, tick: u64) -> Value {
        let mut obj = indexmap::IndexMap::new();
        obj.insert("tick".to_string(), Value::Integer(tick as i64));
        obj.insert(
            "timestamp".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );

        if let Some(payload) = &self.config.payload {
            obj.insert("payload".to_string(), payload.clone());
        }

        Value::Object(obj)
    }
}

#[async_trait]
impl DataSource for TimerSource {
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

        *self.status.write().await = SourceStatus::Connected;
        *self.tick_count.write().await = 0;

        // Emit immediately if configured
        if self.config.immediate {
            let data = self.create_tick_payload(0);
            let update = DataUpdate::full(&self.config.id, data);
            let _ = self.sender.send(update);
            *self.tick_count.write().await = 1;
        }

        let config = self.config.clone();
        let status = self.status.clone();
        let sender = self.sender.clone();
        let tick_count = self.tick_count.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.interval);

            // Skip first tick if we already emitted immediately
            if config.immediate {
                interval.tick().await;
            }

            loop {
                interval.tick().await;

                let tick = {
                    let mut count = tick_count.write().await;
                    let current = *count;
                    *count += 1;
                    current
                };

                let mut obj = indexmap::IndexMap::new();
                obj.insert("tick".to_string(), Value::Integer(tick as i64));
                obj.insert(
                    "timestamp".to_string(),
                    Value::String(Utc::now().to_rfc3339()),
                );

                if let Some(payload) = &config.payload {
                    obj.insert("payload".to_string(), payload.clone());
                }

                let data = Value::Object(obj);
                let update = DataUpdate::full(&config.id, data);

                if sender.send(update).is_err() {
                    // No receivers, but keep running
                }

                // Keep status as connected
                *status.write().await = SourceStatus::Connected;
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
        let tick = *self.tick_count.read().await;
        let data = self.create_tick_payload(tick);
        let update = DataUpdate::full(&self.config.id, data);
        let _ = self.sender.send(update);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_config_default() {
        let config = TimerSourceConfig::default();
        assert!(config.immediate);
        assert_eq!(config.interval, Duration::from_secs(1));
    }

    #[test]
    fn test_timer_source_creation() {
        let config = TimerSourceConfig {
            id: "test".into(),
            interval: Duration::from_millis(100),
            ..Default::default()
        };
        let source = TimerSource::new(config);
        assert_eq!(source.id(), "test");
    }

    #[test]
    fn test_tick_payload() {
        let config = TimerSourceConfig {
            id: "test".into(),
            payload: Some(Value::String("custom".into())),
            ..Default::default()
        };
        let source = TimerSource::new(config);
        let payload = source.create_tick_payload(5);

        if let Value::Object(obj) = payload {
            assert_eq!(obj.get("tick"), Some(&Value::Integer(5)));
            assert!(obj.contains_key("timestamp"));
            assert!(obj.contains_key("payload"));
        } else {
            panic!("Expected object");
        }
    }
}
