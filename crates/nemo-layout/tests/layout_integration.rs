//! Integration tests for the layout subsystem.
//!
//! These tests verify layout construction, component management, property
//! access, data binding propagation, and layout lifecycle operations.

use nemo_config::Value;
use nemo_layout::{BindingSpec, LayoutConfig, LayoutManager, LayoutNode, LayoutType};
use nemo_registry::{register_all_builtins, ComponentRegistry};
use std::sync::Arc;

// ── Helper ───────────────────────────────────────────────────────────────

fn make_manager() -> LayoutManager {
    let registry = Arc::new(ComponentRegistry::new());
    register_all_builtins(&registry);
    LayoutManager::new(registry)
}

/// Create a label node with the required "text" property.
fn label_node(id: &str, text: &str) -> LayoutNode {
    LayoutNode::new("label")
        .with_id(id)
        .with_prop("text", Value::String(text.into()))
}

/// Create a button node with the required "label" property.
fn button_node(id: &str, label: &str) -> LayoutNode {
    LayoutNode::new("button")
        .with_id(id)
        .with_prop("label", Value::String(label.into()))
}

// ── Layout construction ──────────────────────────────────────────────────

#[test]
fn apply_simple_layout() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack")
        .with_id("root")
        .with_child(label_node("title", "Hello"))
        .with_child(button_node("btn", "Click"));

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    assert_eq!(lm.component_count(), 3); // root + 2 children
    assert_eq!(lm.root_id(), Some("root".to_string()));
}

#[test]
fn nested_component_tree() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack")
        .with_id("root")
        .with_child(
            LayoutNode::new("panel")
                .with_id("sidebar")
                .with_child(button_node("nav_btn", "Menu")),
        )
        .with_child(
            LayoutNode::new("panel")
                .with_id("content")
                .with_child(label_node("heading", "Dashboard"))
                .with_child(label_node("subtitle", "Overview")),
        );

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    // root(1) + sidebar(1) + nav_btn(1) + content(1) + heading(1) + subtitle(1) = 6
    assert_eq!(lm.component_count(), 6);

    // Verify specific components exist
    assert!(lm.get_component("root").is_some());
    assert!(lm.get_component("sidebar").is_some());
    assert!(lm.get_component("nav_btn").is_some());
    assert!(lm.get_component("content").is_some());
    assert!(lm.get_component("heading").is_some());
    assert!(lm.get_component("subtitle").is_some());
}

// ── Property access ──────────────────────────────────────────────────────

#[test]
fn set_and_get_property() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack")
        .with_id("root")
        .with_child(label_node("lbl", "initial"));

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    assert_eq!(
        lm.get_property("lbl", "text"),
        Some(&Value::String("initial".into()))
    );

    lm.set_property("lbl", "text", Value::String("updated".into()))
        .unwrap();

    assert_eq!(
        lm.get_property("lbl", "text"),
        Some(&Value::String("updated".into()))
    );
}

#[test]
fn get_property_nonexistent_component() {
    let lm = make_manager();
    assert!(lm.get_property("no_such_id", "text").is_none());
}

#[test]
fn set_property_nonexistent_component_errors() {
    let mut lm = make_manager();
    let result = lm.set_property("no_such_id", "text", Value::String("x".into()));
    assert!(result.is_err());
}

// ── Layout types ─────────────────────────────────────────────────────────

#[test]
fn dock_layout_type() {
    let mut lm = make_manager();
    let root = LayoutNode::new("dock").with_id("root");
    lm.apply_layout(LayoutConfig::new(LayoutType::Dock, root))
        .unwrap();
    assert_eq!(lm.component_count(), 1);
}

#[test]
fn stack_layout_type() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack").with_id("root");
    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();
    assert_eq!(lm.component_count(), 1);
}

// ── Data binding propagation ─────────────────────────────────────────────

