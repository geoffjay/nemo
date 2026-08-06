//! Layout manager for coordinating the complete layout lifecycle.

use crate::binding::{BindingManager, BindingUpdate, ComponentProperty};
use crate::builder::{BuildResult, LayoutBuilder};
use crate::error::LayoutError;
use crate::node::{LayoutConfig, LayoutNode};
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
    /// Monotonic counter for runtime-generated component IDs (`__dyn_N`).
    dynamic_id_counter: u64,
}

/// A built component instance.
#[derive(Debug, Clone)]
pub struct BuiltComponent {
    /// Component ID.
    pub id: String,
    /// Component type.
    pub component_type: String,
    /// Current property values.
    pub properties: HashMap<String, Value>,
    /// Event handlers (event name -> handler string).
    pub handlers: HashMap<String, String>,
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
            dynamic_id_counter: 0,
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
            dynamic_id_counter: 0,
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
            handlers: result.handlers.clone(),
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
        let component =
            self.components
                .get_mut(component_id)
                .ok_or_else(|| LayoutError::InvalidConfig {
                    component_id: component_id.to_string(),
                    reason: "Component not found".to_string(),
                })?;

        component.properties.insert(property.to_string(), value);
        Ok(())
    }

    /// Generates a unique dynamic component ID (`__dyn_N`).
    ///
    /// The counter is monotonic and never resets, so IDs remain document-wide
    /// unique across the lifetime of the manager (matching the `__anon_N`
    /// invariant from the parser).
    pub fn generate_dynamic_id(&mut self) -> String {
        let id = format!("__dyn_{}", self.dynamic_id_counter);
        self.dynamic_id_counter += 1;
        id
    }

    /// Inserts a new built-in component instance at runtime.
    ///
    /// Validates the component type against the registry (same gate as
    /// `build_node`). When `parent` is `Some`, the new component is appended
    /// as a child of that parent; when `None`, it becomes a new root (the
    /// existing root, if any, is left in place — Nemo renders all parentless
    /// components). Required-property validation is skipped: props arrive
    /// programmatically and partial initialization is a legitimate use case.
    pub fn insert_component(
        &mut self,
        id: &str,
        component_type: &str,
        parent: Option<&str>,
        properties: HashMap<String, Value>,
        handlers: HashMap<String, String>,
    ) -> Result<(), LayoutError> {
        if !self.builder.has_component_type(component_type) {
            return Err(LayoutError::UnknownComponent {
                type_name: component_type.to_string(),
            });
        }

        // If a parent is given, it must exist.
        if let Some(parent_id) = parent {
            if !self.components.contains_key(parent_id) {
                return Err(LayoutError::InvalidConfig {
                    component_id: parent_id.to_string(),
                    reason: "Parent component not found".to_string(),
                });
            }
        }

        // Reject duplicate IDs to preserve the document-wide uniqueness invariant.
        if self.components.contains_key(id) {
            return Err(LayoutError::InvalidConfig {
                component_id: id.to_string(),
                reason: "Component ID already exists".to_string(),
            });
        }

        let component = BuiltComponent {
            id: id.to_string(),
            component_type: component_type.to_string(),
            properties,
            handlers,
            children: Vec::new(),
            parent: parent.map(|p| p.to_string()),
        };

        if let Some(parent_id) = parent {
            if let Some(parent_component) = self.components.get_mut(parent_id) {
                parent_component.children.push(id.to_string());
            }
        }

        self.components.insert(id.to_string(), component);
        Ok(())
    }

    /// Removes a component and all its descendants recursively.
    ///
    /// Refuses to remove the current root (the component returned by
    /// `root_id`) to avoid blanking the app. Removes bindings targeting the
    /// component or any descendant. State entries owned by `App` are left
    /// in place (lazy leak — see plan `runtime-component-creation.md`).
    pub fn remove_component(&mut self, id: &str) -> Result<(), LayoutError> {
        // Guard: refuse to remove the root.
        if self.root_id().as_deref() == Some(id) {
            return Err(LayoutError::InvalidConfig {
                component_id: id.to_string(),
                reason: "Cannot remove the root component".to_string(),
            });
        }

        let parent_id = self
            .components
            .get(id)
            .ok_or_else(|| LayoutError::InvalidConfig {
                component_id: id.to_string(),
                reason: "Component not found".to_string(),
            })?
            .parent
            .clone();

        // Collect the subtree (this component + all descendants) breadth-first.
        let subtree: Vec<String> = {
            let mut ids = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(id.to_string());
            while let Some(cid) = queue.pop_front() {
                ids.push(cid.clone());
                if let Some(c) = self.components.get(&cid) {
                    for child in &c.children {
                        queue.push_back(child.clone());
                    }
                }
            }
            ids
        };

        // Detach from parent's children list.
        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.components.get_mut(&parent_id) {
                parent.children.retain(|c| c != id);
            }
        }

        // Remove bindings for every component in the subtree.
        for subtree_id in &subtree {
            self.bindings.unbind_component(subtree_id);
        }

        // Remove all components in the subtree.
        for subtree_id in &subtree {
            self.components.remove(subtree_id);
        }

        Ok(())
    }

    /// Bulk-sets multiple properties on a component (runtime `update_component`).
    pub fn set_properties(
        &mut self,
        component_id: &str,
        properties: HashMap<String, Value>,
    ) -> Result<(), LayoutError> {
        let component =
            self.components
                .get_mut(component_id)
                .ok_or_else(|| LayoutError::InvalidConfig {
                    component_id: component_id.to_string(),
                    reason: "Component not found".to_string(),
                })?;

        for (key, value) in properties {
            component.properties.insert(key, value);
        }
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

        let root = LayoutNode::new("stack").with_id("root").with_child(
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

        let root = LayoutNode::new("stack").with_id("parent").with_child(
            LayoutNode::new("button")
                .with_id("child")
                .with_prop("label", Value::String("Test".into())),
        );

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

        button
            .config
            .bindings
            .push(BindingSpec::one_way("data.text", "label"));

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

    // ── Runtime insert/remove ──────────────────────────────────────────

    #[test]
    fn test_insert_component_with_parent() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root");
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        let mut props = HashMap::new();
        props.insert("label".to_string(), Value::String("Click".into()));
        manager
            .insert_component("btn", "button", Some("root"), props, HashMap::new())
            .unwrap();

        assert_eq!(manager.component_count(), 2);
        let btn = manager.get_component("btn").unwrap();
        assert_eq!(btn.component_type, "button");
        assert_eq!(btn.parent, Some("root".to_string()));
        let root = manager.get_component("root").unwrap();
        assert!(root.children.contains(&"btn".to_string()));
    }

    #[test]
    fn test_insert_component_no_parent() {
        let mut manager = setup_manager();
        let mut props = HashMap::new();
        props.insert("text".to_string(), Value::String("Standalone".into()));
        manager
            .insert_component("lonely", "label", None, props, HashMap::new())
            .unwrap();
        assert_eq!(manager.component_count(), 1);
        assert!(manager.get_component("lonely").is_some());
    }

    #[test]
    fn test_insert_component_unknown_type_rejected() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root");
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        let result = manager.insert_component(
            "x",
            "no_such_type",
            Some("root"),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(result, Err(LayoutError::UnknownComponent { .. })));
    }

    #[test]
    fn test_insert_component_duplicate_id_rejected() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root");
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        let result = manager.insert_component(
            "root",
            "label",
            Some("root"),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(result, Err(LayoutError::InvalidConfig { .. })));
    }

    #[test]
    fn test_insert_component_nonexistent_parent_rejected() {
        let mut manager = setup_manager();
        let result = manager.insert_component(
            "x",
            "label",
            Some("no_parent"),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(result, Err(LayoutError::InvalidConfig { .. })));
    }

    #[test]
    fn test_remove_component_recursive() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root");
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        // Insert a container with a child.
        manager
            .insert_component(
                "panel",
                "stack",
                Some("root"),
                HashMap::new(),
                HashMap::new(),
            )
            .unwrap();
        let mut child_props = HashMap::new();
        child_props.insert("text".to_string(), Value::String("Inner".into()));
        manager
            .insert_component("inner", "label", Some("panel"), child_props, HashMap::new())
            .unwrap();
        assert_eq!(manager.component_count(), 3);

        // Remove the container — the child must go too.
        manager.remove_component("panel").unwrap();
        assert_eq!(manager.component_count(), 1);
        assert!(manager.get_component("panel").is_none());
        assert!(manager.get_component("inner").is_none());
        // Root's children list should no longer contain panel.
        let root = manager.get_component("root").unwrap();
        assert!(!root.children.contains(&"panel".to_string()));
    }

    #[test]
    fn test_remove_component_root_refused() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root");
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        let result = manager.remove_component("root");
        assert!(matches!(result, Err(LayoutError::InvalidConfig { .. })));
        assert_eq!(manager.component_count(), 1);
    }

    #[test]
    fn test_remove_component_nonexistent() {
        let mut manager = setup_manager();
        let result = manager.remove_component("no_such");
        assert!(matches!(result, Err(LayoutError::InvalidConfig { .. })));
    }

    #[test]
    fn test_remove_component_cleans_bindings() {
        let mut manager = setup_manager();

        let mut button = LayoutNode::new("button")
            .with_id("btn")
            .with_prop("label", Value::String("Initial".into()));
        button
            .config
            .bindings
            .push(BindingSpec::one_way("data.text", "label"));

        let root = LayoutNode::new("stack").with_id("root").with_child(button);
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();
        assert_eq!(manager.bindings().binding_count(), 1);

        manager.remove_component("btn").unwrap();
        assert_eq!(manager.bindings().binding_count(), 0);
    }

    #[test]
    fn test_set_properties_bulk() {
        let mut manager = setup_manager();
        let root = LayoutNode::new("stack").with_id("root").with_child(
            LayoutNode::new("label")
                .with_id("lbl")
                .with_prop("text", Value::String("Hi".into())),
        );
        manager
            .apply_layout(LayoutConfig::new(LayoutType::Stack, root))
            .unwrap();

        let mut props = HashMap::new();
        props.insert("text".to_string(), Value::String("Updated".into()));
        props.insert("visible".to_string(), Value::Bool(false));
        manager.set_properties("lbl", props).unwrap();

        assert_eq!(
            manager.get_property("lbl", "text"),
            Some(&Value::String("Updated".into()))
        );
        assert_eq!(
            manager.get_property("lbl", "visible"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn test_generate_dynamic_id_unique() {
        let mut manager = setup_manager();
        let id1 = manager.generate_dynamic_id();
        let id2 = manager.generate_dynamic_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("__dyn_"));
        assert!(id2.starts_with("__dyn_"));
    }
}
