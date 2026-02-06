//! Layout manager for coordinating the complete layout lifecycle.

use crate::binding::{BindingManager, BindingUpdate, ComponentProperty};
use crate::builder::{BuildResult, LayoutBuilder};
use crate::error::LayoutError;
use crate::node::{BindingMode, LayoutConfig, LayoutNode};
use crate::state::StateCoordinator;
use nemo_config::Value;
use nemo_registry::ComponentRegistry;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages the complete layout lifecycle.
pub struct LayoutManager {
    /// Layout builder.
    builder: LayoutBuilder,
    /// Binding manager.
    bindings: BindingManager,
    /// State coordinator.
    state: StateCoordinator,
    /// Built components by ID.
    components: HashMap<String, BuiltComponent>,
    /// Current layout configuration.
    current_config: Option<LayoutConfig>,
}

/// A built component instance.
#[derive(Debug)]
pub struct BuiltComponent {
    /// Component ID.
    pub id: String,
    /// Component type.
    pub component_type: String,
    /// Current property values.
    pub properties: HashMap<String, Value>,
    /// Child component IDs.
    pub children: Vec<String>,
    /// Parent component ID (if any).
    pub parent: Option<String>,
}

impl LayoutManager {
    /// Creates a new layout manager.
    pub fn new(registry: Arc<ComponentRegistry>) -> Self {
        Self {
            builder: LayoutBuilder::new(registry),
            bindings: BindingManager::new(),
            state: StateCoordinator::new(),
            components: HashMap::new(),
            current_config: None,
        }
    }

    /// Creates a layout manager with a custom state coordinator.
    pub fn with_state(registry: Arc<ComponentRegistry>, state: StateCoordinator) -> Self {
        Self {
            builder: LayoutBuilder::new(registry),
            bindings: BindingManager::new(),
            state,
            components: HashMap::new(),
            current_config: None,
        }
    }

    /// Builds and applies a layout configuration.
    pub fn apply_layout(&mut self, config: LayoutConfig) -> Result<(), LayoutError> {
        // Build the layout
        let build_result = self.builder.build(&config)?;

        // Clear existing layout
        self.clear();

        // Convert build results to components
        self.apply_build_result(&build_result, None)?;

        // Set up bindings from the config
        self.setup_bindings_from_node(&config.root)?;

        self.current_config = Some(config);
        Ok(())
    }

    /// Applies a build result recursively.
    fn apply_build_result(
        &mut self,
        result: &BuildResult,
        parent: Option<String>,
    ) -> Result<(), LayoutError> {
        let child_ids: Vec<String> = result.children.iter().map(|c| c.id.clone()).collect();

        let component = BuiltComponent {
            id: result.id.clone(),
            component_type: result.component_type.clone(),
            properties: result.properties.clone(),
            children: child_ids,
            parent,
        };

        self.components.insert(result.id.clone(), component);

        // Process children
        for child in &result.children {
            self.apply_build_result(child, Some(result.id.clone()))?;
        }

        Ok(())
    }

    /// Sets up bindings from a layout node.
    fn setup_bindings_from_node(&mut self, node: &LayoutNode) -> Result<(), LayoutError> {
        let component_id = node.effective_id();

        for binding_spec in &node.config.bindings {
            let target = ComponentProperty::new(&component_id, &binding_spec.target);
            self.bindings.bind(
                &binding_spec.source,
                target,
                binding_spec.mode,
                binding_spec.transform.clone(),
            );
        }

        // Process children
        for child in &node.children {
            self.setup_bindings_from_node(child)?;
        }

        Ok(())
    }

    /// Clears the current layout.
    pub fn clear(&mut self) {
        self.components.clear();
        self.bindings = BindingManager::new();
        self.current_config = None;
    }

    /// Gets a component by ID.
    pub fn get_component(&self, id: &str) -> Option<&BuiltComponent> {
        self.components.get(id)
    }

    /// Gets a mutable component by ID.
    pub fn get_component_mut(&mut self, id: &str) -> Option<&mut BuiltComponent> {
        self.components.get_mut(id)
    }

    /// Returns all component IDs.
    pub fn component_ids(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }

    /// Returns the number of components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Gets the root component ID.
    pub fn root_id(&self) -> Option<String> {
        self.components
            .values()
            .find(|c| c.parent.is_none())
            .map(|c| c.id.clone())
    }