#[test]
fn data_binding_propagates_value() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack").with_id("root").with_child({
        let mut node = label_node("temp_label", "---");
        node.config
            .bindings
            .push(BindingSpec::one_way("data.sensor.temperature", "text"));
        node
    });

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    // Simulate data change
    let updates = lm.on_data_changed("data.sensor.temperature", &Value::String("23.5C".into()));

    // Should produce a binding update
    assert!(!updates.is_empty());
    assert_eq!(updates[0].target.component_id, "temp_label");
    assert_eq!(updates[0].target.property_path, "text");

    // Apply the updates
    lm.apply_updates(updates);

    // Verify the property was updated
    assert_eq!(
        lm.get_property("temp_label", "text"),
        Some(&Value::String("23.5C".into()))
    );
}

#[test]
fn multiple_bindings_on_same_component() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack").with_id("root").with_child({
        let mut node = label_node("status", "---");
        node.config
            .bindings
            .push(BindingSpec::one_way("data.sensor.temperature", "text"));
        node.config
            .bindings
            .push(BindingSpec::one_way("data.sensor.status", "color"));
        node
    });

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    let updates = lm.on_data_changed("data.sensor.temperature", &Value::Float(25.0));
    lm.apply_updates(updates);

    let updates = lm.on_data_changed("data.sensor.status", &Value::String("ok".into()));
    lm.apply_updates(updates);

    assert_eq!(lm.get_property("status", "text"), Some(&Value::Float(25.0)));
    assert_eq!(
        lm.get_property("status", "color"),
        Some(&Value::String("ok".into()))
    );
}

// ── Clear and re-apply ───────────────────────────────────────────────────

#[test]
fn clear_then_reapply() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack")
        .with_id("root")
        .with_child(label_node("lbl", "hi"));

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root.clone()))
        .unwrap();
    assert_eq!(lm.component_count(), 2);

    lm.clear();
    assert_eq!(lm.component_count(), 0);
    assert!(lm.root_id().is_none());

    // Re-apply
    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();
    assert_eq!(lm.component_count(), 2);
    assert!(lm.get_component("lbl").is_some());
}

// ── Component IDs ────────────────────────────────────────────────────────

#[test]
fn component_ids_lists_all() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack")
        .with_id("root")
        .with_child(label_node("a", "A"))
        .with_child(button_node("b", "B"))
        .with_child(label_node("c", "C"));

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    let mut ids = lm.component_ids();
    ids.sort();
    assert_eq!(ids, vec!["a", "b", "c", "root"]);
}

// ── Handlers ─────────────────────────────────────────────────────────────

#[test]
fn component_handlers_are_preserved() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack").with_id("root").with_child(
        button_node("btn", "Click")
            .with_handler("click", "handle_click")
            .with_handler("hover", "handle_hover"),
    );

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    let btn = lm.get_component("btn").unwrap();
    assert_eq!(
        btn.handlers.get("click").map(|s| s.as_str()),
        Some("handle_click")
    );
    assert_eq!(
        btn.handlers.get("hover").map(|s| s.as_str()),
        Some("handle_hover")
    );
}

// ── Component type and parent tracking ───────────────────────────────────

#[test]
fn component_type_and_parent() {
    let mut lm = make_manager();
    let root = LayoutNode::new("stack").with_id("root").with_child(
        LayoutNode::new("panel")
            .with_id("container")
            .with_child(label_node("inner", "text")),
    );

    lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
        .unwrap();

    let root = lm.get_component("root").unwrap();
    assert_eq!(root.component_type, "stack");
    assert!(root.parent.is_none());

    let container = lm.get_component("container").unwrap();
    assert_eq!(container.component_type, "panel");
    assert_eq!(container.parent.as_deref(), Some("root"));

    let inner = lm.get_component("inner").unwrap();
    assert_eq!(inner.component_type, "label");
    assert_eq!(inner.parent.as_deref(), Some("container"));
}
