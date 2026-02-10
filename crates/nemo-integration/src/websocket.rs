//! WebSocket client integration.

use crate::error::IntegrationError;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// WebSocket client for bidirectional communication.
pub struct WebSocketClient {
    /// The WebSocket URL.
    url: String,
    /// Sender for outgoing messages.
    sender: Option<mpsc::Sender<String>>,
    /// Receiver for incoming messages.
    receiver: Option<broadcast::Receiver<String>>,
    /// Connection state.
    connected: Arc<RwLock<bool>>,
    /// Broadcast sender for incoming messages (for cloning receivers).
    broadcast_tx: broadcast::Sender<String>,
}

impl WebSocketClient {
    /// Creates a new WebSocket client.
    pub fn new(url: impl Into<String>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        Self {
            url: url.into(),
            sender: None,
            receiver: None,
            connected: Arc::new(RwLock::new(false)),
            broadcast_tx,
        }
    }

    /// Connects to the WebSocket server.
    pub async fn connect(&mut self) -> Result<(), IntegrationError> {
        let (ws_stream, _) =
            connect_async(&self.url)
                .await
                .map_err(|e| IntegrationError::ConnectionFailed {
                    endpoint: self.url.clone(),
                    reason: e.to_string(),
                })?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<String>(100);
        let broadcast_tx = self.broadcast_tx.clone();
        let connected = self.connected.clone();

        // Mark as connected
        *connected.write().await = true;

        // Spawn task to handle outgoing messages
        let conn = connected.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if write.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            *conn.write().await = false;
        });

        // Spawn task to handle incoming messages
        let conn = connected.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Message::Text(text) = msg {
                    let _ = broadcast_tx.send(text);
                }
            }
            *conn.write().await = false;
        });

        self.sender = Some(tx);
        self.receiver = Some(self.broadcast_tx.subscribe());

        Ok(())
    }

    /// Sends a message.
    pub async fn send(&self, message: impl Into<String>) -> Result<(), IntegrationError> {
        let sender = self.sender.as_ref().ok_or(IntegrationError::NotConnected {
            endpoint: self.url.clone(),
        })?;

        sender
            .send(message.into())
            .await
            .map_err(|_| IntegrationError::ChannelClosed)
    }

    /// Sends a JSON message.
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), IntegrationError> {
        let json = serde_json::to_string(value)?;
        self.send(json).await
    }

    /// Receives a message.
    pub async fn recv(&mut self) -> Result<String, IntegrationError> {
        let receiver = self
            .receiver
            .as_mut()
            .ok_or(IntegrationError::NotConnected {
                endpoint: self.url.clone(),
            })?;

        receiver
            .recv()
            .await
            .map_err(|_| IntegrationError::ChannelClosed)
    }

    /// Gets a new subscription to incoming messages.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcast_tx.subscribe()
    }

    /// Checks if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Returns the URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// WebSocket message handler trait.
#[async_trait::async_trait]
pub trait WebSocketHandler: Send + Sync {
    /// Called when a message is received.
    async fn on_message(&self, message: String);

    /// Called when the connection is established.
    async fn on_connect(&self) {}

    /// Called when the connection is closed.
    async fn on_disconnect(&self) {}

    /// Called when an error occurs.
    async fn on_error(&self, _error: IntegrationError) {}
}

/// Managed WebSocket connection with automatic reconnection.
pub struct ManagedWebSocket {
    /// The client.
    client: WebSocketClient,
    /// Message handler.
    handler: Arc<dyn WebSocketHandler>,
    /// Whether to auto-reconnect.
    auto_reconnect: bool,
    /// Reconnection delay.
    reconnect_delay: std::time::Duration,
}

impl ManagedWebSocket {
    /// Creates a new managed WebSocket.
    pub fn new(url: impl Into<String>, handler: Arc<dyn WebSocketHandler>) -> Self {
        Self {
            client: WebSocketClient::new(url),
            handler,
            auto_reconnect: true,
            reconnect_delay: std::time::Duration::from_secs(5),
        }
    }

    /// Disables auto-reconnect.
    pub fn no_reconnect(mut self) -> Self {
        self.auto_reconnect = false;
        self
    }

    /// Sets the reconnect delay.
    pub fn reconnect_delay(mut self, delay: std::time::Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    /// Starts the managed connection.
    pub async fn start(&mut self) -> Result<(), IntegrationError> {
        loop {
            match self.client.connect().await {
                Ok(()) => {
                    self.handler.on_connect().await;
                    self.run_message_loop().await;
                    self.handler.on_disconnect().await;
                }
                Err(e) => {
                    self.handler.on_error(e).await;
                }
            }

            if !self.auto_reconnect {
                break;
            }

            tokio::time::sleep(self.reconnect_delay).await;
        }

        Ok(())
    }

    /// Runs the message handling loop.
    async fn run_message_loop(&mut self) {
        let mut receiver = self.client.subscribe();
        while let Ok(message) = receiver.recv().await {
            self.handler.on_message(message).await;
        }
    }

    /// Sends a message.
    pub async fn send(&self, message: impl Into<String>) -> Result<(), IntegrationError> {
        self.client.send(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WebSocketClient::new("ws://localhost:8080");
        assert_eq!(client.url(), "ws://localhost:8080");
    }
}
