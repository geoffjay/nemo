//! Central component registry.

use crate::descriptor::{
    ActionDescriptor, ComponentCategory, ComponentDescriptor, DataSourceDescriptor,
    DescriptorSource, TransformDescriptor,
};
use crate::error::RegistrationError;
use nemo_config::ConfigSchema;
use std::collections::HashMap;
use std::sync::RwLock;

/// Type of entity in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// UI component.
    Component,
    /// Data source.
    DataSource,
    /// Transform.
    Transform,
    /// Action.
    Action,
}

/// Event indicating a change in the registry.
#[derive(Debug, Clone)]
pub enum RegistryChange {
    /// An entity was added.
    Added { entity_type: EntityType, name: String },
    /// An entity was removed.
    Removed { entity_type: EntityType, name: String },
    /// An entity was updated.
    Updated { entity_type: EntityType, name: String },
}

/// Central registry for all component types.
pub struct ComponentRegistry {
    components: RwLock<HashMap<String, ComponentDescriptor>>,
    data_sources: RwLock<HashMap<String, DataSourceDescriptor>>,
    transforms: RwLock<HashMap<String, TransformDescriptor>>,
    actions: RwLock<HashMap<String, ActionDescriptor>>,
}

impl ComponentRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        ComponentRegistry {
            components: RwLock::new(HashMap::new()),
            data_sources: RwLock::new(HashMap::new()),
            transforms: RwLock::new(HashMap::new()),
            actions: RwLock::new(HashMap::new()),
        }
    }

    // ---- Component Registration ----

    /// Registers a component descriptor.
    pub fn register_component(
        &self,
        descriptor: ComponentDescriptor,
    ) -> Result<(), RegistrationError> {
        let mut components = self.components.write().map_err(|_| RegistrationError::LockError)?;

        if components.contains_key(&descriptor.name) {
            return Err(RegistrationError::AlreadyRegistered {
                entity_type: EntityType::Component,
                name: descriptor.name.clone(),
            });
        }

        components.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Gets a component descriptor by name.
    pub fn get_component(&self, name: &str) -> Option<ComponentDescriptor> {
        self.components
            .read()
            .ok()
            .and_then(|c| c.get(name).cloned())
    }

    /// Lists all registered components.
    pub fn list_components(&self) -> Vec<ComponentDescriptor> {
        self.components
            .read()
            .ok()
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Lists components by category.
    pub fn list_by_category(&self, category: ComponentCategory) -> Vec<ComponentDescriptor> {
        self.components
            .read()
            .ok()
            .map(|c| {
                c.values()
                    .filter(|d| d.category == category)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ---- Data Source Registration ----

    /// Registers a data source descriptor.
    pub fn register_data_source(
        &self,
        descriptor: DataSourceDescriptor,
    ) -> Result<(), RegistrationError> {
        let mut sources = self
            .data_sources
            .write()
            .map_err(|_| RegistrationError::LockError)?;

        if sources.contains_key(&descriptor.name) {
            return Err(RegistrationError::AlreadyRegistered {
                entity_type: EntityType::DataSource,
                name: descriptor.name.clone(),
            });
        }

        sources.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Gets a data source descriptor by name.
    pub fn get_data_source(&self, name: &str) -> Option<DataSourceDescriptor> {
        self.data_sources
            .read()
            .ok()
            .and_then(|d| d.get(name).cloned())
    }

    /// Lists all registered data sources.
    pub fn list_data_sources(&self) -> Vec<DataSourceDescriptor> {
        self.data_sources
            .read()
            .ok()
            .map(|d| d.values().cloned().collect())
            .unwrap_or_default()
    }

    // ---- Transform Registration ----

    /// Registers a transform descriptor.
    pub fn register_transform(
        &self,
        descriptor: TransformDescriptor,
    ) -> Result<(), RegistrationError> {
        let mut transforms = self
            .transforms
            .write()
            .map_err(|_| RegistrationError::LockError)?;

        if transforms.contains_key(&descriptor.name) {
            return Err(RegistrationError::AlreadyRegistered {
                entity_type: EntityType::Transform,
                name: descriptor.name.clone(),
            });
        }

        transforms.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Gets a transform descriptor by name.
    pub fn get_transform(&self, name: &str) -> Option<TransformDescriptor> {
        self.transforms
            .read()
            .ok()
            .and_then(|t| t.get(name).cloned())
    }

    /// Lists all registered transforms.
    pub fn list_transforms(&self) -> Vec<TransformDescriptor> {
        self.transforms
            .read()
            .ok()
            .map(|t| t.values().cloned().collect())
            .unwrap_or_default()
    }

    // ---- Action Registration ----

    /// Registers an action descriptor.
    pub fn register_action(&self, descriptor: ActionDescriptor) -> Result<(), RegistrationError> {
        let mut actions = self.actions.write().map_err(|_| RegistrationError::LockError)?;

        if actions.contains_key(&descriptor.name) {
            return Err(RegistrationError::AlreadyRegistered {
                entity_type: EntityType::Action,
                name: descriptor.name.clone(),
            });
        }

        actions.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Gets an action descriptor by name.
    pub fn get_action(&self, name: &str) -> Option<ActionDescriptor> {
        self.actions.read().ok().and_then(|a| a.get(name).cloned())
    }

    /// Lists all registered actions.
    pub fn list_actions(&self) -> Vec<ActionDescriptor> {
        self.actions
            .read()
            .ok()
            .map(|a| a.values().cloned().collect())
            .unwrap_or_default()
    }

    // ---- Schema Access ----

    /// Gets the schema for an entity.
    pub fn get_schema(&self, entity_type: EntityType, name: &str) -> Option<ConfigSchema> {
        match entity_type {
            EntityType::Component => self.get_component(name).map(|d| d.schema),
            EntityType::DataSource => self.get_data_source(name).map(|d| d.schema),
            EntityType::Transform => self.get_transform(name).map(|d| d.schema),
            EntityType::Action => self.get_action(name).map(|d| d.schema),
        }
    }

    // ---- Discovery ----

    /// Checks if a component is registered.
    pub fn has_component(&self, name: &str) -> bool {
        self.components
            .read()
            .ok()
            .map(|c| c.contains_key(name))
            .unwrap_or(false)
    }

    /// Checks if a data source is registered.
    pub fn has_data_source(&self, name: &str) -> bool {
        self.data_sources
            .read()
            .ok()
            .map(|d| d.contains_key(name))
            .unwrap_or(false)
    }

    /// Checks if a transform is registered.
    pub fn has_transform(&self, name: &str) -> bool {
        self.transforms
            .read()
            .ok()
            .map(|t| t.contains_key(name))
            .unwrap_or(false)
    }

    /// Checks if an action is registered.
    pub fn has_action(&self, name: &str) -> bool {
        self.actions
            .read()
            .ok()
            .map(|a| a.contains_key(name))
            .unwrap_or(false)
    }

    /// Returns all component names.
    pub fn component_names(&self) -> Vec<String> {
        self.components
            .read()
            .ok()
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all data source names.
    pub fn data_source_names(&self) -> Vec<String> {
        self.data_sources
            .read()
            .ok()
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all transform names.
    pub fn transform_names(&self) -> Vec<String> {
        self.transforms
            .read()
            .ok()
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all action names.
    pub fn action_names(&self) -> Vec<String> {
        self.actions
            .read()
            .ok()
            .map(|a| a.keys().cloned().collect())
            .unwrap_or_default()
    }

    // ---- Search ----

    /// Searches for components matching a query.
    pub fn search_components(&self, query: &str) -> Vec<ComponentDescriptor> {
        let query_lower = query.to_lowercase();
        self.components
            .read()
            .ok()
            .map(|c| {
                c.values()
                    .filter(|d| {
                        d.name.to_lowercase().contains(&query_lower)
                            || d.metadata.display_name.to_lowercase().contains(&query_lower)
                            || d.metadata.description.to_lowercase().contains(&query_lower)
                            || d.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ---- Unregistration ----

    /// Unregisters a component.
    pub fn unregister_component(&self, name: &str) -> Option<ComponentDescriptor> {
        self.components.write().ok().and_then(|mut c| c.remove(name))
    }

    /// Unregisters a data source.
    pub fn unregister_data_source(&self, name: &str) -> Option<DataSourceDescriptor> {
        self.data_sources
            .write()
            .ok()
            .and_then(|mut d| d.remove(name))
    }

    /// Unregisters a transform.
    pub fn unregister_transform(&self, name: &str) -> Option<TransformDescriptor> {
        self.transforms.write().ok().and_then(|mut t| t.remove(name))
    }

    /// Unregisters an action.
    pub fn unregister_action(&self, name: &str) -> Option<ActionDescriptor> {
        self.actions.write().ok().and_then(|mut a| a.remove(name))
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_component() {
        let registry = ComponentRegistry::new();

        let descriptor = ComponentDescriptor::new("button", ComponentCategory::Input);
        registry.register_component(descriptor).unwrap();

        assert!(registry.has_component("button"));
        let retrieved = registry.get_component("button").unwrap();
        assert_eq!(retrieved.name, "button");
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = ComponentRegistry::new();

        let d1 = ComponentDescriptor::new("button", ComponentCategory::Input);
        let d2 = ComponentDescriptor::new("button", ComponentCategory::Input);

        registry.register_component(d1).unwrap();
        let result = registry.register_component(d2);

        assert!(matches!(result, Err(RegistrationError::AlreadyRegistered { .. })));
    }

    #[test]
    fn test_list_by_category() {
        let registry = ComponentRegistry::new();

        registry
            .register_component(ComponentDescriptor::new("button", ComponentCategory::Input))
            .unwrap();
        registry
            .register_component(ComponentDescriptor::new("label", ComponentCategory::Display))
            .unwrap();
        registry
            .register_component(ComponentDescriptor::new("checkbox", ComponentCategory::Input))
            .unwrap();

        let inputs = registry.list_by_category(ComponentCategory::Input);
        assert_eq!(inputs.len(), 2);

        let displays = registry.list_by_category(ComponentCategory::Display);
        assert_eq!(displays.len(), 1);
    }

    #[test]
    fn test_search_components() {
        let registry = ComponentRegistry::new();

        let mut button = ComponentDescriptor::new("button", ComponentCategory::Input);
        button.metadata.display_name = "Button".to_string();
        button.tags = vec!["interactive".to_string(), "clickable".to_string()];
        registry.register_component(button).unwrap();

        let results = registry.search_components("click");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "button");
    }

    #[test]
    fn test_unregister() {
        let registry = ComponentRegistry::new();

        registry
            .register_component(ComponentDescriptor::new("button", ComponentCategory::Input))
            .unwrap();

        assert!(registry.has_component("button"));
        registry.unregister_component("button");
        assert!(!registry.has_component("button"));
    }
}
