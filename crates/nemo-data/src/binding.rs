//! Data binding system for connecting data to UI components.

use crate::error::BindingError;
use crate::repository::{DataPath, RepositoryChange};
use crate::transform::{Transform, TransformContext};
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

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

/// Binding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingMode {
    /// Source to target only.
    OneWay,
    /// Bidirectional binding.
    TwoWay,
    /// Set once on initialization.
    OneTime,
}

impl Default for BindingMode {
    fn default() -> Self {
        Self::OneWay
    }
}

/// Target of a binding (a component property).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingTarget {
    /// Component ID.
    pub component_id: String,
    /// Property path within the component.
    pub property: String,
}

impl BindingTarget {
    /// Creates a new binding target.
    pub fn new(component_id: impl Into<String>, property: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            property: property.into(),
        }
    }
}

/// Configuration for a binding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingConfig {
    /// Binding mode.
    pub mode: BindingMode,
    /// Debounce duration for updates.
    pub debounce: Option<Duration>,
    /// Throttle duration for updates.
    pub throttle: Option<Duration>,
    /// Default value if source is not available.
    pub default: Option<Value>,
}

/// An active binding.
pub struct Binding {
    /// Unique identifier.
    pub id: BindingId,
    /// Data source path.
    pub source: DataPath,
    /// Component target.
    pub target: BindingTarget,
    /// Binding mode.
    pub mode: BindingMode,
    /// Transform to apply (source -> target).
    pub transform: Option<Box<dyn Transform>>,
    /// Inverse transform (target -> source, for two-way).
    pub inverse_transform: Option<Box<dyn Transform>>,
    /// Configuration.
    pub config: BindingConfig,
    /// Last value sent to target.
    pub last_value: Option<Value>,
    /// Whether this binding has been initialized.
    pub initialized: bool,
}

impl Binding {
    /// Creates a new binding.
    pub fn new(source: DataPath, target: BindingTarget, config: BindingConfig) -> Self {
        Self {
            id: BindingId::new(),
            source,
            target,
            mode: config.mode,
            transform: None,
            inverse_transform: None,
            config,
            last_value: None,
            initialized: false,
        }
    }

    /// Sets the transform for this binding.
    pub fn with_transform(mut self, transform: Box<dyn Transform>) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Sets the inverse transform for two-way bindings.
    pub fn with_inverse_transform(mut self, transform: Box<dyn Transform>) -> Self {
        self.inverse_transform = Some(transform);
        self
    }
}

/// A pending update from a binding.
#[derive(Debug, Clone)]
pub struct BindingUpdate {
    /// Binding ID.
    pub binding_id: BindingId,
    /// Target to update.
    pub target: BindingTarget,
    /// New value.
    pub value: Value,
}

/// The binding system manages data-to-UI bindings.
pub struct BindingSystem {
    /// All bindings by ID.
    bindings: HashMap<BindingId, Binding>,
    /// Index from source paths to binding IDs.
    path_index: HashMap<String, Vec<BindingId>>,
    /// Index from target to binding ID.
    target_index: HashMap<BindingTarget, BindingId>,
}

