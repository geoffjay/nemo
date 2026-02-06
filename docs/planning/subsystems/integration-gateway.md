# Integration Gateway Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Integration Gateway enables Nemo applications to communicate with external systems through standardized protocols. It provides RPC (request/response), PubSub (publish/subscribe), and Message Queue patterns—making Nemo a first-class participant in larger system architectures.

This subsystem is critical for the "modular system for other developers to plug into" requirement.

## Responsibilities

1. **Protocol Implementation:** Support RPC, PubSub, and Message Queue patterns
2. **Connection Management:** Handle connection lifecycle, reconnection, health checks
3. **Serialization:** Convert between Nemo values and wire formats
4. **Error Handling:** Graceful degradation, circuit breaking, retry logic
5. **Service Discovery:** Find and connect to services dynamically
6. **Security:** Authentication, authorization, encryption

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Integration Gateway                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    Protocol Adapters                                    │ │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ │ │
│  │  │ JSON-RPC  │ │   gRPC    │ │   MQTT    │ │   NATS    │ │   Redis   │ │ │
│  │  └───────────┘ └───────────┘ └───────────┘ └───────────┘ └───────────┘ │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                   Connection Manager                                    │ │
│  │  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐   │ │
│  │  │  Pool Management  │  │  Health Checking  │  │  Circuit Breaker  │   │ │
│  │  └───────────────────┘  └───────────────────┘  └───────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                   Message Router                                        │ │
│  │  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐   │ │
│  │  │    RPC Router     │  │   Topic Router    │  │   Queue Router    │   │ │
│  │  └───────────────────┘  └───────────────────┘  └───────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                   Integration API                                       │ │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐               │ │
│  │  │RpcClient  │ │PubSubClient│ │QueueClient│ │RpcServer  │               │ │
│  │  └───────────┘ └───────────┘ └───────────┘ └───────────┘               │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Communication Patterns

### 1. RPC (Request/Response)

Synchronous-style communication for querying and commanding external services.

```
┌──────────┐    Request     ┌──────────┐
│  Client  │───────────────▶│  Server  │
│          │◀───────────────│          │
└──────────┘    Response    └──────────┘
```

**Use Cases:**
- Query external APIs
- Execute commands on remote systems
- Fetch configuration from services
- Authenticate with identity providers

### 2. PubSub (Publish/Subscribe)

Asynchronous event-driven communication through topics.

```
┌──────────┐    Publish     ┌──────────┐    Deliver    ┌──────────┐
│Publisher │───────────────▶│  Broker  │──────────────▶│Subscriber│
└──────────┘                └──────────┘               └──────────┘
                                 │                     ┌──────────┐
                                 └────────────────────▶│Subscriber│
                                                       └──────────┘
```

**Use Cases:**
- Real-time data feeds
- Event notifications
- Collaborative features
- System-wide broadcasts

### 3. Message Queue

Reliable message delivery with persistence and acknowledgment.

```
┌──────────┐    Enqueue     ┌──────────┐    Dequeue    ┌──────────┐
│ Producer │───────────────▶│  Queue   │──────────────▶│ Consumer │
└──────────┘                └──────────┘               └──────────┘
                                 │                          │
                                 │◀────────── Ack ──────────┘
```

**Use Cases:**
- Task distribution
- Reliable command execution
- Asynchronous processing
- Work queues

---

## Core Components

### 1. ProtocolAdapter Trait

**Purpose:** Abstract interface for protocol implementations.

```rust
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Protocol name (e.g., "json-rpc", "grpc", "mqtt")
    fn protocol(&self) -> &str;
    
    /// Connect to endpoint
    async fn connect(&mut self, endpoint: &Endpoint) -> Result<(), ConnectionError>;
    
    /// Disconnect
    async fn disconnect(&mut self) -> Result<(), ConnectionError>;
    
    /// Connection status
    fn status(&self) -> ConnectionStatus;
    
    /// Send request and await response (RPC)
    async fn request(&self, request: Request) -> Result<Response, RpcError>;
    
    /// Publish message to topic (PubSub)
    async fn publish(&self, topic: &str, message: Message) -> Result<(), PublishError>;
    
    /// Subscribe to topic (PubSub)
    async fn subscribe(&self, topic: &str) -> Result<Subscription, SubscribeError>;
    
    /// Send to queue (MQ)
    async fn enqueue(&self, queue: &str, message: Message) -> Result<(), EnqueueError>;
    
    /// Receive from queue (MQ)
    async fn dequeue(&self, queue: &str) -> Result<QueueMessage, DequeueError>;
}

pub struct Endpoint {
    pub address: String,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub tls: TlsConfig,
    pub auth: Option<AuthConfig>,
}

pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}
```

