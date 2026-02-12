//! Nemo Integration Gateway - External system communication.
//!
//! This crate provides integration with external systems including:
//! - HTTP client for REST APIs
//! - WebSocket client for real-time communication
//! - MQTT client for IoT messaging
//! - Redis client for pub/sub and caching
//! - NATS client for distributed messaging

pub mod error;
pub mod http;
pub mod mqtt;
pub mod nats;
pub mod redis_pubsub;
pub mod websocket;

pub use error::IntegrationError;
pub use http::{HttpClient, HttpRequest, HttpResponse};
pub use mqtt::{MqttClient, MqttMessage, QoS};
pub use nats::{NatsClient, NatsMessage};
pub use redis_pubsub::{RedisClient, RedisMessage};
pub use websocket::{ManagedWebSocket, WebSocketClient, WebSocketHandler};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for an integration endpoint.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// Endpoint type.
    pub endpoint_type: EndpointType,
    /// Connection URL or address.
    pub url: String,
    /// Additional configuration options.
    pub options: HashMap<String, String>,
}

/// Type of integration endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    /// HTTP endpoint.
    Http,
    /// WebSocket endpoint.
    WebSocket,
    /// MQTT broker.
    Mqtt,
    /// Redis server.
    Redis,
    /// NATS server.
    Nats,
}

/// The integration gateway manages all external connections.
pub struct IntegrationGateway {
    /// HTTP clients by name.
    http_clients: RwLock<HashMap<String, Arc<HttpClient>>>,
    /// WebSocket clients by name.
    ws_clients: RwLock<HashMap<String, Arc<RwLock<WebSocketClient>>>>,
    /// MQTT clients by name.
    mqtt_clients: RwLock<HashMap<String, Arc<RwLock<MqttClient>>>>,
    /// Redis clients by name.
    redis_clients: RwLock<HashMap<String, Arc<RwLock<RedisClient>>>>,
    /// NATS clients by name.
    nats_clients: RwLock<HashMap<String, Arc<RwLock<NatsClient>>>>,
}

