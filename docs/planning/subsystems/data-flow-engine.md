# Data Flow Engine Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Data Flow Engine is the nervous system of Nemo. It manages the complete lifecycle of data: collection from various sources, transformation through pipelines, storage in typed repositories, triggering actions based on conditions, and binding data to UI components for display.

This subsystem embodies the core promise of Nemo: **strong connections between data collection, data storage, data-driven action, and data display**.

## Responsibilities

1. **Collection:** Gather data from diverse sources (HTTP, WebSocket, files, timers)
2. **Transformation:** Process data through configurable pipelines
3. **Storage:** Maintain typed, observable data repositories
4. **Actions:** Trigger operations based on data conditions
5. **Binding:** Connect data to UI components reactively
6. **Observation:** Enable introspection and debugging of data flow

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Data Flow Engine                                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                         Data Sources                                    │ │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐   │ │
│  │  │  HTTP   │ │WebSocket│ │  File   │ │  Timer  │ │ Custom (Plugin) │   │ │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────────┬────────┘   │ │
│  └───────┼──────────┼──────────┼──────────┼────────────────┼──────────────┘ │
│          └──────────┴──────────┴──────────┴────────────────┘                │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Transform Pipeline                                 │ │
│  │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐                  │ │
│  │  │   Map   │──▶│  Filter │──▶│Aggregate│──▶│  Custom │                  │ │
│  │  └─────────┘   └─────────┘   └─────────┘   └─────────┘                  │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Data Repository                                    │ │
│  │  ┌─────────────────────────────────────────────────────────────────┐    │ │
│  │  │  Typed Storage  │  Change Tracking  │  Query Interface          │    │ │
│  │  └─────────────────────────────────────────────────────────────────┘    │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                         │                   │                                │
│            ┌────────────┴───────┐   ┌──────┴──────┐                         │
│            ▼                    ▼   ▼             ▼                          │
│  ┌─────────────────┐   ┌─────────────────┐  ┌─────────────────┐             │
│  │  Action System  │   │  Binding System │  │  Event Emitter  │             │
│  └─────────────────┘   └─────────────────┘  └─────────────────┘             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. DataSource Trait

**Purpose:** Abstract interface for all data collection mechanisms.

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Unique identifier for this source
    fn id(&self) -> &str;
    
    /// Schema of data this source produces
    fn schema(&self) -> &DataSchema;
    
    /// Start collecting data
    async fn start(&mut self) -> Result<(), DataSourceError>;
    
    /// Stop collecting data
    async fn stop(&mut self) -> Result<(), DataSourceError>;
    
    /// Subscribe to data updates
    fn subscribe(&self) -> broadcast::Receiver<DataUpdate>;
    
    /// Current connection status
    fn status(&self) -> SourceStatus;
    
    /// Manual refresh (for polling sources)
    async fn refresh(&mut self) -> Result<(), DataSourceError>;
}

pub enum SourceStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

pub struct DataUpdate {
    pub source_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
    pub update_type: UpdateType,
}

pub enum UpdateType {
    Full,        // Complete replacement
    Partial,     // Patch/delta
    Append,      // Add to collection
    Delete,      // Remove from collection
}
```

### 2. Built-in Data Sources

#### HTTP Polling Source

```rust
pub struct HttpSource {
    config: HttpSourceConfig,
    client: reqwest::Client,
    status: SourceStatus,
    sender: broadcast::Sender<DataUpdate>,
}

pub struct HttpSourceConfig {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub interval: Duration,
    pub timeout: Duration,
    pub retry: RetryConfig,
    pub auth: Option<AuthConfig>,
}
```

**Configuration:**
```hcl
data "http" "api_data" {
  url      = "https://api.example.com/data"
  method   = "GET"
  interval = "30s"
  timeout  = "10s"
  
  headers = {
    "Authorization" = "Bearer ${var.api_token}"
  }
  
  retry {
    max_attempts = 3
    backoff      = "exponential"
  }
}
```

#### WebSocket Source

```rust
pub struct WebSocketSource {
    config: WebSocketConfig,
    connection: Option<WebSocketConnection>,
    status: SourceStatus,
    sender: broadcast::Sender<DataUpdate>,
}

pub struct WebSocketConfig {
    pub url: String,
    pub protocols: Vec<String>,
    pub headers: HashMap<String, String>,
    pub reconnect: ReconnectConfig,
    pub heartbeat: Option<HeartbeatConfig>,
}
```

**Configuration:**
```hcl
data "websocket" "live_feed" {
  url = "wss://stream.example.com/feed"
  
  reconnect {
    enabled      = true
    max_attempts = 10
    delay        = "1s"
    max_delay    = "30s"
  }
  
  heartbeat {
    interval = "30s"
    message  = "{\"type\": \"ping\"}"
  }
}
```

#### File Watcher Source

```rust
pub struct FileSource {
    config: FileSourceConfig,
    watcher: notify::RecommendedWatcher,
    status: SourceStatus,
    sender: broadcast::Sender<DataUpdate>,
}

