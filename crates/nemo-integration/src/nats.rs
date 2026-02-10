//! NATS client integration.

use crate::error::IntegrationError;
use async_nats::Client;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// NATS client for pub/sub messaging.
pub struct NatsClient {
    /// Server URLs.
    urls: Vec<String>,
    /// Client instance.
    client: Option<Client>,
    /// Message broadcast sender.
    message_tx: broadcast::Sender<NatsMessage>,
    /// Connection state.
    connected: Arc<RwLock<bool>>,
}

/// A NATS message.
#[derive(Debug, Clone)]
pub struct NatsMessage {
    /// Subject.
    pub subject: String,
    /// Payload.
    pub payload: Vec<u8>,
    /// Reply subject (for request/reply pattern).
    pub reply: Option<String>,
}

impl NatsMessage {
    /// Returns the payload as a string.
    pub fn payload_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.payload)
    }

    /// Parses the payload as JSON.
    pub fn payload_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, IntegrationError> {
        serde_json::from_slice(&self.payload).map_err(|e| e.into())
    }
}

impl NatsClient {
    /// Creates a new NATS client.
    pub fn new(url: impl Into<String>) -> Self {
        let (message_tx, _) = broadcast::channel(100);
        Self {
            urls: vec![url.into()],
            client: None,
            message_tx,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Creates a new NATS client with multiple servers.
    pub fn with_servers(urls: Vec<String>) -> Self {
        let (message_tx, _) = broadcast::channel(100);
        Self {
            urls,
            client: None,
            message_tx,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Connects to the NATS server.
    pub async fn connect(&mut self) -> Result<(), IntegrationError> {
        let url = self.urls.join(",");
        let client =
            async_nats::connect(&url)
                .await
                .map_err(|e| IntegrationError::ConnectionFailed {
                    endpoint: url,
                    reason: e.to_string(),
                })?;

        self.client = Some(client);
        *self.connected.write().await = true;
        Ok(())
    }

    /// Publishes a message.
    pub async fn publish(
        &self,
        subject: &str,
        payload: impl AsRef<[u8]>,
    ) -> Result<(), IntegrationError> {
        let client = self.client.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: "NATS".to_string(),
        })?;

        client
            .publish(subject.to_string(), payload.as_ref().to_vec().into())
            .await
            .map_err(|e| IntegrationError::Protocol(e.to_string()))
    }

    /// Publishes a JSON message.
    pub async fn publish_json<T: serde::Serialize>(
        &self,
        subject: &str,
        value: &T,
    ) -> Result<(), IntegrationError> {
        let payload = serde_json::to_vec(value)?;
        self.publish(subject, payload).await
    }

    /// Subscribes to a subject.
    pub async fn subscribe(
        &self,
        subject: &str,
    ) -> Result<broadcast::Receiver<NatsMessage>, IntegrationError> {
        let client = self.client.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: "NATS".to_string(),
        })?;

        let mut subscription = client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| IntegrationError::Protocol(e.to_string()))?;

        let message_tx = self.message_tx.clone();
        let receiver = self.message_tx.subscribe();

        tokio::spawn(async move {
            while let Some(msg) = subscription.next().await {
                let nats_msg = NatsMessage {
                    subject: msg.subject.as_str().to_string(),
                    payload: msg.payload.to_vec(),
                    reply: msg.reply.as_ref().map(|s| s.as_str().to_string()),
                };
                let _ = message_tx.send(nats_msg);
            }
        });

        Ok(receiver)
    }

    /// Makes a request and waits for a reply.
    pub async fn request(
        &self,
        subject: &str,
        payload: impl AsRef<[u8]>,
    ) -> Result<NatsMessage, IntegrationError> {
        let client = self.client.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: "NATS".to_string(),
        })?;

        let response = client
            .request(subject.to_string(), payload.as_ref().to_vec().into())
            .await
            .map_err(|e| IntegrationError::RequestFailed(e.to_string()))?;

        Ok(NatsMessage {
            subject: response.subject.to_string(),
            payload: response.payload.to_vec(),
            reply: response.reply.map(|s| s.to_string()),
        })
    }

    /// Makes a JSON request and parses the response.
    pub async fn request_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        subject: &str,
        value: &T,
    ) -> Result<R, IntegrationError> {
        let payload = serde_json::to_vec(value)?;
        let response = self.request(subject, payload).await?;
        response.payload_json()
    }

    /// Checks if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Returns the server URLs.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NatsClient::new("nats://localhost:4222");
        assert_eq!(client.urls(), &["nats://localhost:4222"]);
    }

    #[test]
    fn test_message_payload() {
        let msg = NatsMessage {
            subject: "test".to_string(),
            payload: b"hello".to_vec(),
            reply: None,
        };
        assert_eq!(msg.payload_str().unwrap(), "hello");
    }
}