impl IntegrationGateway {
    /// Creates a new integration gateway.
    pub fn new() -> Self {
        Self {
            http_clients: RwLock::new(HashMap::new()),
            ws_clients: RwLock::new(HashMap::new()),
            mqtt_clients: RwLock::new(HashMap::new()),
            redis_clients: RwLock::new(HashMap::new()),
            nats_clients: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an HTTP client.
    pub async fn register_http(&self, name: impl Into<String>, client: HttpClient) {
        let mut clients = self.http_clients.write().await;
        clients.insert(name.into(), Arc::new(client));
    }

    /// Gets an HTTP client by name.
    pub async fn http(&self, name: &str) -> Option<Arc<HttpClient>> {
        let clients = self.http_clients.read().await;
        clients.get(name).cloned()
    }

    /// Registers a WebSocket client.
    pub async fn register_websocket(&self, name: impl Into<String>, client: WebSocketClient) {
        let mut clients = self.ws_clients.write().await;
        clients.insert(name.into(), Arc::new(RwLock::new(client)));
    }

    /// Gets a WebSocket client by name.
    pub async fn websocket(&self, name: &str) -> Option<Arc<RwLock<WebSocketClient>>> {
        let clients = self.ws_clients.read().await;
        clients.get(name).cloned()
    }

    /// Registers an MQTT client.
    #[allow(clippy::arc_with_non_send_sync)]
    pub async fn register_mqtt(&self, name: impl Into<String>, client: MqttClient) {
        let mut clients = self.mqtt_clients.write().await;
        clients.insert(name.into(), Arc::new(RwLock::new(client)));
    }

    /// Gets an MQTT client by name.
    pub async fn mqtt(&self, name: &str) -> Option<Arc<RwLock<MqttClient>>> {
        let clients = self.mqtt_clients.read().await;
        clients.get(name).cloned()
    }

    /// Registers a Redis client.
    pub async fn register_redis(&self, name: impl Into<String>, client: RedisClient) {
        let mut clients = self.redis_clients.write().await;
        clients.insert(name.into(), Arc::new(RwLock::new(client)));
    }

    /// Gets a Redis client by name.
    pub async fn redis(&self, name: &str) -> Option<Arc<RwLock<RedisClient>>> {
        let clients = self.redis_clients.read().await;
        clients.get(name).cloned()
    }

    /// Registers a NATS client.
    pub async fn register_nats(&self, name: impl Into<String>, client: NatsClient) {
        let mut clients = self.nats_clients.write().await;
        clients.insert(name.into(), Arc::new(RwLock::new(client)));
    }

    /// Gets a NATS client by name.
    pub async fn nats(&self, name: &str) -> Option<Arc<RwLock<NatsClient>>> {
        let clients = self.nats_clients.read().await;
        clients.get(name).cloned()
    }

    /// Creates and registers a client from configuration.
    pub async fn create_from_config(
        &self,
        name: impl Into<String>,
        config: EndpointConfig,
    ) -> Result<(), IntegrationError> {
        let name = name.into();
        match config.endpoint_type {
            EndpointType::Http => {
                let client = HttpClient::with_base_url(&config.url);
                self.register_http(name, client).await;
            }
            EndpointType::WebSocket => {
                let client = WebSocketClient::new(&config.url);
                self.register_websocket(name, client).await;
            }
            EndpointType::Mqtt => {
                let parts: Vec<&str> = config.url.split(':').collect();
                let host = parts.first().unwrap_or(&"localhost");
                let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1883);
                let client_id = config
                    .options
                    .get("client_id")
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                let client = MqttClient::new(&client_id, *host, port);
                self.register_mqtt(name, client).await;
            }
            EndpointType::Redis => {
                let client = RedisClient::new(&config.url);
                self.register_redis(name, client).await;
            }
            EndpointType::Nats => {
                let client = NatsClient::new(&config.url);
                self.register_nats(name, client).await;
            }
        }
        Ok(())
    }

    /// Lists all registered HTTP client names.
    pub async fn list_http_clients(&self) -> Vec<String> {
        self.http_clients.read().await.keys().cloned().collect()
    }

    /// Lists all registered WebSocket client names.
    pub async fn list_websocket_clients(&self) -> Vec<String> {
        self.ws_clients.read().await.keys().cloned().collect()
    }

    /// Lists all registered MQTT client names.
    pub async fn list_mqtt_clients(&self) -> Vec<String> {
        self.mqtt_clients.read().await.keys().cloned().collect()
    }

    /// Lists all registered Redis client names.
    pub async fn list_redis_clients(&self) -> Vec<String> {
        self.redis_clients.read().await.keys().cloned().collect()
    }

    /// Lists all registered NATS client names.
    pub async fn list_nats_clients(&self) -> Vec<String> {
        self.nats_clients.read().await.keys().cloned().collect()
    }
}

impl Default for IntegrationGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_creation() {
        let gateway = IntegrationGateway::new();
        assert!(gateway.list_http_clients().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_http_client() {
        let gateway = IntegrationGateway::new();
        let client = HttpClient::with_base_url("https://api.example.com");
        gateway.register_http("api", client).await;

        assert!(gateway.http("api").await.is_some());
        assert!(gateway.http("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_create_from_config() {
        let gateway = IntegrationGateway::new();
        let config = EndpointConfig {
            endpoint_type: EndpointType::Http,
            url: "https://api.example.com".to_string(),
            options: HashMap::new(),
        };

        gateway.create_from_config("api", config).await.unwrap();
        assert!(gateway.http("api").await.is_some());
    }

    // ── Lookup miss returns None ──────────────────────────────────────

    #[tokio::test]
    async fn test_http_lookup_miss() {
        let gw = IntegrationGateway::new();
        assert!(gw.http("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_mqtt_lookup_miss() {
        let gw = IntegrationGateway::new();
        assert!(gw.mqtt("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_redis_lookup_miss() {
        let gw = IntegrationGateway::new();
        assert!(gw.redis("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_nats_lookup_miss() {
        let gw = IntegrationGateway::new();
        assert!(gw.nats("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_websocket_lookup_miss() {
        let gw = IntegrationGateway::new();
        assert!(gw.websocket("nonexistent").await.is_none());
    }

    // ── List clients empty ────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_all_empty() {
        let gw = IntegrationGateway::new();
        assert!(gw.list_http_clients().await.is_empty());
        assert!(gw.list_mqtt_clients().await.is_empty());
        assert!(gw.list_redis_clients().await.is_empty());
        assert!(gw.list_nats_clients().await.is_empty());
    }

    // ── Multiple registrations ────────────────────────────────────────

    #[tokio::test]
    async fn test_register_multiple_http_clients() {
        let gw = IntegrationGateway::new();
        gw.register_http("api1", HttpClient::with_base_url("https://a.com"))
            .await;
        gw.register_http("api2", HttpClient::with_base_url("https://b.com"))
            .await;

        let clients = gw.list_http_clients().await;
        assert_eq!(clients.len(), 2);
        assert!(gw.http("api1").await.is_some());
        assert!(gw.http("api2").await.is_some());
    }

    // ── Overwrite registration ────────────────────────────────────────

    #[tokio::test]
    async fn test_overwrite_http_client() {
        let gw = IntegrationGateway::new();
        gw.register_http("api", HttpClient::with_base_url("https://old.com"))
            .await;
        gw.register_http("api", HttpClient::with_base_url("https://new.com"))
            .await;

        let clients = gw.list_http_clients().await;
        assert_eq!(clients.len(), 1);
    }

    // ── create_from_config variants ───────────────────────────────────

    #[tokio::test]
    async fn test_create_websocket_from_config() {
        let gw = IntegrationGateway::new();
        let config = EndpointConfig {
            endpoint_type: EndpointType::WebSocket,
            url: "ws://localhost:8080".to_string(),
            options: HashMap::new(),
        };
        gw.create_from_config("ws1", config).await.unwrap();
        assert!(gw.websocket("ws1").await.is_some());
    }
}