    /// Processes a data change and returns updates.
    pub fn on_data_changed(&mut self, source_path: &str, value: &Value) -> Vec<BindingUpdate> {
        self.bindings.on_data_changed(source_path, value)
    }

    /// Applies binding updates to components.
    pub fn apply_updates(&mut self, updates: Vec<BindingUpdate>) {
        for update in updates {
            if let Some(component) = self.components.get_mut(&update.target.component_id) {
                component
                    .properties
                    .insert(update.target.property_path, update.value);
            }
        }
    }

    /// Updates a component property.
    pub fn set_property(
        &mut self,
        component_id: &str,
        property: &str,
        value: Value,
    ) -> Result<(), LayoutError> {
        let component = self.components.get_mut(component_id).ok_or_else(|| {
            LayoutError::InvalidConfig {
                component_id: component_id.to_string(),
                reason: "Component not found".to_string(),
            }
        })?;

        component.properties.insert(property.to_string(), value);
        Ok(())
    }

    /// Gets a component property.
    pub fn get_property(&self, component_id: &str, property: &str) -> Option<&Value> {
        self.components
            .get(component_id)
            .and_then(|c| c.properties.get(property))
    }

    /// Returns the state coordinator.
    pub fn state(&self) -> &StateCoordinator {
        &self.state
    }

    /// Returns the mutable state coordinator.
    pub fn state_mut(&mut self) -> &mut StateCoordinator {
        &mut self.state
    }

    /// Returns the binding manager.
    pub fn bindings(&self) -> &BindingManager {
        &self.bindings
    }

    /// Returns the mutable binding manager.
    pub fn bindings_mut(&mut self) -> &mut BindingManager {
        &mut self.bindings
    }

    /// Saves component state.
    pub fn save_state(&mut self) -> Result<(), crate::error::StateError> {
        self.state.persist()
    }

    /// Restores component state.
    pub fn restore_state(&mut self) -> Result<(), crate::error::StateError> {
        self.state.restore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{BindingSpec, LayoutType};
    use nemo_registry::register_all_builtins;

    fn setup_manager() -> LayoutManager {
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        LayoutManager::new(registry)
    }

    #[test]
    fn test_manager_creation() {
        let manager = setup_manager();
        assert_eq!(manager.component_count(), 0);
    }

    #[test]
    fn test_apply_simple_layout() {
        let mut manager = setup_manager();

        let root = LayoutNode::new("stack")
            .with_id("root")
            .with_child(
                LayoutNode::new("button")
                    .with_id("btn1")
                    .with_prop("label", Value::String("Click".into())),
            );

        let config = LayoutConfig::new(LayoutType::Stack, root);
        manager.apply_layout(config).unwrap();

        assert_eq!(manager.component_count(), 2);
        assert!(manager.get_component("root").is_some());
        assert!(manager.get_component("btn1").is_some());
    }

    #[test]
    fn test_root_id() {
        let mut manager = setup_manager();

        let root = LayoutNode::new("stack").with_id("root");
        let config = LayoutConfig::new(LayoutType::Stack, root);
        manager.apply_layout(config).unwrap();

        assert_eq!(manager.root_id(), Some("root".to_string()));
    }

    #[test]
    fn test_parent_child_relationship() {
        let mut manager = setup_manager();

        let root = LayoutNode::new("stack")
            .with_id("parent")
            .with_child(LayoutNode::new("button").with_id("child").with_prop("label", Value::String("Test".into())));

        let config = LayoutConfig::new(LayoutType::Stack, root);
        manager.apply_layout(config).unwrap();

        let child = manager.get_component("child").unwrap();
        assert_eq!(child.parent, Some("parent".to_string()));

        let parent = manager.get_component("parent").unwrap();
        assert!(parent.children.contains(&"child".to_string()));
    }

    #[test]
    fn test_data_binding() {
        let mut manager = setup_manager();

        let mut button = LayoutNode::new("button")
            .with_id("btn")
            .with_prop("label", Value::String("Initial".into()));

        button.config.bindings.push(BindingSpec::one_way("data.text", "label"));

        let root = LayoutNode::new("stack").with_id("root").with_child(button);
        let config = LayoutConfig::new(LayoutType::Stack, root);
        manager.apply_layout(config).unwrap();

        // Simulate data change
        let updates = manager.on_data_changed("data.text", &Value::String("Updated".into()));
        assert_eq!(updates.len(), 1);

        manager.apply_updates(updates);
        assert_eq!(
            manager.get_property("btn", "label"),
            Some(&Value::String("Updated".into()))
        );
    }
}