pub struct FileSourceConfig {
    pub path: PathBuf,
    pub format: FileFormat,
    pub watch: bool,
    pub debounce: Duration,
}

pub enum FileFormat {
    Json,
    Yaml,
    Toml,
    Csv,
    Lines,
    Raw,
}
```

#### Timer Source

```rust
pub struct TimerSource {
    config: TimerConfig,
    status: SourceStatus,
    sender: broadcast::Sender<DataUpdate>,
}

pub struct TimerConfig {
    pub interval: Duration,
    pub immediate: bool,  // Emit immediately on start
    pub payload: Value,   // Static payload to emit
}
```

**Configuration:**
```hcl
data "timer" "clock" {
  interval  = "1s"
  immediate = true
  
  payload = {
    type = "tick"
  }
}
```

### 3. Transform Pipeline

**Purpose:** Process data through a series of transformations.

```rust
pub trait Transform: Send + Sync {
    /// Transform input data
    fn transform(&self, input: Value, context: &TransformContext) -> Result<Value, TransformError>;
    
    /// Schema transformation (for type checking)
    fn transform_schema(&self, input: &DataSchema) -> DataSchema;
}

pub struct TransformContext {
    pub source_id: String,
    pub timestamp: DateTime<Utc>,
    pub variables: HashMap<String, Value>,
}

pub struct Pipeline {
    transforms: Vec<Box<dyn Transform>>,
}

impl Pipeline {
    pub fn execute(&self, input: Value, context: &TransformContext) -> Result<Value, PipelineError> {
        let mut current = input;
        for (i, transform) in self.transforms.iter().enumerate() {
            current = transform.transform(current, context)
                .map_err(|e| PipelineError::TransformFailed { stage: i, error: e })?;
        }
        Ok(current)
    }
}
```

#### Built-in Transforms

| Transform | Purpose | Configuration |
|-----------|---------|---------------|
| `map` | Transform each item | RHAI expression or path mapping |
| `filter` | Keep matching items | RHAI predicate expression |
| `select` | Extract fields | List of field paths |
| `flatten` | Flatten nested arrays | Depth parameter |
| `aggregate` | Combine items | Aggregation function |
| `window` | Time/count windows | Window size and slide |
| `join` | Combine data sources | Join key and type |
| `sort` | Order items | Sort key and direction |
| `distinct` | Remove duplicates | Key field |
| `jq` | JQ query language | JQ expression |
| `rhai` | Custom RHAI script | Script path or inline |

**Configuration:**
```hcl
data "http" "raw_data" {
  url = "https://api.example.com/items"
  
  transform {
    type = "filter"
    expr = "item.status == 'active'"
  }
  
  transform {
    type = "map"
    expr = """
      #{
        id: item.id,
        name: item.attributes.name,
        value: item.metrics.value
      }
    """
  }
  
  transform {
    type = "sort"
    key  = "value"
    desc = true
  }
  
  transform {
    type  = "window"
    size  = 100
    slide = 10
  }
}
```

### 4. Data Repository

**Purpose:** Store, index, and provide access to data with change notification.

```rust
pub struct DataRepository {
    stores: HashMap<String, Box<dyn DataStore>>,
    change_notifier: broadcast::Sender<RepositoryChange>,
}

impl DataRepository {
    /// Register a data store
    pub fn register(&mut self, id: &str, store: Box<dyn DataStore>);
    
    /// Get data by path
    pub fn get(&self, path: &DataPath) -> Option<Value>;
    
    /// Set data at path
    pub fn set(&mut self, path: &DataPath, value: Value) -> Result<(), RepositoryError>;
    
    /// Query data
    pub fn query(&self, query: &DataQuery) -> Result<Vec<Value>, QueryError>;
    
    /// Subscribe to changes
    pub fn subscribe(&self) -> broadcast::Receiver<RepositoryChange>;
    
    /// Subscribe to specific path changes
    pub fn subscribe_path(&self, path: &DataPath) -> broadcast::Receiver<RepositoryChange>;
}

pub trait DataStore: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&mut self, key: &str, value: Value);
    fn delete(&mut self, key: &str);
    fn keys(&self) -> Vec<String>;
    fn clear(&mut self);
}

pub struct RepositoryChange {
    pub path: DataPath,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub timestamp: DateTime<Utc>,
}
```

#### Store Types

```rust
/// In-memory store (default)
pub struct MemoryStore {
    data: HashMap<String, Value>,
}

/// Persistent store (SQLite-backed)
pub struct PersistentStore {
    db: rusqlite::Connection,
    cache: HashMap<String, Value>,
}

