//! MQTT client integration.

use crate::error::IntegrationError;
pub use rumqttc::QoS;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

/// MQTT client for pub/sub messaging.
pub struct MqttClient {
    /// Client instance.
    client: Option<AsyncClient>,
    /// Event loop handle.
    event_loop: Option<EventLoop>,
    /// Connection options.
    options: MqttOptions,
    /// Message broadcast sender.
    message_tx: broadcast::Sender<MqttMessage>,
    /// Connection state.
    connected: Arc<RwLock<bool>>,
}

/// An MQTT message.
#[derive(Debug, Clone)]
pub struct MqttMessage {
    /// Topic.
    pub topic: String,
    /// Payload.
    pub payload: Vec<u8>,
    /// QoS level.
    pub qos: u8,
    /// Retain flag.
    pub retain: bool,
}

impl MqttMessage {
    /// Returns the payload as a string.
    pub fn payload_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.payload)
    }

    /// Parses the payload as JSON.
    pub fn payload_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, IntegrationError> {
        serde_json::from_slice(&self.payload).map_err(|e| e.into())
    }
}

impl MqttClient {
    /// Creates a new MQTT client.
    pub fn new(
        id: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        let mut options = MqttOptions::new(id, host, port);
        options.set_keep_alive(Duration::from_secs(30));

        let (message_tx, _) = broadcast::channel(100);

        Self {
            client: None,
            event_loop: None,
            options,
            message_tx,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Sets credentials.
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.options.set_credentials(username, password);
        self
    }

    /// Sets keep-alive interval.
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        self.options.set_keep_alive(duration);
        self
    }

    /// Connects to the MQTT broker.
    pub async fn connect(&mut self) -> Result<(), IntegrationError> {
        let (client, event_loop) = AsyncClient::new(self.options.clone(), 10);

        self.client = Some(client);
        self.event_loop = Some(event_loop);
        *self.connected.write().await = true;

        Ok(())
    }

    /// Starts the event loop in a background task.
    pub fn start_event_loop(&mut self) -> Result<(), IntegrationError> {
        let mut event_loop = self
            .event_loop
            .take()
            .ok_or(IntegrationError::NotConnected {
                endpoint: "MQTT".to_string(),
            })?;

        let message_tx = self.message_tx.clone();
        let connected = self.connected.clone();

        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let msg = MqttMessage {
                            topic: publish.topic.clone(),
                            payload: publish.payload.to_vec(),
                            qos: publish.qos as u8,
                            retain: publish.retain,
                        };
                        let _ = message_tx.send(msg);
                    }
                    Ok(_) => {}
                    Err(_) => {
                        *connected.write().await = false;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribes to a topic.
    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<(), IntegrationError> {
        let client = self.client.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: "MQTT".to_string(),
        })?;

        client
            .subscribe(topic, qos)
            .await
            .map_err(|e| IntegrationError::Mqtt(e.to_string()))
    }

    /// Publishes a message.
    pub async fn publish(
        &self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        qos: QoS,
        retain: bool,
    ) -> Result<(), IntegrationError> {
        let client = self.client.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: "MQTT".to_string(),
        })?;

        client
            .publish(topic, qos, retain, payload)
            .await
            .map_err(|e| IntegrationError::Mqtt(e.to_string()))
    }

    /// Publishes a JSON message.
    pub async fn publish_json<T: serde::Serialize>(
        &self,
        topic: &str,
        value: &T,
        qos: QoS,
        retain: bool,
    ) -> Result<(), IntegrationError> {
        let payload = serde_json::to_vec(value)?;
        self.publish(topic, payload, qos, retain).await
    }

    /// Gets a receiver for incoming messages.
    pub fn messages(&self) -> broadcast::Receiver<MqttMessage> {
        self.message_tx.subscribe()
    }

    /// Checks if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Disconnects from the broker.
    pub async fn disconnect(&mut self) -> Result<(), IntegrationError> {
        if let Some(client) = &self.client {
            client
                .disconnect()
                .await
                .map_err(|e| IntegrationError::Mqtt(e.to_string()))?;
        }
        *self.connected.write().await = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MqttClient::new("test-client", "localhost", 1883);
        assert!(client.client.is_none());
    }

    #[test]
    fn test_message_payload() {
        let msg = MqttMessage {
            topic: "test".to_string(),
            payload: b"hello".to_vec(),
            qos: 0,
            retain: false,
        };
        assert_eq!(msg.payload_str().unwrap(), "hello");
    }
}
