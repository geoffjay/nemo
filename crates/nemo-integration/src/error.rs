//! Integration gateway error types.
//!
//! # Recovery Classification
//!
//! Integration errors are a mix of transient network failures (retryable)
//! and configuration problems (fatal). See the per-variant table below.

use thiserror::Error;

/// Errors that can occur in the integration gateway.
///
/// # Recovery
///
/// | Variant            | Recoverable? | Notes                                   |
/// |-------------------|-------------|------------------------------------------|
/// | `ConnectionFailed`| Transient   | Remote endpoint down; retry with backoff |
/// | `RequestFailed`   | Transient   | HTTP/protocol error; retry               |
/// | `Timeout`         | Transient   | Increase timeout or retry                |
/// | `InvalidConfig`   | Fatal       | Bad URL, missing fields; fix config      |
/// | `Protocol`        | Fatal       | Unexpected protocol message              |
/// | `Serialization`   | Fatal       | Data cannot be serialized/deserialized   |
/// | `ChannelClosed`   | Fatal       | Internal channel dropped; logic error    |
/// | `NotConnected`    | Transient   | Reconnect first, then retry              |
/// | `Http`            | Transient   | Underlying reqwest error                 |
/// | `WebSocket`       | Transient   | Connection/frame error; reconnect        |
/// | `Mqtt`            | Transient   | Broker communication error; reconnect    |
/// | `Redis`           | Transient   | Server communication error; reconnect    |
/// | `Io`              | Transient   | File/network I/O error                   |
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