impl BindingSystem {
    /// Creates a new binding system.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            path_index: HashMap::new(),
            target_index: HashMap::new(),
        }
    }

    /// Creates a new binding.
    pub fn create_binding(
        &mut self,
        source: DataPath,
        target: BindingTarget,
        config: BindingConfig,
    ) -> BindingId {
        let binding = Binding::new(source.clone(), target.clone(), config);
        let id = binding.id;

        // Add to path index
        let source_str = source.to_string();
        self.path_index.entry(source_str).or_default().push(id);

        // Add to target index
        self.target_index.insert(target, id);

        // Store binding
        self.bindings.insert(id, binding);

        id
    }

    /// Removes a binding.
    pub fn remove_binding(&mut self, id: BindingId) {
        if let Some(binding) = self.bindings.remove(&id) {
            // Remove from path index
            let source_str = binding.source.to_string();
            if let Some(ids) = self.path_index.get_mut(&source_str) {
                ids.retain(|bid| *bid != id);
            }

            // Remove from target index
            self.target_index.remove(&binding.target);
        }
    }

    /// Gets a binding by ID.
    pub fn get_binding(&self, id: BindingId) -> Option<&Binding> {
        self.bindings.get(&id)
    }

    /// Gets a binding by ID mutably.
    pub fn get_binding_mut(&mut self, id: BindingId) -> Option<&mut Binding> {
        self.bindings.get_mut(&id)
    }

    /// Processes a data change and returns updates to apply.
    pub fn on_data_changed(&mut self, change: &RepositoryChange) -> Vec<BindingUpdate> {
        let mut updates = Vec::new();
        let path_str = change.path.to_string();

        // Find all bindings for this path
        if let Some(binding_ids) = self.path_index.get(&path_str).cloned() {
            for id in binding_ids {
                if let Some(binding) = self.bindings.get_mut(&id) {
                    // Skip one-time bindings that are already initialized
                    if binding.mode == BindingMode::OneTime && binding.initialized {
                        continue;
                    }

                    if let Some(new_value) = &change.new_value {
                        // Apply transform if present
                        let transformed = if let Some(transform) = &binding.transform {
                            let ctx = TransformContext::default();
                            transform
                                .transform(new_value.clone(), &ctx)
                                .unwrap_or_else(|_| new_value.clone())
                        } else {
                            new_value.clone()
                        };

                        // Check if value actually changed
                        if binding.last_value.as_ref() != Some(&transformed) {
                            binding.last_value = Some(transformed.clone());
                            binding.initialized = true;

                            updates.push(BindingUpdate {
                                binding_id: id,
                                target: binding.target.clone(),
                                value: transformed,
                            });
                        }
                    }
                }
            }
        }

        updates
    }

    /// Processes a UI change (for two-way bindings).
    pub fn on_ui_changed(
        &mut self,
        target: &BindingTarget,
        value: Value,
    ) -> Result<Option<(DataPath, Value)>, BindingError> {
        let binding_id = self
            .target_index
            .get(target)
            .ok_or_else(|| BindingError::TargetNotFound(target.component_id.clone()))?;

        let binding = self
            .bindings
            .get_mut(binding_id)
            .ok_or_else(|| BindingError::TargetNotFound(target.component_id.clone()))?;

        if binding.mode != BindingMode::TwoWay {
            return Err(BindingError::InvalidMode);
        }

        // Apply inverse transform if present
        let transformed = if let Some(transform) = &binding.inverse_transform {
            let ctx = TransformContext::default();
            transform.transform(value, &ctx)?
        } else {
            value
        };

        Ok(Some((binding.source.clone(), transformed)))
    }

    /// Lists all binding IDs.
    pub fn list_bindings(&self) -> Vec<BindingId> {
        self.bindings.keys().copied().collect()
    }

    /// Gets the number of bindings.
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for BindingSystem {
    fn default() -> Self {
        Self::new()
    }
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
    fn test_create_binding() {
        let mut system = BindingSystem::new();
        let source = DataPath::parse("data.test").unwrap();
        let target = BindingTarget::new("component1", "value");

        let id = system.create_binding(source, target, BindingConfig::default());

        assert!(system.get_binding(id).is_some());
        assert_eq!(system.binding_count(), 1);
    }

    #[test]
    fn test_remove_binding() {
        let mut system = BindingSystem::new();
        let source = DataPath::parse("data.test").unwrap();
        let target = BindingTarget::new("component1", "value");

        let id = system.create_binding(source, target, BindingConfig::default());
        assert_eq!(system.binding_count(), 1);

        system.remove_binding(id);
        assert_eq!(system.binding_count(), 0);
    }

    #[test]
    fn test_on_data_changed() {
        let mut system = BindingSystem::new();
        let source = DataPath::parse("data.test").unwrap();
        let target = BindingTarget::new("component1", "value");

        system.create_binding(source.clone(), target, BindingConfig::default());

        let change = RepositoryChange {
            path: source,
            old_value: None,
            new_value: Some(Value::Integer(42)),
            timestamp: chrono::Utc::now(),
        };

        let updates = system.on_data_changed(&change);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].value, Value::Integer(42));
    }

    #[test]
    fn test_one_time_binding() {
        let mut system = BindingSystem::new();
        let source = DataPath::parse("data.test").unwrap();
        let target = BindingTarget::new("component1", "value");

        let config = BindingConfig {
            mode: BindingMode::OneTime,
            ..Default::default()
        };

        system.create_binding(source.clone(), target, config);

        // First change should produce update
        let change1 = RepositoryChange {
            path: source.clone(),
            old_value: None,
            new_value: Some(Value::Integer(1)),
            timestamp: chrono::Utc::now(),
        };
        let updates1 = system.on_data_changed(&change1);
        assert_eq!(updates1.len(), 1);

        // Second change should not produce update
        let change2 = RepositoryChange {
            path: source,
            old_value: Some(Value::Integer(1)),
            new_value: Some(Value::Integer(2)),
            timestamp: chrono::Utc::now(),
        };
        let updates2 = system.on_data_changed(&change2);
        assert_eq!(updates2.len(), 0);
    }
}
