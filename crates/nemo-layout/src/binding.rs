//! Binding management for connecting data to components.

use crate::error::BindingError;
use crate::node::BindingMode;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingId(u64);

impl BindingId {
    /// Generates a new unique binding ID.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for BindingId {
    fn default() -> Self {
        Self::new()
    }
}

/// Target of a binding (a component property).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentProperty {
    /// Component ID.
    pub component_id: String,
    /// Property path within the component.
    pub property_path: String,
}

impl ComponentProperty {
    /// Creates a new component property.
    pub fn new(component_id: impl Into<String>, property_path: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            property_path: property_path.into(),
        }
    }
}

/// An active binding between data and a component.
#[derive(Debug)]
pub struct ActiveBinding {
    /// Binding ID.
    pub id: BindingId,
    /// Data source path.
    pub source: String,
    /// Target component property.
    pub target: ComponentProperty,
    /// Binding mode.
    pub mode: BindingMode,
    /// Transform expression (optional).
    pub transform: Option<String>,
    /// Last value sent to target.
    pub last_value: Option<Value>,
}

impl ActiveBinding {
    /// Creates a new active binding.
    pub fn new(source: impl Into<String>, target: ComponentProperty, mode: BindingMode) -> Self {
        Self {
            id: BindingId::new(),
            source: source.into(),
            target,
            mode,
            transform: None,
            last_value: None,
        }
    }

    /// Sets the transform expression.
    pub fn with_transform(mut self, transform: impl Into<String>) -> Self {
        self.transform = Some(transform.into());
        self
    }
}

/// Manager for component bindings.
pub struct BindingManager {
    /// Active bindings by ID.
    bindings: HashMap<BindingId, ActiveBinding>,
    /// Index from source path to binding IDs.
    source_index: HashMap<String, Vec<BindingId>>,
    /// Index from component ID to binding IDs.
    component_index: HashMap<String, Vec<BindingId>>,
}

impl BindingManager {
    /// Creates a new binding manager.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            source_index: HashMap::new(),
            component_index: HashMap::new(),
        }
    }

    /// Creates a binding between data and a component.
    pub fn bind(
        &mut self,
        source: impl Into<String>,
        target: ComponentProperty,
        mode: BindingMode,
        transform: Option<String>,
    ) -> BindingId {
        let source = source.into();
        let component_id = target.component_id.clone();

        let mut binding = ActiveBinding::new(source.clone(), target, mode);
        if let Some(t) = transform {
            binding = binding.with_transform(t);
        }

        let id = binding.id;

        // Add to indices
        self.source_index
            .entry(source)
            .or_default()
            .push(id);
        self.component_index
            .entry(component_id)
            .or_default()
            .push(id);

        self.bindings.insert(id, binding);
        id
    }

    /// Removes a binding.
    pub fn unbind(&mut self, id: BindingId) {
        if let Some(binding) = self.bindings.remove(&id) {
            // Remove from source index
            if let Some(ids) = self.source_index.get_mut(&binding.source) {
                ids.retain(|bid| *bid != id);
            }
            // Remove from component index
            if let Some(ids) = self.component_index.get_mut(&binding.target.component_id) {
                ids.retain(|bid| *bid != id);
            }
        }
    }

    /// Gets a binding by ID.
    pub fn get(&self, id: BindingId) -> Option<&ActiveBinding> {
        self.bindings.get(&id)
    }

    /// Gets a mutable binding by ID.
    pub fn get_mut(&mut self, id: BindingId) -> Option<&mut ActiveBinding> {
        self.bindings.get_mut(&id)
    }

    /// Gets all bindings for a source path.
    pub fn bindings_for_source(&self, source: &str) -> Vec<&ActiveBinding> {
        self.source_index
            .get(source)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.bindings.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all bindings for a component.
    pub fn bindings_for_component(&self, component_id: &str) -> Vec<&ActiveBinding> {
        self.component_index
            .get(component_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.bindings.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Removes all bindings for a component.
    pub fn unbind_component(&mut self, component_id: &str) {
        if let Some(ids) = self.component_index.remove(component_id) {
            for id in ids {
                if let Some(binding) = self.bindings.remove(&id) {
                    if let Some(source_ids) = self.source_index.get_mut(&binding.source) {
                        source_ids.retain(|bid| *bid != id);
                    }
                }
            }
        }
    }

    /// Returns the total number of bindings.
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Processes a data change and returns updates to apply.
    pub fn on_data_changed(
        &mut self,
        source_path: &str,
        new_value: &Value,
    ) -> Vec<BindingUpdate> {
        let mut updates = Vec::new();

        if let Some(binding_ids) = self.source_index.get(source_path).cloned() {
            for id in binding_ids {
                if let Some(binding) = self.bindings.get_mut(&id) {
                    // Skip if value hasn't changed
                    if binding.last_value.as_ref() == Some(new_value) {
                        continue;
                    }

                    // Apply transform if present (simplified - just pass through for now)
                    let transformed = new_value.clone();

                    binding.last_value = Some(transformed.clone());

                    updates.push(BindingUpdate {
                        binding_id: id,
                        target: binding.target.clone(),
                        value: transformed,
                    });
                }
            }
        }

        updates
    }
}

impl Default for BindingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A pending update from a binding.
#[derive(Debug, Clone)]
pub struct BindingUpdate {
    /// Binding ID that produced this update.
    pub binding_id: BindingId,
    /// Target to update.
    pub target: ComponentProperty,
    /// New value.
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_id_unique() {
        let id1 = BindingId::new();
        let id2 = BindingId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bind_and_unbind() {
        let mut manager = BindingManager::new();

        let target = ComponentProperty::new("comp1", "value");
        let id = manager.bind("data.test", target, BindingMode::OneWay, None);

        assert_eq!(manager.binding_count(), 1);
        assert!(manager.get(id).is_some());

        manager.unbind(id);
        assert_eq!(manager.binding_count(), 0);
    }

    #[test]
    fn test_bindings_for_source() {
        let mut manager = BindingManager::new();

        let target1 = ComponentProperty::new("comp1", "value");
        let target2 = ComponentProperty::new("comp2", "value");

        manager.bind("data.test", target1, BindingMode::OneWay, None);
        manager.bind("data.test", target2, BindingMode::OneWay, None);
        manager.bind("data.other", ComponentProperty::new("comp3", "value"), BindingMode::OneWay, None);

        let bindings = manager.bindings_for_source("data.test");
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_on_data_changed() {
        let mut manager = BindingManager::new();

        let target = ComponentProperty::new("comp1", "value");
        manager.bind("data.test", target, BindingMode::OneWay, None);

        let updates = manager.on_data_changed("data.test", &Value::Integer(42));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].value, Value::Integer(42));
    }
}
