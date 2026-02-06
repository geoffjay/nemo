//! Integration gateway error types.

use thiserror::Error;

/// Errors that can occur in the integration gateway.
#[derive(Debug, Error)]
pub enum IntegrationError {
    /// Connection failed.
    #[error("Connection failed to '{endpoint}': {reason}")]
    ConnectionFailed { endpoint: String, reason: String },

    /// Request failed.
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Timeout.
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Protocol error.
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Channel closed.
    #[error("Channel closed")]
    ChannelClosed,

    /// Not connected.
    #[error("Not connected to '{endpoint}'")]
    NotConnected { endpoint: String },

    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// MQTT error.
    #[error("MQTT error: {0}")]
    Mqtt(String),

    /// Redis error.
    #[error("Redis error: {0}")]
    Redis(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<serde_json::Error> for IntegrationError {
    fn from(e: serde_json::Error) -> Self {
        IntegrationError::Serialization(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for IntegrationError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        IntegrationError::WebSocket(e.to_string())
    }
}
