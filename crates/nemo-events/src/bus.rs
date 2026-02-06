//! The core EventBus implementation.

use crate::{Event, EventFilter, EventSubscription, EventTracer};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

/// The central event bus for publishing and subscribing to events.
pub struct EventBus {
    tx: broadcast::Sender<Event>,
    tracer: Option<Arc<EventTracer>>,
}

impl EventBus {
    /// Create a new event bus with the specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx, tracer: None }
    }

    /// Create a new event bus with default capacity (1024).
    pub fn with_default_capacity() -> Self {
        Self::new(1024)
    }

    /// Enable event tracing for debugging.
    pub fn with_tracing(mut self) -> Self {
        self.tracer = Some(Arc::new(EventTracer::new()));
        self
    }

    /// Enable event tracing with a custom tracer.
    pub fn with_tracer(mut self, tracer: Arc<EventTracer>) -> Self {
        self.tracer = Some(tracer);
        self
    }

    /// Get the tracer if tracing is enabled.
    pub fn tracer(&self) -> Option<&Arc<EventTracer>> {
        self.tracer.as_ref()
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: Event) {
        if let Some(tracer) = &self.tracer {
            tracer.trace(&event);
        }
        let _ = self.tx.send(event);
    }

    /// Emit a simple event with the given type and payload.
    pub fn emit_simple(&self, event_type: &str, payload: Value) {
        self.emit(Event::new(event_type, payload));
    }

    /// Emit a simple event with no payload.
    pub fn emit_signal(&self, event_type: &str) {
        self.emit(Event::simple(event_type));
    }

    /// Subscribe to all events.
    pub fn subscribe(&self) -> EventSubscription {
        EventSubscription::new(self.tx.subscribe(), EventFilter::Any)
    }

    /// Subscribe with a custom filter.
    pub fn subscribe_filtered(&self, filter: EventFilter) -> EventSubscription {
        EventSubscription::new(self.tx.subscribe(), filter)
    }

    /// Subscribe to a specific event type.
    pub fn subscribe_type(&self, event_type: &str) -> EventSubscription {
        self.subscribe_filtered(EventFilter::Type(event_type.to_string()))
    }

    /// Subscribe to events matching a prefix.
    pub fn subscribe_prefix(&self, prefix: &str) -> EventSubscription {
        self.subscribe_filtered(EventFilter::Prefix(prefix.to_string()))
    }

    /// Subscribe to events matching a glob pattern.
    pub fn subscribe_pattern(&self, pattern: &str) -> EventSubscription {
        self.subscribe_filtered(EventFilter::Pattern(pattern.to_string()))
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            tracer: self.tracer.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_emit_and_receive() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        bus.emit_simple("test.event", json!({"value": 42}));

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.payload["value"], 42);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        bus.emit_signal("test.event");

        let e1 = sub1.recv().await.unwrap();
        let e2 = sub2.recv().await.unwrap();

        assert_eq!(e1.id, e2.id);
    }

    #[tokio::test]
    async fn test_filtered_subscription() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe_type("wanted");

        bus.emit_signal("unwanted");
        bus.emit_signal("wanted");

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "wanted");
    }

    #[tokio::test]
    async fn test_tracing() {
        let bus = EventBus::new(16).with_tracing();

        bus.emit_signal("event.1");
        bus.emit_signal("event.2");

        let tracer = bus.tracer().unwrap();
        assert_eq!(tracer.count(), 2);
    }

    #[tokio::test]
    async fn test_clone_shares_channel() {
        let bus1 = EventBus::new(16);
        let bus2 = bus1.clone();

        let mut sub = bus1.subscribe();
        bus2.emit_signal("from-clone");

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "from-clone");
    }
}
