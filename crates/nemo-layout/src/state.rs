//! State coordination for component state persistence.

use crate::error::StateError;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Component state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    /// State values by key.
    pub values: HashMap<String, Value>,
    /// Metadata about the state.
    pub metadata: StateMetadata,
}

impl ComponentState {
    /// Creates a new empty state.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            metadata: StateMetadata::default(),
        }
    }

    /// Sets a state value.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.values.insert(key.into(), value);
        self.metadata.modified = true;
    }

    /// Gets a state value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Removes a state value.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.metadata.modified = true;
        self.values.remove(key)
    }

    /// Clears all state.
    pub fn clear(&mut self) {
        self.values.clear();
        self.metadata.modified = true;
    }

    /// Returns true if the state has been modified since last save.
    pub fn is_modified(&self) -> bool {
        self.metadata.modified
    }

    /// Marks the state as saved.
    pub fn mark_saved(&mut self) {
        self.metadata.modified = false;
    }
}

impl Default for ComponentState {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about component state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Whether the state has been modified since last save.
    pub modified: bool,
    /// Version of the state schema.
    pub version: u32,
}

/// Trait for state persistence backends.
pub trait StatePersistence: Send + Sync {
    /// Saves states to storage.
    fn save(&self, states: &HashMap<String, ComponentState>) -> Result<(), StateError>;

    /// Loads states from storage.
    fn load(&self) -> Result<HashMap<String, ComponentState>, StateError>;
}

/// In-memory state persistence (for testing).
pub struct MemoryPersistence {
    states: std::sync::RwLock<HashMap<String, ComponentState>>,
}

impl MemoryPersistence {
    /// Creates a new memory persistence.
    pub fn new() -> Self {
        Self {
            states: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPersistence {
    fn default() -> Self {
        Self::new()
    }
}

impl StatePersistence for MemoryPersistence {
    fn save(&self, states: &HashMap<String, ComponentState>) -> Result<(), StateError> {
        let mut storage = self
            .states
            .write()
            .map_err(|_| StateError::PersistenceFailed("Lock error".to_string()))?;
        *storage = states.clone();
        Ok(())
    }

    fn load(&self) -> Result<HashMap<String, ComponentState>, StateError> {
        let storage = self
            .states
            .read()
            .map_err(|_| StateError::PersistenceFailed("Lock error".to_string()))?;
        Ok(storage.clone())
    }
}

/// Coordinator for managing component state across sessions.
pub struct StateCoordinator {
    /// States by component ID.
    states: HashMap<String, ComponentState>,
    /// Optional persistence backend.
    persistence: Option<Box<dyn StatePersistence>>,
}

impl StateCoordinator {
    /// Creates a new state coordinator.
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            persistence: None,
        }
    }

    /// Creates a new state coordinator with persistence.
    pub fn with_persistence(persistence: Box<dyn StatePersistence>) -> Self {
        Self {
            states: HashMap::new(),
            persistence: Some(persistence),
        }
    }

    /// Gets state for a component.
    pub fn get_state(&self, component_id: &str) -> Option<&ComponentState> {
        self.states.get(component_id)
    }

    /// Gets mutable state for a component.
    pub fn get_state_mut(&mut self, component_id: &str) -> Option<&mut ComponentState> {
        self.states.get_mut(component_id)
    }

    /// Gets or creates state for a component.
    pub fn get_or_create(&mut self, component_id: &str) -> &mut ComponentState {
        self.states
            .entry(component_id.to_string())
            .or_insert_with(ComponentState::new)
    }

    /// Sets state for a component.
    pub fn set_state(&mut self, component_id: impl Into<String>, state: ComponentState) {
        self.states.insert(component_id.into(), state);
    }

    /// Removes state for a component.
    pub fn remove_state(&mut self, component_id: &str) -> Option<ComponentState> {
        self.states.remove(component_id)
    }

    /// Returns all component IDs with state.
    pub fn component_ids(&self) -> Vec<String> {
        self.states.keys().cloned().collect()
    }

    /// Saves all states to persistence.
    pub fn persist(&mut self) -> Result<(), StateError> {
        if let Some(persistence) = &self.persistence {
            persistence.save(&self.states)?;

            // Mark all as saved
            for state in self.states.values_mut() {
                state.mark_saved();
            }
        }
        Ok(())
    }

    /// Restores states from persistence.
    pub fn restore(&mut self) -> Result<(), StateError> {
        if let Some(persistence) = &self.persistence {
            self.states = persistence.load()?;
        }
        Ok(())
    }

    /// Returns true if any state has been modified.
    pub fn has_modifications(&self) -> bool {
        self.states.values().any(|s| s.is_modified())
    }

    /// Clears all state.
    pub fn clear(&mut self) {
        self.states.clear();
    }
}

impl Default for StateCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_state() {
        let mut state = ComponentState::new();
        state.set("key", Value::Integer(42));

        assert_eq!(state.get("key"), Some(&Value::Integer(42)));
        assert!(state.is_modified());

        state.mark_saved();
        assert!(!state.is_modified());
    }

    #[test]
    fn test_state_coordinator() {
        let mut coordinator = StateCoordinator::new();

        let state = coordinator.get_or_create("comp1");
        state.set("value", Value::String("test".into()));

        assert!(coordinator.get_state("comp1").is_some());
        assert!(coordinator.get_state("comp2").is_none());
    }

    #[test]
    fn test_memory_persistence() {
        let persistence = Box::new(MemoryPersistence::new());
        let mut coordinator = StateCoordinator::with_persistence(persistence);

        {
            let state = coordinator.get_or_create("comp1");
            state.set("value", Value::Integer(100));
        }

        coordinator.persist().unwrap();

        // Clear and restore
        coordinator.clear();
        assert!(coordinator.get_state("comp1").is_none());

        coordinator.restore().unwrap();
        assert!(coordinator.get_state("comp1").is_some());
    }
}
