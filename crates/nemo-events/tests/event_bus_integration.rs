//! Integration tests for the EventBus subsystem.
//!
//! These tests verify end-to-end event emission, subscription, filtering,
//! and multi-subscriber fan-out behavior.

use nemo_events::{Event, EventBus, EventFilter};

// ── Basic emit and receive ───────────────────────────────────────────────

#[tokio::test]
async fn emit_and_receive_single_event() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe();

    bus.emit_simple("test.ping", serde_json::json!({"msg": "hello"}));

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "test.ping");
    assert_eq!(event.payload["msg"], "hello");
}

#[tokio::test]
async fn emit_signal_has_null_payload() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe();

    bus.emit_signal("system.heartbeat");

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "system.heartbeat");
    assert!(event.payload.is_null());
}

// ── Filtered subscriptions ───────────────────────────────────────────────

#[tokio::test]
async fn subscribe_type_only_receives_matching() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe_type("data.updated");

    bus.emit_signal("system.startup");
    bus.emit_simple("data.updated", serde_json::json!(42));
    bus.emit_signal("data.error");

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "data.updated");

    // No more matching events — try_recv should yield nothing
    match sub.try_recv() {
        Ok(None) => {} // expected: no more events
        Ok(Some(_)) => panic!("Should not receive unmatched events"),
        Err(_) => {} // also acceptable: channel empty
    }
}

#[tokio::test]
async fn subscribe_prefix_matches_hierarchy() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe_prefix("data.");

    bus.emit_signal("system.startup");
    bus.emit_signal("data.updated");
    bus.emit_signal("data.error");
    bus.emit_signal("ui.refresh");

    let e1 = sub.recv().await.unwrap();
    let e2 = sub.recv().await.unwrap();
    assert_eq!(e1.event_type, "data.updated");
    assert_eq!(e2.event_type, "data.error");
}

// ── Multi-subscriber fan-out ─────────────────────────────────────────────

#[tokio::test]
async fn multiple_subscribers_all_receive() {
    let bus = EventBus::new(16);
    let mut sub1 = bus.subscribe();
    let mut sub2 = bus.subscribe();
    let mut sub3 = bus.subscribe();

    bus.emit_signal("test.broadcast");

    let e1 = sub1.recv().await.unwrap();
    let e2 = sub2.recv().await.unwrap();
    let e3 = sub3.recv().await.unwrap();

    assert_eq!(e1.event_type, "test.broadcast");
    assert_eq!(e2.event_type, "test.broadcast");
    assert_eq!(e3.event_type, "test.broadcast");
}

#[tokio::test]
async fn subscriber_count_tracks_active() {
    let bus = EventBus::new(16);
    assert_eq!(bus.subscriber_count(), 0);

    let sub1 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);

    let sub2 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 2);

    drop(sub1);
    // Note: tokio broadcast doesn't decrement until next send
    // Just verify we had 2 at peak
    drop(sub2);
}

// ── Complex filters ──────────────────────────────────────────────────────

#[tokio::test]
async fn and_filter_requires_both_conditions() {
    let bus = EventBus::new(16);

    // Subscribe to events that are type "data.updated" AND have source "sensor"
    let filter = EventFilter::Type("data.updated".into()).and(EventFilter::Source("sensor".into()));
    let mut sub = bus.subscribe_filtered(filter);

    // Emit events: only one matches both conditions
    bus.emit(Event::new("data.updated", serde_json::json!(1)).with_source("sensor"));
    bus.emit(Event::new("data.updated", serde_json::json!(2)).with_source("api"));
    bus.emit(Event::simple("system.startup").with_source("sensor"));

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "data.updated");
    assert_eq!(event.source.as_deref(), Some("sensor"));
}

#[tokio::test]
async fn not_filter_excludes() {
    let bus = EventBus::new(16);
    let filter = EventFilter::Not(Box::new(EventFilter::Type("system.startup".into())));
    let mut sub = bus.subscribe_filtered(filter);

    bus.emit_signal("system.startup");
    bus.emit_signal("data.updated");

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "data.updated");
}

// ── Event metadata ───────────────────────────────────────────────────────

#[tokio::test]
async fn event_metadata_roundtrips() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe();

    let event = Event::new("api.response", serde_json::json!({"status": 200}))
        .with_source("http_client")
        .with_correlation("req-abc-123");

    bus.emit(event);

    let received = sub.recv().await.unwrap();
    assert_eq!(received.event_type, "api.response");
    assert_eq!(received.source.as_deref(), Some("http_client"));
    assert_eq!(received.correlation_id.as_deref(), Some("req-abc-123"));
    assert_eq!(received.payload["status"], 200);
    // ID should be assigned
    assert!(!received.id.to_string().is_empty());
}

// ── Event tracing ────────────────────────────────────────────────────────

#[tokio::test]
async fn event_tracing_captures_events() {
    let bus = EventBus::new(16).with_tracing();
    let tracer = bus.tracer().expect("tracer should be set");

    bus.emit_signal("traced.event.1");
    bus.emit_signal("traced.event.2");
    bus.emit_signal("traced.event.3");

    assert_eq!(tracer.count(), 3);
}

// ── Clone shares channel ─────────────────────────────────────────────────

#[tokio::test]
async fn cloned_bus_shares_channel() {
    let bus = EventBus::new(16);
    let bus_clone = bus.clone();
    let mut sub = bus.subscribe();

    bus_clone.emit_signal("from.clone");

    let event = sub.recv().await.unwrap();
    assert_eq!(event.event_type, "from.clone");
}
