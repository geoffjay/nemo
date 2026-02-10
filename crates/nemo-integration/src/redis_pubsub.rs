//! Redis pub/sub integration.

use crate::error::IntegrationError;
use futures::StreamExt;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Redis client for pub/sub and key-value operations.
pub struct RedisClient {
    /// Connection URL.
    url: String,
    /// Connection instance.
    connection: Option<MultiplexedConnection>,
    /// Message broadcast sender.
    message_tx: broadcast::Sender<RedisMessage>,
    /// Connection state.
    connected: Arc<RwLock<bool>>,
}

/// A Redis pub/sub message.
#[derive(Debug, Clone)]
pub struct RedisMessage {
    /// Channel name.
    pub channel: String,
    /// Message payload.
    pub payload: String,
}

impl RedisClient {
    /// Creates a new Redis client.
    pub fn new(url: impl Into<String>) -> Self {
        let (message_tx, _) = broadcast::channel(100);
        Self {
            url: url.into(),
            connection: None,
            message_tx,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Connects to Redis.
    pub async fn connect(&mut self) -> Result<(), IntegrationError> {
        let client =
            Client::open(self.url.as_str()).map_err(|e| IntegrationError::ConnectionFailed {
                endpoint: self.url.clone(),
                reason: e.to_string(),
            })?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| IntegrationError::ConnectionFailed {
                endpoint: self.url.clone(),
                reason: e.to_string(),
            })?;

        self.connection = Some(connection);
        *self.connected.write().await = true;
        Ok(())
    }

    /// Gets a value by key.
    pub async fn get(&self, key: &str) -> Result<Option<String>, IntegrationError> {
        let mut conn = self
            .connection
            .clone()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        let value: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))?;
        Ok(value)
    }

    /// Sets a value.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), IntegrationError> {
        let mut conn = self
            .connection
            .clone()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        conn.set(key, value)
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))
    }

    /// Sets a value with expiration.
    pub async fn set_ex(
        &self,
        key: &str,
        value: &str,
        seconds: u64,
    ) -> Result<(), IntegrationError> {
        let mut conn = self
            .connection
            .clone()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        conn.set_ex(key, value, seconds)
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))
    }

    /// Deletes a key.
    pub async fn del(&self, key: &str) -> Result<bool, IntegrationError> {
        let mut conn = self
            .connection
            .clone()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        let deleted: i32 = conn
            .del(key)
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))?;
        Ok(deleted > 0)
    }

    /// Publishes a message to a channel.
    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), IntegrationError> {
        let mut conn = self
            .connection
            .clone()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        conn.publish(channel, message)
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))
    }

    /// Subscribes to channels and returns a receiver.
    pub async fn subscribe(
        &self,
        channels: &[&str],
    ) -> Result<broadcast::Receiver<RedisMessage>, IntegrationError> {
        let client =
            Client::open(self.url.as_str()).map_err(|e| IntegrationError::ConnectionFailed {
                endpoint: self.url.clone(),
                reason: e.to_string(),
            })?;

        let mut pubsub = client
            .get_async_pubsub()
            .await
            .map_err(|e| IntegrationError::Redis(e.to_string()))?;

        for channel in channels {
            pubsub
                .subscribe(*channel)
                .await
                .map_err(|e| IntegrationError::Redis(e.to_string()))?;
        }

        let message_tx = self.message_tx.clone();
        let receiver = self.message_tx.subscribe();

        // Spawn task to forward messages
        tokio::spawn(async move {
            let mut stream = pubsub.into_on_message();
            while let Some(msg) = stream.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    let redis_msg = RedisMessage {
                        channel: msg.get_channel_name().to_string(),
                        payload,
                    };
                    let _ = message_tx.send(redis_msg);
                }
            }
        });

        Ok(receiver)
    }

    /// Checks if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Returns the connection URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl RedisMessage {
    /// Parses the payload as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, IntegrationError> {
        serde_json::from_str(&self.payload).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = RedisClient::new("redis://localhost:6379");
        assert_eq!(client.url(), "redis://localhost:6379");
    }

    #[test]
    fn test_message_json() {
        let msg = RedisMessage {
            channel: "test".to_string(),
            payload: r#"{"key": "value"}"#.to_string(),
        };
        let parsed: serde_json::Value = msg.json().unwrap();
        assert_eq!(parsed["key"], "value");
    }
}
