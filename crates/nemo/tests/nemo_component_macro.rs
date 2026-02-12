//! Integration tests for the `#[derive(NemoComponent)]` proc macro.
//!
//! These tests verify that the macro correctly generates `new()` constructors
//! that extract properties from `BuiltComponent`, and optional `children()` builders.

use nemo_config::Value;
use nemo_layout::BuiltComponent;
use nemo_macros::NemoComponent;
use std::collections::HashMap;

// ── Helper ────────────────────────────────────────────────────────────────

fn make_component(props: Vec<(&str, Value)>) -> BuiltComponent {
    let mut properties = HashMap::new();
    for (k, v) in props {
        properties.insert(k.to_string(), v);
    }
    BuiltComponent {
        id: "test".to_string(),
        component_type: "test".to_string(),
        properties,
        handlers: HashMap::new(),
        children: vec![],
        parent: None,
    }
}

// ── Basic property extraction ─────────────────────────────────────────────

#[derive(NemoComponent)]
struct StringProp {
    #[property]
    label: String,
}

#[test]
fn test_string_property_present() {
    let c = make_component(vec![("label", Value::String("Hello".into()))]);
    let s = StringProp::new(c);
    assert_eq!(s.label, "Hello");
}

#[test]
fn test_string_property_missing_uses_default() {
    let c = make_component(vec![]);
    let s = StringProp::new(c);
    assert_eq!(s.label, ""); // String::default()
}

#[test]
fn test_string_property_from_non_string_value() {
    // When value is not a string, it should use v.to_string() fallback
    let c = make_component(vec![("label", Value::Integer(42))]);
    let s = StringProp::new(c);
    assert_eq!(s.label, "42");
}

// ── Integer property ──────────────────────────────────────────────────────

#[derive(NemoComponent)]
struct IntProp {
    #[property]
    count: i64,
}

#[test]
fn test_i64_property_present() {
    let c = make_component(vec![("count", Value::Integer(99))]);
    assert_eq!(IntProp::new(c).count, 99);
}

#[test]
fn test_i64_property_missing_uses_default() {
    let c = make_component(vec![]);
    assert_eq!(IntProp::new(c).count, 0); // i64::default()
}

// ── Float property ────────────────────────────────────────────────────────

#[derive(NemoComponent)]
struct FloatProp {
    #[property]
    ratio: f64,
}

#[test]
fn test_f64_property_present() {
    let c = make_component(vec![("ratio", Value::Float(3.14))]);
    assert!((FloatProp::new(c).ratio - 3.14).abs() < f64::EPSILON);
}

// ── Bool property ─────────────────────────────────────────────────────────

#[derive(NemoComponent)]
struct BoolProp {
    #[property]
    active: bool,
}

#[test]
fn test_bool_property_present() {
    let c = make_component(vec![("active", Value::Bool(true))]);
    assert!(BoolProp::new(c).active);
}

#[test]
fn test_bool_property_missing_uses_default() {
    let c = make_component(vec![]);
    assert!(!BoolProp::new(c).active); // bool::default() = false
}

// ── Optional properties ───────────────────────────────────────────────────

#[derive(NemoComponent)]
struct OptionalProps {
    #[property]
    label: Option<String>,
    #[property]
    count: Option<i64>,
    #[property]
    ratio: Option<f64>,
    #[property]
    active: Option<bool>,
}

#[test]
fn test_optional_properties_present() {
    let c = make_component(vec![
        ("label", Value::String("Hi".into())),
        ("count", Value::Integer(5)),
        ("ratio", Value::Float(1.5)),
        ("active", Value::Bool(true)),
    ]);
    let o = OptionalProps::new(c);
    assert_eq!(o.label.as_deref(), Some("Hi"));
    assert_eq!(o.count, Some(5));
    assert_eq!(o.ratio, Some(1.5));
    assert_eq!(o.active, Some(true));
}

#[test]
fn test_optional_properties_missing() {
    let c = make_component(vec![]);
    let o = OptionalProps::new(c);
    assert!(o.label.is_none());
    assert!(o.count.is_none());
    assert!(o.ratio.is_none());
    assert!(o.active.is_none());
}

// ── Default values ────────────────────────────────────────────────────────