### 2. Protocol Implementations

#### JSON-RPC Adapter

```rust
pub struct JsonRpcAdapter {
    client: reqwest::Client,
    endpoint: Option<Endpoint>,
    request_id: AtomicU64,
}

impl JsonRpcAdapter {
    pub fn new(config: JsonRpcConfig) -> Self;
}

pub struct JsonRpcConfig {
    pub version: JsonRpcVersion,  // "2.0"
    pub timeout: Duration,
    pub batch_size: Option<usize>,
}
```

**Configuration:**
```hcl
integration "backend_api" {
  type = "json-rpc"
  
  endpoint {
    address = "https://api.example.com"
    path    = "/rpc"
  }
  
  config {
    version = "2.0"
    timeout = "30s"
  }
  
  auth {
    type = "bearer"
    token = "${env.API_TOKEN}"
  }
}
```

#### MQTT Adapter

```rust
pub struct MqttAdapter {
    client: rumqttc::AsyncClient,
    event_loop: rumqttc::EventLoop,
    subscriptions: HashMap<String, broadcast::Sender<Message>>,
}

impl MqttAdapter {
    pub fn new(config: MqttConfig) -> Self;
}

pub struct MqttConfig {
    pub client_id: String,
    pub keep_alive: Duration,
    pub clean_session: bool,
    pub qos: QoS,
}
```

**Configuration:**
```hcl
integration "iot_broker" {
  type = "mqtt"
  
  endpoint {
    address = "mqtt.example.com"
    port    = 8883
    tls     = true
  }
  
  config {
    client_id     = "nemo-${env.INSTANCE_ID}"
    keep_alive    = "60s"
    clean_session = true
    qos           = 1
  }
  
  auth {
    type     = "basic"
    username = "${env.MQTT_USER}"
    password = "${env.MQTT_PASS}"
  }
}
```

#### Redis Adapter (PubSub + Streams)

```rust
pub struct RedisAdapter {
    client: redis::aio::MultiplexedConnection,
    pubsub: Option<redis::aio::PubSub>,
}

impl RedisAdapter {
    pub fn new(config: RedisConfig) -> Self;
}

pub struct RedisConfig {
    pub database: u8,
    pub pool_size: usize,
    pub consumer_group: Option<String>,  // For streams
}
```

**Configuration:**
```hcl
integration "redis_events" {
  type = "redis"
  
  endpoint {
    address = "redis.example.com"
    port    = 6379
  }
  
  config {
    database   = 0
    pool_size  = 10
    
    # For Redis Streams (message queue pattern)
    consumer_group = "nemo-consumers"
  }
  
  auth {
    type     = "password"
    password = "${env.REDIS_PASSWORD}"
  }
}
```

#### NATS Adapter

```rust
pub struct NatsAdapter {
    client: async_nats::Client,
    jetstream: Option<async_nats::jetstream::Context>,
}

impl NatsAdapter {
    pub fn new(config: NatsConfig) -> Self;
}

pub struct NatsConfig {
    pub name: String,
    pub jetstream: bool,  // Enable JetStream for persistence
}
```

### 3. ConnectionManager

**Purpose:** Manage connection lifecycle, pooling, and health.

```rust
pub struct ConnectionManager {
    connections: HashMap<String, ManagedConnection>,
    health_checker: HealthChecker,
    circuit_breakers: HashMap<String, CircuitBreaker>,
}

impl ConnectionManager {
    /// Get or create connection
    pub async fn get_connection(&mut self, integration_id: &str) -> Result<&dyn ProtocolAdapter, ConnectionError>;
    
    /// Health check all connections
    pub async fn health_check(&self) -> HashMap<String, HealthStatus>;
    
    /// Force reconnect
    pub async fn reconnect(&mut self, integration_id: &str) -> Result<(), ConnectionError>;
    
    /// Shutdown all connections
    pub async fn shutdown(&mut self);
}

pub struct ManagedConnection {
    adapter: Box<dyn ProtocolAdapter>,
    config: IntegrationConfig,
    last_used: Instant,
    error_count: u32,
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure: Option<Instant>,
}

pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}
```

### 4. MessageRouter

**Purpose:** Route messages between integrations and internal handlers.

