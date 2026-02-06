//! Event struct and related types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global counter for event IDs.
static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for an event.
pub type EventId = u64;

/// An event that can be published to and received from the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event.
    pub id: EventId,
    /// The type of event (e.g., "data.updated", "ui.notification").
    pub event_type: String,
    /// The event payload.
    pub payload: Value,
    /// Unix timestamp in milliseconds when the event was created.
    pub timestamp: u64,
    /// Optional source identifier (component that emitted the event).
    pub source: Option<String>,
    /// Optional correlation ID for request/response patterns.
    pub correlation_id: Option<String>,
}

impl Event {
    /// Creates a new event with the given type and payload.
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Event {
            id: EVENT_COUNTER.fetch_add(1, Ordering::Relaxed),
            event_type: event_type.into(),
            payload,
            timestamp,
            source: None,
            correlation_id: None,
        }
    }

    /// Creates a new event with no payload.
    pub fn simple(event_type: impl Into<String>) -> Self {
        Self::new(event_type, Value::Null)
    }

    /// Sets the source of the event.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets the correlation ID for request/response patterns.
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

/// Standard event type constants.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_creation() {
        let event = Event::new("test.event", json!({"key": "value"}));
        assert_eq!(event.event_type, "test.event");
        assert!(event.id > 0);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_event_with_source() {
        let event = Event::new("test.event", json!(null)).with_source("test-component");
        assert_eq!(event.source, Some("test-component".to_string()));
    }

    #[test]
    fn test_event_ids_increment() {
        let e1 = Event::simple("test");
        let e2 = Event::simple("test");
        assert!(e2.id > e1.id);
    }
}