#[derive(NemoComponent)]
struct WithDefaults {
    #[property(default = "Click Me")]
    label: String,
    #[property(default = 10)]
    size: i64,
    #[property(default = 0.5)]
    opacity: f64,
    #[property(default = true)]
    visible: bool,
}

#[test]
fn test_defaults_used_when_props_missing() {
    let c = make_component(vec![]);
    let w = WithDefaults::new(c);
    assert_eq!(w.label, "Click Me");
    assert_eq!(w.size, 10);
    assert!((w.opacity - 0.5).abs() < f64::EPSILON);
    assert!(w.visible);
}

#[test]
fn test_defaults_overridden_when_props_present() {
    let c = make_component(vec![
        ("label", Value::String("OK".into())),
        ("size", Value::Integer(20)),
        ("opacity", Value::Float(1.0)),
        ("visible", Value::Bool(false)),
    ]);
    let w = WithDefaults::new(c);
    assert_eq!(w.label, "OK");
    assert_eq!(w.size, 20);
    assert!((w.opacity - 1.0).abs() < f64::EPSILON);
    assert!(!w.visible);
}

// ── Renamed property key ──────────────────────────────────────────────────

#[derive(NemoComponent)]
struct RenamedProp {
    #[property(name = "text_content")]
    content: String,
}

#[test]
fn test_renamed_property_key() {
    let c = make_component(vec![("text_content", Value::String("Hello".into()))]);
    assert_eq!(RenamedProp::new(c).content, "Hello");
}

#[test]
fn test_renamed_key_original_name_ignored() {
    // The field name "content" should NOT be used as key
    let c = make_component(vec![("content", Value::String("Wrong".into()))]);
    assert_eq!(RenamedProp::new(c).content, ""); // falls through to default
}

// ── #[source] attribute ───────────────────────────────────────────────────

#[derive(NemoComponent)]
struct WithSource {
    #[property]
    label: String,
    #[source]
    source: BuiltComponent,
}

#[test]
fn test_source_stores_built_component() {
    let mut c = make_component(vec![("label", Value::String("Hi".into()))]);
    c.id = "my_button".to_string();
    c.component_type = "button".to_string();
    c.handlers
        .insert("click".to_string(), "on_click".to_string());

    let w = WithSource::new(c);
    assert_eq!(w.label, "Hi");
    assert_eq!(w.source.id, "my_button");
    assert_eq!(w.source.component_type, "button");
    assert_eq!(
        w.source.handlers.get("click").map(|s| s.as_str()),
        Some("on_click")
    );
}

// ── Fields without attributes get Default ─────────────────────────────────

#[derive(NemoComponent)]
struct WithUnattributed {
    #[property]
    label: String,
    extra: Option<i32>,
}

#[test]
fn test_unattributed_field_gets_default() {
    let c = make_component(vec![("label", Value::String("Hi".into()))]);
    let w = WithUnattributed::new(c);
    assert_eq!(w.label, "Hi");
    assert_eq!(w.extra, None); // Default::default() for Option<i32>
}

// ── Mixed attributes ──────────────────────────────────────────────────────

#[derive(NemoComponent)]
struct FullComponent {
    #[property(default = "Button")]
    label: String,
    #[property]
    disabled: Option<bool>,
    #[property(default = "secondary")]
    variant: String,
    runtime: Option<String>, // no attribute — gets Default
    #[source]
    source: BuiltComponent,
}

#[test]
fn test_full_component_all_props() {
    let mut c = make_component(vec![
        ("label", Value::String("Save".into())),
        ("disabled", Value::Bool(true)),
        ("variant", Value::String("primary".into())),
    ]);
    c.id = "save_btn".to_string();

    let fc = FullComponent::new(c);
    assert_eq!(fc.label, "Save");
    assert_eq!(fc.disabled, Some(true));
    assert_eq!(fc.variant, "primary");
    assert!(fc.runtime.is_none());
    assert_eq!(fc.source.id, "save_btn");
}

#[test]
fn test_full_component_defaults() {
    let c = make_component(vec![]);
    let fc = FullComponent::new(c);
    assert_eq!(fc.label, "Button");
    assert!(fc.disabled.is_none());
    assert_eq!(fc.variant, "secondary");
}
