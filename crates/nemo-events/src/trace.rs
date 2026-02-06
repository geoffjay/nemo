//! Event tracing for debugging.

use crate::Event;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// A traced event with timing information.
#[derive(Debug, Clone)]
pub struct TracedEvent {
    /// The event that was traced.
    pub event: Event,
    /// When the event was received by the tracer.
    pub traced_at: Instant,
}

/// Event tracer for debugging and monitoring.
pub struct EventTracer {
    events: RwLock<VecDeque<TracedEvent>>,
    max_events: usize,
    enabled: AtomicBool,
}

impl EventTracer {
    /// Create a new event tracer with default capacity (1000 events).
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create a new event tracer with the specified capacity.
    pub fn with_capacity(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
            enabled: AtomicBool::new(true),
        }
    }

    /// Trace an event.
    pub fn trace(&self, event: &Event) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let traced = TracedEvent {
            event: event.clone(),
            traced_at: Instant::now(),
        };

        let mut events = self.events.write().unwrap();
        events.push_back(traced);

        while events.len() > self.max_events {
            events.pop_front();
        }
    }

    /// Get all traced events.
    pub fn get_events(&self) -> Vec<TracedEvent> {
        self.events.read().unwrap().iter().cloned().collect()
    }

    /// Get traced events matching a specific event type.
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<TracedEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.event.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Get traced events matching a type prefix.
    pub fn get_events_by_prefix(&self, prefix: &str) -> Vec<TracedEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.event.event_type.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Get the count of traced events.
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Clear all traced events.
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }

    /// Enable tracing.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable tracing.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Check if tracing is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for EventTracer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tracer_traces_events() {
        let tracer = EventTracer::new();
        let event = Event::new("test.event", json!({"key": "value"}));

        tracer.trace(&event);

        let events = tracer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.event_type, "test.event");
    }

    #[test]
    fn test_tracer_respects_capacity() {
        let tracer = EventTracer::with_capacity(3);

        for i in 0..5 {
            tracer.trace(&Event::new(format!("event.{}", i), json!(null)));
        }

        let events = tracer.get_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event.event_type, "event.2");
    }

    #[test]
    fn test_tracer_enable_disable() {
        let tracer = EventTracer::new();

        tracer.trace(&Event::new("event.1", json!(null)));
        tracer.disable();
        tracer.trace(&Event::new("event.2", json!(null)));
        tracer.enable();
        tracer.trace(&Event::new("event.3", json!(null)));

        let events = tracer.get_events();
        assert_eq!(events.len(), 2);
    }
}
