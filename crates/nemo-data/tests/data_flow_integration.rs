//! Integration tests for the data flow subsystem.
//!
//! These tests verify end-to-end data flow: source creation, pipeline
//! transformation, repository storage, binding propagation, and
//! repository change subscriptions.

use nemo_config::Value;
use nemo_data::{
    create_source, BindingConfig, BindingTarget, DataFlowEngine, DataPath, DataRepository,
    DataUpdate, Pipeline, SkipTransform, TakeTransform, UpdateType,
};

// ── Data source creation ─────────────────────────────────────────────────

#[test]
fn create_source_all_supported_types() {
    let timer_cfg = make_val(vec![("type", Value::String("timer".into()))]);
    assert!(create_source("t", "timer", &timer_cfg).is_some());

    let http_cfg = make_val(vec![
        ("type", Value::String("http".into())),
        ("url", Value::String("https://example.com".into())),
    ]);
    assert!(create_source("h", "http", &http_cfg).is_some());

    let ws_cfg = make_val(vec![
        ("type", Value::String("websocket".into())),
        ("url", Value::String("ws://localhost:8080".into())),
    ]);
    assert!(create_source("ws", "websocket", &ws_cfg).is_some());

    let mqtt_cfg = make_val(vec![("type", Value::String("mqtt".into()))]);
    assert!(create_source("mq", "mqtt", &mqtt_cfg).is_some());

    let nats_cfg = make_val(vec![
        ("type", Value::String("nats".into())),
        ("url", Value::String("nats://localhost:4222".into())),
    ]);
    assert!(create_source("n", "nats", &nats_cfg).is_some());

    let redis_cfg = make_val(vec![
        ("type", Value::String("redis".into())),
        ("url", Value::String("redis://localhost:6379".into())),
    ]);
    assert!(create_source("r", "redis", &redis_cfg).is_some());

    let file_cfg = make_val(vec![
        ("type", Value::String("file".into())),
        ("path", Value::String("/tmp/data.json".into())),
    ]);
    assert!(create_source("f", "file", &file_cfg).is_some());
}

#[test]
fn create_source_unknown_type_returns_none() {
    let cfg = make_val(vec![("type", Value::String("kafka".into()))]);
    assert!(create_source("x", "kafka", &cfg).is_none());
}

// ── DataFlowEngine source registration ───────────────────────────────────

#[tokio::test]
async fn register_and_list_sources() {
    let engine = DataFlowEngine::new();

    let timer_cfg = make_val(vec![
        ("type", Value::String("timer".into())),
        ("interval", Value::Integer(60)),
    ]);
    let source = create_source("clock", "timer", &timer_cfg).unwrap();
    engine.register_source(source).await;

    assert!(engine.has_source("clock").await);
    assert!(!engine.has_source("nonexistent").await);

    let ids = engine.source_ids().await;
    assert_eq!(ids, vec!["clock".to_string()]);
}

#[tokio::test]
async fn unregister_source() {
    let engine = DataFlowEngine::new();

    let cfg = make_val(vec![("type", Value::String("timer".into()))]);
    let source = create_source("temp", "timer", &cfg).unwrap();
    engine.register_source(source).await;
    assert!(engine.has_source("temp").await);

    let removed = engine.unregister_source("temp").await;
    assert!(removed.is_some());
    assert!(!engine.has_source("temp").await);
}

// ── Process update stores data in repository ─────────────────────────────

#[tokio::test]
async fn process_update_stores_in_repository() {
    let engine = DataFlowEngine::new();

    let update = DataUpdate {
        source_id: "sensor".into(),
        data: make_val(vec![
            ("temperature", Value::Float(23.5)),
            ("humidity", Value::Integer(65)),
        ]),
        update_type: UpdateType::Full,
        timestamp: chrono::Utc::now(),
    };

    engine.process_update(update).await.unwrap();

    // Data should be stored under data.sensor.*
    let path = DataPath::parse("data.sensor.temperature").unwrap();
    let val = engine.repository.get(&path);
    assert_eq!(val, Some(Value::Float(23.5)));

    let path = DataPath::parse("data.sensor.humidity").unwrap();
    let val = engine.repository.get(&path);
    assert_eq!(val, Some(Value::Integer(65)));
}

// ── Pipeline transforms data before storage ──────────────────────────────

#[tokio::test]
async fn pipeline_transforms_data() {
    let engine = DataFlowEngine::new();

    // Set up a take(2) pipeline for the "readings" source
    let mut pipeline = Pipeline::new();
    pipeline.add(Box::new(TakeTransform::new(2)));
    engine.set_pipeline("readings", pipeline).await;

    let update = DataUpdate {
        source_id: "readings".into(),
        data: Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ]),
        update_type: UpdateType::Full,
        timestamp: chrono::Utc::now(),
    };

    engine.process_update(update).await.unwrap();

    let path = DataPath::parse("data.readings").unwrap();
    let stored = engine.repository.get(&path).unwrap();
    let items = stored.as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], Value::Integer(1));
    assert_eq!(items[1], Value::Integer(2));
}

