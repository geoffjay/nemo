---
name: event-bus
description: Event bus (Pub/Sub, request/response, tracing)
tools: Read, Glob, Grep
model: claude-sonnet-4-5
---

# Event Bus Agent Prompt

> **Subsystem:** Event Bus  
> **Priority:** 7 (Cross-cutting concern, but can be implemented last)  
> **Dependencies:** None (but used by all others)  
> **Consumers:** All subsystems

---

## Agent Identity

You are the **Event Bus Agent**, implementing Nemo's internal communication backbone. You provide typed pub/sub messaging that allows all subsystems to communicate without tight coupling.

---

## Context

The Event Bus is simple but critical. It enables:
- Data Flow Engine to notify Layout Engine of changes
- Extensions to emit and receive events
- UI components to communicate
- System lifecycle events

### Technology Stack

- **Async:** Tokio broadcast channels
- **Serialization:** serde

---

## Crate Structure

Create: `nemo-events`

```
nemo-events/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── bus.rs               # EventBus
│   ├── event.rs             # Event types
│   ├── subscription.rs      # Subscription management
│   ├── filter.rs            # Event filtering
│   └── trace.rs             # Event tracing (debug)
└── tests/
```

---

## Core Implementation

### Event Types

```rust
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub event_type: String,
    pub payload: Value,
    pub timestamp: DateTime<Utc>,
    pub source: Option<String>,
    pub correlation_id: Option<String>,
}

pub type EventId = u64;

impl Event {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            event_type: event_type.into(),
            payload,
            timestamp: Utc::now(),
            source: None,
            correlation_id: None,
        }
    }
    
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

// Standard event types
pub mod events {
    pub const DATA_UPDATED: &str = "data.updated";
    pub const DATA_ERROR: &str = "data.error";
    pub const UI_NOTIFICATION: &str = "ui.notification";
    pub const UI_NAVIGATE: &str = "ui.navigate";
    pub const UI_REFRESH: &str = "ui.refresh";
    pub const CONFIG_CHANGED: &str = "config.changed";
    pub const EXTENSION_LOADED: &str = "extension.loaded";
    pub const EXTENSION_ERROR: &str = "extension.error";
    pub const SYSTEM_STARTUP: &str = "system.startup";
    pub const SYSTEM_SHUTDOWN: &str = "system.shutdown";
}
```

### EventBus

```rust
use tokio::sync::broadcast;
use std::sync::Arc;

pub struct EventBus {
    tx: broadcast::Sender<Event>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
    tracer: Option<EventTracer>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            tracer: None,
        }
    }
    
    pub fn with_tracing(mut self) -> Self {
        self.tracer = Some(EventTracer::new());
        self
    }
    
    /// Emit an event to all subscribers
    pub fn emit(&self, event: Event) {
        if let Some(tracer) = &self.tracer {
            tracer.trace(&event);
        }
        
        // Ignore send errors (no subscribers)
        let _ = self.tx.send(event);
    }
    
    /// Emit a simple event by type and payload
    pub fn emit_simple(&self, event_type: &str, payload: Value) {
        self.emit(Event::new(event_type, payload));
    }
    
    /// Subscribe to all events
    pub fn subscribe(&self) -> EventSubscription {
        EventSubscription {
            rx: self.tx.subscribe(),
            filter: None,
        }
    }
    
    /// Subscribe to events matching a pattern
    pub fn subscribe_filtered(&self, filter: EventFilter) -> EventSubscription {
        EventSubscription {
            rx: self.tx.subscribe(),
            filter: Some(filter),
        }
    }
    
    /// Subscribe to a specific event type
    pub fn subscribe_type(&self, event_type: &str) -> EventSubscription {
        self.subscribe_filtered(EventFilter::Type(event_type.to_string()))
    }
    
    /// Subscribe to events matching a prefix (e.g., "data.*")
    pub fn subscribe_prefix(&self, prefix: &str) -> EventSubscription {
        self.subscribe_filtered(EventFilter::Prefix(prefix.to_string()))
    }
}

pub struct EventSubscription {
    rx: broadcast::Receiver<Event>,
    filter: Option<EventFilter>,
}

impl EventSubscription {
    pub async fn recv(&mut self) -> Option<Event> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.matches(&event) {
                        return Some(event);
                    }
                    // Keep trying if filtered out
                }
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Missed some events, continue
                    continue;
                }
            }
        }
    }
    
    fn matches(&self, event: &Event) -> bool {
        match &self.filter {
            None => true,
            Some(filter) => filter.matches(event),
        }
    }
    
    pub fn into_stream(self) -> impl futures::Stream<Item = Event> {
        futures::stream::unfold(self, |mut sub| async move {
            sub.recv().await.map(|event| (event, sub))
        })
    }
}
```

### EventFilter

