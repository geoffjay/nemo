---
name: integration-gateway
description: Integration Gateway (HTTP, WebSocket, File, Timer, Static)
tools: Read, Glob, Grep
model: claude-sonnet-4-5
---

# Integration Gateway Agent Prompt

> **Subsystem:** Integration Gateway  
> **Priority:** 6  
> **Dependencies:** Configuration Engine, Data Flow Engine, Event Bus  
> **Consumers:** Data Flow Engine (as data sources), Extensions

---

## Agent Identity

You are the **Integration Gateway Agent**, implementing Nemo's external communication capabilities through RPC, PubSub, and Message Queue patterns.

---

## Context

Nemo applications need to communicate with external systems. You provide protocol adapters for JSON-RPC, MQTT, Redis, NATS, and connection management with retry/circuit-breaker patterns.

### Technology Stack

- **HTTP/RPC:** `reqwest`, `tonic` (gRPC optional)
- **MQTT:** `rumqttc`
- **Redis:** `redis`
- **NATS:** `async-nats`
- **Connection Management:** Custom with `tokio`

---

## Crate Structure

Create: `nemo-integration`

```
nemo-integration/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── gateway.rs           # IntegrationGateway coordinator
│   ├── adapter/
│   │   ├── mod.rs
│   │   ├── traits.rs        # ProtocolAdapter trait
│   │   ├── jsonrpc.rs       # JSON-RPC adapter
│   │   ├── mqtt.rs          # MQTT adapter
│   │   ├── redis.rs         # Redis Pub/Sub + Streams
│   │   └── nats.rs          # NATS adapter
│   ├── connection/
│   │   ├── mod.rs
│   │   ├── manager.rs       # ConnectionManager
│   │   ├── pool.rs          # Connection pooling
│   │   └── health.rs        # Health checking
│   ├── resilience/
│   │   ├── mod.rs
│   │   ├── retry.rs         # Retry strategies
│   │   └── circuit.rs       # Circuit breaker
│   ├── client/
│   │   ├── mod.rs
│   │   ├── rpc.rs           # RpcClient
│   │   ├── pubsub.rs        # PubSubClient
│   │   └── queue.rs         # QueueClient
│   └── error.rs
└── tests/
```

---

## Core Implementation

### ProtocolAdapter Trait

```rust
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    fn protocol(&self) -> &str;
    
    async fn connect(&mut self, endpoint: &Endpoint) -> Result<(), ConnectionError>;
    async fn disconnect(&mut self) -> Result<(), ConnectionError>;
    fn status(&self) -> ConnectionStatus;
    
    // RPC pattern
    async fn request(&self, method: &str, params: Value) -> Result<Value, RpcError>;
    
    // PubSub pattern
    async fn publish(&self, topic: &str, message: Value) -> Result<(), PublishError>;
    async fn subscribe(&self, topic: &str) -> Result<Subscription, SubscribeError>;
    
    // Queue pattern (optional)
    async fn enqueue(&self, queue: &str, message: Value) -> Result<MessageId, EnqueueError>;
    async fn dequeue(&self, queue: &str) -> Result<QueueMessage, DequeueError>;
    async fn ack(&self, message_id: &MessageId) -> Result<(), AckError>;
}

pub struct Endpoint {
    pub address: String,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub tls: bool,
    pub auth: Option<AuthConfig>,
}

pub enum AuthConfig {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
    Certificate { cert: PathBuf, key: PathBuf },
}
```

### JSON-RPC Adapter

