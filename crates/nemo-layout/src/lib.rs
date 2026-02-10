//! Nemo Layout Engine - UI composition from configuration.
//!
//! This crate provides the layout system for Nemo applications, including:
//! - Layout nodes representing the component tree structure
//! - A builder for constructing layouts from configuration
//! - Binding management for connecting data to components
//! - State coordination for persisting component state
//! - A layout manager that orchestrates the complete lifecycle

pub mod binding;
pub mod builder;
pub mod error;
pub mod manager;
pub mod node;
pub mod state;

pub use binding::{ActiveBinding, BindingId, BindingManager, BindingUpdate, ComponentProperty};
pub use builder::{BuildResult, LayoutBuilder, LayoutTypeBuilder};
pub use error::{BindingError, LayoutError, StateError};
pub use manager::{BuiltComponent, LayoutManager};
pub use node::{
    Alignment, BindingMode, BindingSpec, ComponentConfig, LayoutConfig, LayoutHints, LayoutNode,
    LayoutType, Size,
};
pub use state::{
    ComponentState, MemoryPersistence, StateCoordinator, StateMetadata, StatePersistence,
};

#[cfg(test)]
mod tests {
    use super::*;
    use nemo_config::Value;
    use nemo_registry::{register_all_builtins, ComponentRegistry};
    use std::sync::Arc;

    #[test]
    fn test_complete_workflow() {
        // Create registry with built-ins
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);

        // Create layout manager
        let mut manager = LayoutManager::new(registry);

        // Build a simple layout
        let layout = LayoutConfig::new(
            LayoutType::Stack,
            LayoutNode::new("stack")
                .with_id("main")
                .with_child(
                    LayoutNode::new("label")
                        .with_id("title")
                        .with_prop("text", Value::String("Hello".into())),
                )
                .with_child(
                    LayoutNode::new("button")
                        .with_id("action-btn")
                        .with_prop("label", Value::String("Click Me".into())),
                ),
        );

        // Apply the layout
        manager.apply_layout(layout).unwrap();

        // Verify structure
        assert_eq!(manager.component_count(), 3);
        assert!(manager.get_component("main").is_some());
        assert!(manager.get_component("title").is_some());
        assert!(manager.get_component("action-btn").is_some());

        // Verify parent-child relationships
        let main = manager.get_component("main").unwrap();
        assert!(main.children.contains(&"title".to_string()));
        assert!(main.children.contains(&"action-btn".to_string()));

        // Verify properties
        assert_eq!(
            manager.get_property("title", "text"),
            Some(&Value::String("Hello".into()))
        );
    }

    #[test]
    fn test_layout_with_variables() {
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let mut manager = LayoutManager::new(registry);

        let layout = LayoutConfig::new(
            LayoutType::Stack,
            LayoutNode::new("label")
                .with_id("label1")
                .with_prop("text", Value::String("${var.greeting}".into())),
        )
        .with_variable("greeting", Value::String("Hello World!".into()));

        manager.apply_layout(layout).unwrap();

        assert_eq!(
            manager.get_property("label1", "text"),
            Some(&Value::String("Hello World!".into()))
        );
    }
}