```rust
pub struct MessageRouter {
    rpc_handlers: HashMap<String, Box<dyn RpcHandler>>,
    topic_subscriptions: HashMap<String, Vec<broadcast::Sender<Message>>>,
    queue_consumers: HashMap<String, Vec<mpsc::Sender<QueueMessage>>>,
}

impl MessageRouter {
    // ---- RPC Server ----
    
    /// Register RPC handler
    pub fn register_rpc(&mut self, method: &str, handler: Box<dyn RpcHandler>);
    
    /// Handle incoming RPC request
    pub async fn handle_rpc(&self, request: Request) -> Response;
    
    // ---- PubSub ----
    
    /// Subscribe to topic
    pub fn subscribe_topic(&mut self, pattern: &str) -> broadcast::Receiver<Message>;
    
    /// Route incoming message to subscribers
    pub fn route_topic_message(&self, topic: &str, message: Message);
    
    // ---- Message Queue ----
    
    /// Register queue consumer
    pub fn register_consumer(&mut self, queue: &str, consumer: mpsc::Sender<QueueMessage>);
    
    /// Route message to consumer
    pub async fn route_queue_message(&self, queue: &str, message: QueueMessage);
}

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle(&self, params: Value) -> Result<Value, RpcError>;
}
```

---

## Integration API

### Client APIs

```rust
/// RPC Client for request/response communication
pub struct RpcClient {
    connection_manager: Arc<ConnectionManager>,
    integration_id: String,
}

impl RpcClient {
    /// Call remote method
    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<T, RpcError>;
    
    /// Call with timeout
    pub async fn call_with_timeout<T: DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
        timeout: Duration,
    ) -> Result<T, RpcError>;
    
    /// Batch multiple calls
    pub async fn batch(&self, calls: Vec<RpcCall>) -> Vec<Result<Value, RpcError>>;
}

/// PubSub Client for event-driven communication
pub struct PubSubClient {
    connection_manager: Arc<ConnectionManager>,
    integration_id: String,
}

impl PubSubClient {
    /// Publish message to topic
    pub async fn publish(&self, topic: &str, message: impl Serialize) -> Result<(), PublishError>;
    
    /// Subscribe to topic pattern
    pub async fn subscribe(&self, pattern: &str) -> Subscription;
}

pub struct Subscription {
    receiver: broadcast::Receiver<Message>,
}

impl Subscription {
    /// Receive next message
    pub async fn recv(&mut self) -> Option<Message>;
    
    /// Convert to stream
    pub fn into_stream(self) -> impl Stream<Item = Message>;
}

/// Queue Client for reliable messaging
pub struct QueueClient {
    connection_manager: Arc<ConnectionManager>,
    integration_id: String,
}

impl QueueClient {
    /// Send message to queue
    pub async fn send(&self, queue: &str, message: impl Serialize) -> Result<MessageId, EnqueueError>;
    
    /// Receive message from queue
    pub async fn receive(&self, queue: &str) -> Result<QueueMessage, DequeueError>;
    
    /// Acknowledge message
    pub async fn ack(&self, message_id: &MessageId) -> Result<(), AckError>;
    
    /// Negative acknowledge (requeue)
    pub async fn nack(&self, message_id: &MessageId) -> Result<(), AckError>;
}
```

### Server APIs

```rust
/// RPC Server for handling incoming requests
pub struct RpcServer {
    router: Arc<MessageRouter>,
    integration_id: String,
}

impl RpcServer {
    /// Register method handler
    pub fn register(&mut self, method: &str, handler: impl RpcHandler + 'static);
    
    /// Register method with closure
    pub fn register_fn<F, Fut>(&mut self, method: &str, f: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, RpcError>> + Send;
    
    /// Start serving
    pub async fn serve(&self) -> Result<(), ServerError>;
}
```

---

## Configuration

### Full Integration Example

```hcl
# RPC integration for backend API
integration "backend" {
  type = "json-rpc"
  
  endpoint {
    address = "${var.api_url}"
    tls     = true
  }
  
  auth {
    type  = "bearer"
    token = "${env.API_TOKEN}"
  }
  
  retry {
    max_attempts = 3
    backoff      = "exponential"
    initial      = "100ms"
    max          = "10s"
  }
  
  circuit_breaker {
    failure_threshold = 5
    success_threshold = 2
    timeout           = "30s"
  }
  
  health_check {
    enabled  = true
    interval = "30s"
    method   = "health.check"
  }
}

# PubSub integration for real-time events
integration "events" {
  type = "mqtt"
  
  endpoint {
    address = "mqtt.example.com"
    port    = 8883
    tls     = true
  }
  
  config {
    client_id     = "nemo-${env.INSTANCE_ID}"
    keep_alive    = "60s"
    qos           = 1
  }
  
  reconnect {
    enabled      = true
    max_attempts = -1  # Infinite
    delay        = "1s"
    max_delay    = "60s"
  }
}

# Message queue for task processing
integration "tasks" {
  type = "redis"
  
  endpoint {
    address = "redis.example.com"
  }
  
  config {
    database       = 1
    consumer_group = "nemo-workers"
  }
}
```

### Using Integrations in Data Flow

