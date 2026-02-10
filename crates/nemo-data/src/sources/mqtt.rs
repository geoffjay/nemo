//! MQTT streaming data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Configuration for an MQTT data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// MQTT broker host.
    pub host: String,
    /// MQTT broker port.
    pub port: u16,
    /// Topics to subscribe to.
    pub topics: Vec<String>,
    /// Quality of service level (0, 1, or 2).
    #[serde(default)]
    pub qos: u8,
    /// Client ID for the MQTT connection.
    pub client_id: Option<String>,
}

impl Default for MqttSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            host: "localhost".to_string(),
            port: 1883,
            topics: Vec::new(),
            qos: 0,
            client_id: None,
        }
    }
}

/// MQTT streaming data source.
pub struct MqttSource {
    config: MqttSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
}

impl MqttSource {
    /// Creates a new MQTT source.
    pub fn new(config: MqttSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("MQTT stream from {}:{}", config.host, config.port),
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
impl DataSource for MqttSource {
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

        let client_id = config
            .client_id
            .clone()
            .unwrap_or_else(|| format!("nemo-{}", config.id));

        let mut mqttoptions = rumqttc::MqttOptions::new(&client_id, &config.host, config.port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 100);

        // Subscribe to all configured topics
        let qos = match config.qos {
            0 => rumqttc::QoS::AtMostOnce,
            1 => rumqttc::QoS::AtLeastOnce,
            _ => rumqttc::QoS::ExactlyOnce,
        };

        for topic in &config.topics {
            if let Err(e) = client.subscribe(topic, qos).await {
                *status.write().await = SourceStatus::Error(e.to_string());
                return Err(DataSourceError::Connection(e.to_string()));
            }
        }

        let source_id = config.id.clone();
        let task = tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(rumqttc::Event::Incoming(rumqttc::Incoming::Publish(publish))) => {
                        *status.write().await = SourceStatus::Connected;

                        let topic = publish.topic.clone();
                        let payload_str = String::from_utf8_lossy(&publish.payload).to_string();

                        // Try to parse as JSON, fall back to string
                        let payload_value = if let Ok(json) =
                            serde_json::from_str::<serde_json::Value>(&payload_str)
                        {
                            Value::from(json)
                        } else {
                            Value::String(payload_str)
                        };

                        let mut data = indexmap::IndexMap::new();
                        data.insert("topic".to_string(), Value::String(topic));
                        data.insert("payload".to_string(), payload_value);

                        let update = DataUpdate::full(&source_id, Value::Object(data));
                        let _ = sender.send(update);
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Incoming::ConnAck(_))) => {
                        *status.write().await = SourceStatus::Connected;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        *status.write().await = SourceStatus::Error(e.to_string());
                        // Brief delay before retrying
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
        // MQTT is streaming, refresh doesn't apply
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_config_default() {
        let config = MqttSourceConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.qos, 0);
    }

    #[test]
    fn test_mqtt_source_creation() {
        let config = MqttSourceConfig {
            id: "test".into(),
            host: "broker.example.com".into(),
            port: 1883,
            topics: vec!["sensors/#".into()],
            ..Default::default()
        };
        let source = MqttSource::new(config);
        assert_eq!(source.id(), "test");
        assert_eq!(source.status(), SourceStatus::Disconnected);
    }
}
