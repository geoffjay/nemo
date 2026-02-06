//! Nemo Event Bus - Decoupled communication between subsystems.
//!
//! This crate provides a typed pub/sub messaging system that enables
//! communication between all Nemo subsystems without tight coupling.

mod bus;
mod event;
mod filter;
mod subscription;
mod trace;

pub use bus::EventBus;
pub use event::{events, Event, EventId};
pub use filter::EventFilter;
pub use subscription::{EventStream, EventSubscription, SubscriptionClosed};
pub use trace::{EventTracer, TracedEvent};

/// Result type for event bus operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the event bus.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The subscription channel is closed.
    #[error("subscription closed")]
    SubscriptionClosed,

    /// Event serialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_usage() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        let event = Event::new("test.event", json!({"key": "value"}))
            .with_source("test")
            .with_correlation("req-123");

        bus.emit(event);

        let received = sub.recv().await.unwrap();
        assert_eq!(received.event_type, "test.event");
        assert_eq!(received.source, Some("test".to_string()));
        assert_eq!(received.correlation_id, Some("req-123".to_string()));
    }

    #[tokio::test]
    async fn test_complex_filter() {
        let bus = EventBus::new(16);

        let filter = EventFilter::Prefix("data.".to_string()).and(EventFilter::Not(Box::new(
            EventFilter::Type("data.heartbeat".to_string()),
        )));
        let mut sub = bus.subscribe_filtered(filter);

        bus.emit_signal("data.heartbeat");
        bus.emit_signal("data.updated");

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "data.updated");
    }
}