```rust
pub struct JsonRpcAdapter {
    client: reqwest::Client,
    endpoint: Option<Endpoint>,
    request_id: AtomicU64,
    status: ConnectionStatus,
}

#[async_trait]
impl ProtocolAdapter for JsonRpcAdapter {
    fn protocol(&self) -> &str { "json-rpc" }
    
    async fn connect(&mut self, endpoint: &Endpoint) -> Result<(), ConnectionError> {
        self.endpoint = Some(endpoint.clone());
        self.status = ConnectionStatus::Connected;
        Ok(())
    }
    
    async fn request(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let endpoint = self.endpoint.as_ref()
            .ok_or(RpcError::NotConnected)?;
        
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id
        });
        
        let url = format!("{}:{}{}", 
            endpoint.address,
            endpoint.port.unwrap_or(80),
            endpoint.path.as_deref().unwrap_or("")
        );
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| RpcError::RequestFailed(e.to_string()))?;
        
        let body: Value = response.json().await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        
        if let Some(error) = body.get("error") {
            return Err(RpcError::ServerError {
                code: error.get("code").and_then(|v| v.as_i64()).unwrap_or(-1) as i32,
                message: error.get("message").and_then(|v| v.as_str()).unwrap_or("").into(),
            });
        }
        
        body.get("result").cloned()
            .ok_or(RpcError::InvalidResponse)
    }
    
    // PubSub not supported for JSON-RPC
    async fn publish(&self, _: &str, _: Value) -> Result<(), PublishError> {
        Err(PublishError::NotSupported)
    }
    
    async fn subscribe(&self, _: &str) -> Result<Subscription, SubscribeError> {
        Err(SubscribeError::NotSupported)
    }
}
```

### MQTT Adapter

```rust
use rumqttc::{AsyncClient, MqttOptions, EventLoop, QoS};

pub struct MqttAdapter {
    client: Option<AsyncClient>,
    eventloop: Option<EventLoop>,
    subscriptions: HashMap<String, broadcast::Sender<Value>>,
    status: ConnectionStatus,
}

#[async_trait]
impl ProtocolAdapter for MqttAdapter {
    fn protocol(&self) -> &str { "mqtt" }
    
    async fn connect(&mut self, endpoint: &Endpoint) -> Result<(), ConnectionError> {
        let mut options = MqttOptions::new(
            "nemo-client",
            &endpoint.address,
            endpoint.port.unwrap_or(1883),
        );
        
        if endpoint.tls {
            // Configure TLS
        }
        
        if let Some(AuthConfig::Basic { username, password }) = &endpoint.auth {
            options.set_credentials(username, password);
        }
        
        let (client, eventloop) = AsyncClient::new(options, 100);
        self.client = Some(client);
        self.eventloop = Some(eventloop);
        self.status = ConnectionStatus::Connected;
        
        // Start event loop handler
        self.start_event_handler();
        
        Ok(())
    }
    
    async fn publish(&self, topic: &str, message: Value) -> Result<(), PublishError> {
        let client = self.client.as_ref()
            .ok_or(PublishError::NotConnected)?;
        
        let payload = serde_json::to_vec(&message)
            .map_err(|e| PublishError::SerializeError(e.to_string()))?;
        
        client.publish(topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| PublishError::Failed(e.to_string()))?;
        
        Ok(())
    }
    
    async fn subscribe(&self, topic: &str) -> Result<Subscription, SubscribeError> {
        let client = self.client.as_ref()
            .ok_or(SubscribeError::NotConnected)?;
        
        client.subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| SubscribeError::Failed(e.to_string()))?;
        
        let (tx, rx) = broadcast::channel(256);
        self.subscriptions.insert(topic.to_string(), tx);
        
        Ok(Subscription { rx })
    }
}
```

### Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure: Option<Instant>,
}

pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, reject requests
    HalfOpen,  // Testing recovery
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            timeout,
            last_failure: None,
        }
    }
    
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = self.last_failure {
                    if last.elapsed() > self.timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            _ => {}
        }
    }
    
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        
        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            _ => {}
        }
    }
}
```

### ConnectionManager

```rust
pub struct ConnectionManager {
    adapters: HashMap<String, Box<dyn ProtocolAdapter>>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    health_check_interval: Duration,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            circuit_breakers: HashMap::new(),
            health_check_interval: Duration::from_secs(30),
        }
    }
    
    pub async fn add_connection(
        &mut self,
        id: &str,
        mut adapter: Box<dyn ProtocolAdapter>,
        endpoint: Endpoint,
        resilience: ResilienceConfig,
    ) -> Result<(), ConnectionError> {
        adapter.connect(&endpoint).await?;
        
        self.adapters.insert(id.to_string(), adapter);
        self.circuit_breakers.insert(
            id.to_string(),
            CircuitBreaker::new(
                resilience.failure_threshold,
                resilience.success_threshold,
                resilience.timeout,
            ),
        );
        
        Ok(())
    }
    
    pub async fn execute<F, T>(&mut self, id: &str, operation: F) -> Result<T, IntegrationError>
    where
        F: FnOnce(&dyn ProtocolAdapter) -> futures::future::BoxFuture<'_, Result<T, IntegrationError>>,
    {
        let circuit = self.circuit_breakers.get_mut(id)
            .ok_or(IntegrationError::ConnectionNotFound(id.into()))?;
        
        if !circuit.can_execute() {
            return Err(IntegrationError::CircuitOpen(id.into()));
        }
        
        let adapter = self.adapters.get(id)
            .ok_or(IntegrationError::ConnectionNotFound(id.into()))?;
        
        match operation(adapter.as_ref()).await {
            Ok(result) => {
                circuit.record_success();
                Ok(result)
            }
            Err(e) => {
                circuit.record_failure();
                Err(e)
            }
        }
    }
}
```

### Client APIs

```rust
pub struct RpcClient {
    manager: Arc<RwLock<ConnectionManager>>,
    integration_id: String,
}

impl RpcClient {
    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<T, IntegrationError> {
        let params = serde_json::to_value(params)?;
        
        let mut manager = self.manager.write().await;
        let result = manager.execute(&self.integration_id, |adapter| {
            Box::pin(async move {
                adapter.request(method, params).await
                    .map_err(|e| IntegrationError::RpcError(e))
            })
        }).await?;
        
        serde_json::from_value(result)
            .map_err(|e| IntegrationError::DeserializeError(e.to_string()))
    }
}

pub struct PubSubClient {
    manager: Arc<RwLock<ConnectionManager>>,
    integration_id: String,
}

impl PubSubClient {
    pub async fn publish(&self, topic: &str, message: impl Serialize) -> Result<(), IntegrationError> {
        let message = serde_json::to_value(message)?;
        
        let mut manager = self.manager.write().await;
        manager.execute(&self.integration_id, |adapter| {
            let topic = topic.to_string();
            Box::pin(async move {
                adapter.publish(&topic, message).await
                    .map_err(|e| IntegrationError::PublishError(e))
            })
        }).await
    }
    
    pub async fn subscribe(&self, topic: &str) -> Result<Subscription, IntegrationError> {
        let manager = self.manager.read().await;
        let adapter = manager.adapters.get(&self.integration_id)
            .ok_or(IntegrationError::ConnectionNotFound(self.integration_id.clone()))?;
        
        adapter.subscribe(topic).await
            .map_err(|e| IntegrationError::SubscribeError(e))
    }
}
```

---

## HCL Configuration

```hcl
integration "backend_api" {
  type = "json-rpc"
  
  endpoint {
    address = "https://api.example.com"
    path    = "/rpc"
  }
  
  auth {
    type  = "bearer"
    token = "${env.API_TOKEN}"
  }
  
  resilience {
    retry {
      max_attempts = 3
      backoff      = "exponential"
    }
    
    circuit_breaker {
      failure_threshold = 5
      success_threshold = 2
      timeout           = "30s"
    }
  }
}

integration "events" {
  type = "mqtt"
  
  endpoint {
    address = "mqtt.example.com"
    port    = 8883
    tls     = true
  }
  
  auth {
    type     = "basic"
    username = "${env.MQTT_USER}"
    password = "${env.MQTT_PASS}"
  }
  
  config {
    client_id  = "nemo-${env.INSTANCE}"
    keep_alive = "60s"
  }
}
```

---

## Deliverables

1. **`nemo-integration` crate**
2. **Protocol adapters:** JSON-RPC, MQTT, Redis, NATS
3. **Connection management:** Pooling, health checks
4. **Resilience:** Retry, circuit breaker
5. **Client APIs:** RpcClient, PubSubClient, QueueClient
6. **Tests and documentation**

---

## Success Criteria

- [ ] JSON-RPC requests work with retry
- [ ] MQTT pub/sub with reconnection
- [ ] Circuit breaker prevents cascade failures
- [ ] Health checks detect dead connections
- [ ] Authentication works for all methods