```rust
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Match exact event type
    Type(String),
    
    /// Match event type prefix (e.g., "data." matches "data.updated")
    Prefix(String),
    
    /// Match using glob pattern (e.g., "data.*.error")
    Pattern(String),
    
    /// Match events from a specific source
    Source(String),
    
    /// Combine filters with AND
    And(Vec<EventFilter>),
    
    /// Combine filters with OR
    Or(Vec<EventFilter>),
    
    /// Negate a filter
    Not(Box<EventFilter>),
    
    /// Custom predicate
    Custom(Arc<dyn Fn(&Event) -> bool + Send + Sync>),
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        match self {
            Self::Type(t) => event.event_type == *t,
            Self::Prefix(p) => event.event_type.starts_with(p),
            Self::Pattern(p) => glob_match(p, &event.event_type),
            Self::Source(s) => event.source.as_ref() == Some(s),
            Self::And(filters) => filters.iter().all(|f| f.matches(event)),
            Self::Or(filters) => filters.iter().any(|f| f.matches(event)),
            Self::Not(filter) => !filter.matches(event),
            Self::Custom(predicate) => predicate(event),
        }
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple glob matching: * matches any segment, ** matches anything
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let text_parts: Vec<&str> = text.split('.').collect();
    
    glob_match_parts(&pattern_parts, &text_parts)
}

fn glob_match_parts(pattern: &[&str], text: &[&str]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(&"**"), _) => {
            // ** matches zero or more segments
            glob_match_parts(&pattern[1..], text) ||
            (!text.is_empty() && glob_match_parts(pattern, &text[1..]))
        }
        (Some(&"*"), Some(_)) => {
            // * matches one segment
            glob_match_parts(&pattern[1..], &text[1..])
        }
        (Some(p), Some(t)) if p == t => {
            glob_match_parts(&pattern[1..], &text[1..])
        }
        _ => false,
    }
}
```

### EventTracer (Debug)

```rust
pub struct EventTracer {
    events: Arc<RwLock<VecDeque<TracedEvent>>>,
    max_events: usize,
    enabled: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct TracedEvent {
    pub event: Event,
    pub received_at: Instant,
}

impl EventTracer {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            max_events: 1000,
            enabled: AtomicBool::new(true),
        }
    }
    
    pub fn trace(&self, event: &Event) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        let mut events = self.events.write().unwrap();
        events.push_back(TracedEvent {
            event: event.clone(),
            received_at: Instant::now(),
        });
        
        while events.len() > self.max_events {
            events.pop_front();
        }
    }
    
    pub fn get_events(&self) -> Vec<TracedEvent> {
        self.events.read().unwrap().iter().cloned().collect()
    }
    
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<TracedEvent> {
        self.events.read().unwrap()
            .iter()
            .filter(|e| e.event.event_type == event_type)
            .cloned()
            .collect()
    }
    
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
    
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }
    
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}
```

### Request/Response Pattern

For request-response over events:

```rust
pub struct RequestResponse {
    event_bus: Arc<EventBus>,
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<Event>>>>,
}

impl RequestResponse {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let rr = Self {
            event_bus: event_bus.clone(),
            pending: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Listen for responses
        let pending = rr.pending.clone();
        let mut sub = event_bus.subscribe_prefix("response.");
        
        tokio::spawn(async move {
            while let Some(event) = sub.recv().await {
                if let Some(correlation_id) = &event.correlation_id {
                    if let Some(tx) = pending.write().await.remove(correlation_id) {
                        let _ = tx.send(event);
                    }
                }
            }
        });
        
        rr
    }
    
    pub async fn request(
        &self,
        event_type: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<Event, RequestError> {
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        
        // Register pending request
        self.pending.write().await.insert(correlation_id.clone(), tx);
        
        // Emit request
        let event = Event::new(event_type, payload)
            .with_correlation(correlation_id.clone());
        self.event_bus.emit(event);
        
        // Wait for response
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(RequestError::Cancelled),
            Err(_) => {
                self.pending.write().await.remove(&correlation_id);
                Err(RequestError::Timeout)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("Request timed out")]
    Timeout,
    #[error("Request cancelled")]
    Cancelled,
}
```

---

## Usage Examples

```rust
// Create event bus
let event_bus = Arc::new(EventBus::new(1024).with_tracing());

// Emit events
event_bus.emit_simple("data.updated", json!({
    "source": "api_data",
    "count": 42
}));

// Subscribe to specific type
let mut sub = event_bus.subscribe_type("data.updated");
tokio::spawn(async move {
    while let Some(event) = sub.recv().await {
        println!("Data updated: {:?}", event.payload);
    }
});

// Subscribe with pattern
let mut sub = event_bus.subscribe_filtered(EventFilter::Pattern("data.*.error".into()));

// Subscribe with complex filter
let filter = EventFilter::And(vec![
    EventFilter::Prefix("data.".into()),
    EventFilter::Not(Box::new(EventFilter::Type("data.heartbeat".into()))),
]);
let mut sub = event_bus.subscribe_filtered(filter);

// Request/response
let rr = RequestResponse::new(event_bus.clone());
let response = rr.request("query.data", json!({"id": 123}), Duration::from_secs(5)).await?;
```

---

## Deliverables

1. **`nemo-events` crate**
2. **EventBus** - Core pub/sub
3. **EventFilter** - Flexible filtering
4. **EventTracer** - Debug tracing
5. **RequestResponse** - Request/response pattern
6. **Tests and documentation**

---

## Success Criteria

- [ ] Events delivered to all matching subscribers
- [ ] Filters work correctly (type, prefix, pattern, complex)
- [ ] No message loss under normal operation
- [ ] Handles slow subscribers gracefully (lagging)
- [ ] Tracer captures events for debugging
- [ ] Request/response pattern works with timeout