/// Cached store (wraps another store with LRU cache)
pub struct CachedStore {
    inner: Box<dyn DataStore>,
    cache: LruCache<String, Value>,
}
```

### 5. Action System

**Purpose:** Execute operations based on data conditions.

```rust
pub struct ActionSystem {
    triggers: Vec<ActionTrigger>,
    actions: HashMap<String, Box<dyn Action>>,
}

impl ActionSystem {
    /// Register an action
    pub fn register_action(&mut self, name: &str, action: Box<dyn Action>);
    
    /// Add a trigger
    pub fn add_trigger(&mut self, trigger: ActionTrigger);
    
    /// Process a data change (called by repository)
    pub fn on_data_changed(&mut self, change: &RepositoryChange, cx: &mut Context<Self>);
}

pub struct ActionTrigger {
    pub id: String,
    pub condition: TriggerCondition,
    pub action: String,
    pub action_params: Value,
    pub debounce: Option<Duration>,
    pub throttle: Option<Duration>,
}

pub enum TriggerCondition {
    /// Trigger when path changes
    PathChanged(DataPath),
    
    /// Trigger when expression evaluates to true
    Expression { path: DataPath, expr: String },
    
    /// Trigger when value crosses threshold
    Threshold { path: DataPath, threshold: Value, direction: ThresholdDirection },
    
    /// Trigger on any update to path
    AnyUpdate(DataPath),
}

#[async_trait]
pub trait Action: Send + Sync {
    async fn execute(&self, params: Value, context: &ActionContext) -> Result<Value, ActionError>;
}
```

**Configuration:**
```hcl
# Named action definition
action "notify_user" {
  type = "notification"
  
  config {
    title   = "Alert"
    message = "${trigger.value} exceeded threshold"
    level   = "warning"
  }
}

action "refresh_panel" {
  type = "ui"
  
  config {
    action = "refresh"
    target = "panel.main-view"
  }
}

# Trigger definitions
on "data.metrics.cpu" {
  condition {
    type      = "threshold"
    threshold = 90
    direction = "above"
  }
  
  action = "notify_user"
  debounce = "10s"
}

on "data.api_data.updated" {
  action = "refresh_panel"
}
```

#### Built-in Actions

| Action Type | Purpose | Parameters |
|-------------|---------|------------|
| `notification` | Show notification | title, message, level |
| `ui.refresh` | Refresh component | target component ID |
| `ui.navigate` | Navigate/focus | target component ID |
| `ui.toggle` | Toggle visibility | target component ID |
| `data.set` | Update data | path, value |
| `data.delete` | Delete data | path |
| `http.request` | Make HTTP request | url, method, body |
| `script.run` | Execute RHAI script | script path, params |
| `sequence` | Run actions in order | list of actions |
| `parallel` | Run actions concurrently | list of actions |
| `conditional` | Branch on condition | condition, if_true, if_false |

### 6. Binding System

**Purpose:** Connect data to UI components reactively.

```rust
pub struct BindingSystem {
    bindings: HashMap<BindingId, Binding>,
    path_index: HashMap<DataPath, Vec<BindingId>>,
}

impl BindingSystem {
    /// Create a new binding
    pub fn create_binding(
        &mut self,
        source: DataPath,
        target: BindingTarget,
        config: BindingConfig,
    ) -> BindingId;
    
    /// Process data change
    pub fn on_data_changed(&mut self, change: &RepositoryChange, cx: &mut Context<Self>);
    
    /// Process UI change (for two-way bindings)
    pub fn on_ui_changed(&mut self, target: &BindingTarget, value: Value, cx: &mut Context<Self>);
}

pub struct Binding {
    pub id: BindingId,
    pub source: DataPath,
    pub target: BindingTarget,
    pub mode: BindingMode,
    pub transform: Option<Box<dyn Transform>>,
    pub inverse_transform: Option<Box<dyn Transform>>,  // For two-way
    pub converter: Option<TypeConverter>,
}

pub struct BindingTarget {
    pub component_id: String,
    pub property: String,
}

pub enum BindingMode {
    OneWay,       // Source → Target only
    TwoWay,       // Source ↔ Target
    OneTime,      // Set once on initialization
}

pub struct BindingConfig {
    pub mode: BindingMode,
    pub transform: Option<String>,        // RHAI expression
    pub inverse_transform: Option<String>, // For two-way
    pub debounce: Option<Duration>,
    pub throttle: Option<Duration>,
    pub default: Option<Value>,
}
```

---

## Data Path Syntax

Data paths provide a consistent way to reference data throughout the system:

```
data.<source_id>.<path>
state.<component_id>.<property>
var.<variable_name>
env.<environment_variable>
```

**Examples:**
```
data.api_data.records[0].name
data.api_data.loading
state.table.selected_rows
var.api_base_url
env.API_TOKEN
```

**Path Operations:**
```rust
pub struct DataPath {
    segments: Vec<PathSegment>,
}