```hcl
# RPC as data source
data "rpc" "user_data" {
  integration = integration.backend
  method      = "users.list"
  params      = { status = "active" }
  interval    = "5m"
}

# PubSub subscription as data source
data "subscription" "price_updates" {
  integration = integration.events
  topic       = "prices/#"
  
  transform {
    type = "jq"
    query = ".payload"
  }
}

# Action that uses RPC
action "create_order" {
  type = "rpc"
  
  config {
    integration = integration.backend
    method      = "orders.create"
    params      = { /* from trigger */ }
  }
}

# Action that publishes event
action "notify_created" {
  type = "publish"
  
  config {
    integration = integration.events
    topic       = "orders/created"
    message     = { order_id = "${trigger.result.id}" }
  }
}
```

### Exposing RPC Server

```hcl
# Expose Nemo as RPC server
server "nemo_api" {
  type = "json-rpc"
  
  listen {
    address = "0.0.0.0"
    port    = 8080
    tls     = true
    cert    = "${var.tls_cert}"
    key     = "${var.tls_key}"
  }
  
  # Methods exposed
  method "data.get" {
    handler = "script.api_handlers.get_data"
    auth    = required
  }
  
  method "action.execute" {
    handler = "script.api_handlers.execute_action"
    auth    = required
  }
  
  auth {
    type   = "jwt"
    secret = "${env.JWT_SECRET}"
  }
}
```

---

## Serialization

### Wire Formats

| Protocol | Default Format | Alternatives |
|----------|---------------|--------------|
| JSON-RPC | JSON | - |
| gRPC | Protobuf | JSON |
| MQTT | MessagePack | JSON, Binary |
| NATS | JSON | MessagePack, Binary |
| Redis | JSON | MessagePack |

### Type Conversion

```rust
pub trait WireFormat {
    fn serialize(&self, value: &Value) -> Result<Vec<u8>, SerializeError>;
    fn deserialize(&self, bytes: &[u8]) -> Result<Value, DeserializeError>;
}

pub struct JsonFormat;
pub struct MessagePackFormat;
pub struct ProtobufFormat { schema: prost_reflect::DescriptorPool }
```

---

## Error Handling

### Error Types

```rust
pub enum IntegrationError {
    /// Connection failed
    ConnectionError { integration: String, reason: String },
    
    /// Request timed out
    Timeout { integration: String, operation: String },
    
    /// Circuit breaker open
    CircuitOpen { integration: String },
    
    /// Authentication failed
    AuthError { integration: String, reason: String },
    
    /// RPC error from server
    RpcError { code: i32, message: String, data: Option<Value> },
    
    /// Serialization error
    SerializeError { reason: String },
    
    /// Message rejected by broker
    PublishError { topic: String, reason: String },
}
```

### Retry Strategies

```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub retryable_errors: Vec<ErrorClass>,
}

pub enum BackoffStrategy {
    Constant(Duration),
    Linear { initial: Duration, increment: Duration },
    Exponential { initial: Duration, max: Duration, factor: f64 },
    Jittered { base: BackoffStrategy, jitter: f64 },
}

pub enum ErrorClass {
    Timeout,
    ConnectionReset,
    ServiceUnavailable,
    RateLimited,
}
```

---

## Security

### Authentication Methods

| Method | Use Case | Configuration |
|--------|----------|---------------|
| None | Internal/trusted | - |
| Basic | Simple services | username, password |
| Bearer | API tokens | token |
| JWT | Stateless auth | secret/public_key |
| mTLS | High security | client_cert, client_key |
| OAuth2 | External APIs | client_id, client_secret, scopes |

### TLS Configuration

```rust
pub struct TlsConfig {
    pub enabled: bool,
    pub verify_server: bool,
    pub ca_cert: Option<PathBuf>,
    pub client_cert: Option<PathBuf>,
    pub client_key: Option<PathBuf>,
    pub min_version: TlsVersion,
}
```

---

## Observability

### Metrics

- Connection pool utilization
- Request latency (p50, p95, p99)
- Error rates by integration and error type
- Circuit breaker state changes
- Message throughput (pub/sub, queue)
- Retry counts

### Tracing

```rust
pub struct IntegrationSpan {
    pub integration_id: String,
    pub operation: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, Value>,
}
```

---

## Agent Prompt Considerations

When creating an agent to implement the Integration Gateway:

- **Async expertise:** Heavy async/await, connection management
- **Protocol knowledge:** Understand RPC, PubSub, MQ semantics
- **Resilience patterns:** Circuit breakers, retries, timeouts
- **Security awareness:** Auth methods, TLS, secrets handling
- **Testing:** Mock servers, network partitions, error scenarios
- **Performance:** Connection pooling, batching, backpressure

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