#[tokio::test]
async fn multi_stage_pipeline() {
    let engine = DataFlowEngine::new();

    // skip(1) -> take(2) pipeline
    let mut pipeline = Pipeline::new();
    pipeline.add(Box::new(SkipTransform::new(1)));
    pipeline.add(Box::new(TakeTransform::new(2)));
    engine.set_pipeline("items", pipeline).await;

    let update = DataUpdate {
        source_id: "items".into(),
        data: Value::Array(vec![
            Value::Integer(10),
            Value::Integer(20),
            Value::Integer(30),
            Value::Integer(40),
        ]),
        update_type: UpdateType::Full,
        timestamp: chrono::Utc::now(),
    };

    engine.process_update(update).await.unwrap();

    let path = DataPath::parse("data.items").unwrap();
    let stored = engine.repository.get(&path).unwrap();
    let items = stored.as_array().unwrap();
    assert_eq!(items, &vec![Value::Integer(20), Value::Integer(30)]);
}

// ── Repository subscription ──────────────────────────────────────────────

#[tokio::test]
async fn repository_change_subscription() {
    let repo = DataRepository::new();
    let mut sub = repo.subscribe();

    let path = DataPath::parse("data.test").unwrap();
    repo.set(&path, Value::String("hello".into())).unwrap();

    let change = sub.recv().await.unwrap();
    assert_eq!(change.path.to_string(), "data.test");
}

#[tokio::test]
async fn repository_update_from_source() {
    let repo = DataRepository::new();

    let data = make_val(vec![
        ("temp", Value::Float(25.0)),
        ("unit", Value::String("C".into())),
    ]);
    repo.update_from_source("sensor1", data).unwrap();

    let path = DataPath::parse("data.sensor1.temp").unwrap();
    assert_eq!(repo.get(&path), Some(Value::Float(25.0)));

    let path = DataPath::parse("data.sensor1.unit").unwrap();
    assert_eq!(repo.get(&path), Some(Value::String("C".into())));
}

// ── Multiple sources with separate namespaces ────────────────────────────

#[tokio::test]
async fn multiple_sources_isolated_namespaces() {
    let engine = DataFlowEngine::new();

    // Source 1: temperature
    let u1 = DataUpdate {
        source_id: "temp_sensor".into(),
        data: make_val(vec![("value", Value::Float(22.5))]),
        update_type: UpdateType::Full,
        timestamp: chrono::Utc::now(),
    };

    // Source 2: humidity
    let u2 = DataUpdate {
        source_id: "humidity_sensor".into(),
        data: make_val(vec![("value", Value::Integer(60))]),
        update_type: UpdateType::Full,
        timestamp: chrono::Utc::now(),
    };

    engine.process_update(u1).await.unwrap();
    engine.process_update(u2).await.unwrap();

    let temp_path = DataPath::parse("data.temp_sensor.value").unwrap();
    assert_eq!(engine.repository.get(&temp_path), Some(Value::Float(22.5)));

    let hum_path = DataPath::parse("data.humidity_sensor.value").unwrap();
    assert_eq!(engine.repository.get(&hum_path), Some(Value::Integer(60)));
}

// ── Binding creation ─────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_remove_binding() {
    let engine = DataFlowEngine::new();

    let source = DataPath::parse("data.sensor.temperature").unwrap();
    let target = BindingTarget {
        component_id: "temp_label".into(),
        property: "text".into(),
    };

    let id = engine
        .create_binding(source, target, BindingConfig::default())
        .await;

    // Verify binding exists by removing it (no panic = it existed)
    engine.remove_binding(id).await;
}

// ── Data repository CRUD ─────────────────────────────────────────────────

#[test]
fn repository_set_get_delete() {
    let repo = DataRepository::new();
    let path = DataPath::parse("data.test.key").unwrap();

    // Initially empty
    assert!(repo.get(&path).is_none());

    // Set
    repo.set(&path, Value::Integer(42)).unwrap();
    assert_eq!(repo.get(&path), Some(Value::Integer(42)));

    // Overwrite
    repo.set(&path, Value::String("updated".into())).unwrap();
    assert_eq!(repo.get(&path), Some(Value::String("updated".into())));

    // Delete sets to null (not removal)
    repo.delete(&path).unwrap();
    assert_eq!(repo.get(&path), Some(Value::Null));
}

// ── Helper ───────────────────────────────────────────────────────────────

fn make_val(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = indexmap::IndexMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Object(map)
}