pub enum PathSegment {
    Property(String),
    Index(usize),
    Wildcard,              // *
    RecursiveWildcard,     // **
}

impl DataPath {
    pub fn parse(s: &str) -> Result<Self, PathError>;
    pub fn get(&self, root: &Value) -> Option<&Value>;
    pub fn set(&self, root: &mut Value, value: Value) -> Result<(), PathError>;
    pub fn matches(&self, other: &DataPath) -> bool;  // For subscriptions
}
```

---

## Reactive Data Flow

### Change Propagation

```
┌──────────┐    Update    ┌────────────┐    Change     ┌─────────────┐
│  Source  │─────────────▶│ Repository │──────────────▶│  Listeners  │
└──────────┘              └────────────┘               └─────────────┘
                                │                             │
                                ▼                             ▼
                    ┌──────────────────────┐        ┌──────────────┐
                    │    Action System     │        │   Bindings   │
                    │  (condition check)   │        │   (update)   │
                    └──────────────────────┘        └──────────────┘
                                │                             │
                                ▼                             ▼
                    ┌──────────────────────┐        ┌──────────────┐
                    │   Execute Actions    │        │  Update UI   │
                    └──────────────────────┘        └──────────────┘
```

### Dependency Tracking

For computed bindings that depend on multiple sources:

```rust
pub struct ComputedBinding {
    pub id: BindingId,
    pub sources: Vec<DataPath>,
    pub compute: Box<dyn Fn(&[Value]) -> Value>,
    pub target: BindingTarget,
    pub cached_value: Option<Value>,
}

impl ComputedBinding {
    pub fn should_recompute(&self, changed_path: &DataPath) -> bool {
        self.sources.iter().any(|s| s.matches(changed_path))
    }
}
```

**Configuration:**
```hcl
bind "total_display" {
  sources = [
    data.orders.items,
    data.settings.tax_rate
  ]
  
  compute = """
    let subtotal = sources[0].map(|i| i.price).sum();
    let tax = subtotal * sources[1];
    subtotal + tax
  """
  
  target = {
    component = "total-label"
    property  = "text"
  }
}
```

---

## Error Handling

### Error Types

```rust
pub enum DataFlowError {
    /// Source failed to connect or fetch
    SourceError { source_id: String, error: DataSourceError },
    
    /// Transform failed
    TransformError { source_id: String, stage: usize, error: TransformError },
    
    /// Repository operation failed
    RepositoryError { path: DataPath, error: RepositoryError },
    
    /// Binding update failed
    BindingError { binding_id: BindingId, error: BindingError },
    
    /// Action execution failed
    ActionError { action: String, error: ActionError },
}
```

### Error Recovery Strategies

| Error Type | Recovery Strategy |
|------------|-------------------|
| Source connection failure | Retry with backoff, show status indicator |
| Transform failure | Skip update, log error, use last good value |
| Repository full | Evict old entries, warn user |
| Binding type mismatch | Convert if possible, log warning |
| Action failure | Log error, continue other actions |

---

## Observability

### Data Flow Tracing

```rust
pub struct DataFlowTracer {
    enabled: bool,
    events: Vec<TraceEvent>,
}

pub enum TraceEvent {
    SourceUpdate { source_id: String, data: Value, timestamp: DateTime<Utc> },
    TransformApplied { source_id: String, stage: usize, input: Value, output: Value },
    RepositoryWrite { path: DataPath, value: Value },
    BindingUpdate { binding_id: BindingId, value: Value },
    ActionExecuted { action: String, params: Value, result: Value },
}

impl DataFlowTracer {
    pub fn trace(&mut self, event: TraceEvent);
    pub fn dump(&self) -> Vec<TraceEvent>;
    pub fn clear(&mut self);
}
```

### Metrics

- Source fetch latency and error rates
- Transform execution time
- Repository size and query performance
- Binding update frequency
- Action execution counts and failures

---

## Performance Considerations

1. **Batching:** Coalesce rapid updates before propagating
2. **Debouncing:** Prevent excessive updates from fast sources
3. **Lazy Evaluation:** Only compute bindings when targets are visible
4. **Caching:** Cache transform results when inputs unchanged
5. **Incremental Updates:** Use patches instead of full replacements
6. **Background Processing:** Heavy transforms run off main thread

---

## Agent Prompt Considerations

When creating an agent to implement the Data Flow Engine:

- **Async expertise required:** Heavy use of async/await and channels
- **Reactive patterns:** Understand pub/sub and observer patterns
- **Type safety:** Preserve type information through transformations
- **Error resilience:** System must not crash on data errors
- **Testing:** Mock sources, verify transform chains, test bindings
- **Memory management:** Watch for unbounded growth in repositories

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
