//! Nemo Component Registry - Central catalog of components, data sources, transforms, and actions.
//!
//! This crate provides the registry system for storing and looking up all available
//! components, data sources, transforms, and actions in a Nemo application.

pub mod builtins;
pub mod descriptor;
pub mod error;
pub mod registry;

pub use builtins::{
    register_all_builtins, register_builtin_actions, register_builtin_components,
    register_builtin_data_sources, register_builtin_transforms,
};
pub use descriptor::{
    ActionDescriptor, ActionError, ActionFactory, ActionMetadata, BindableProperty,
    BindingDirection, ComponentCategory, ComponentDescriptor, ComponentError, ComponentFactory,
    ComponentMetadata, DataSourceDescriptor, DataSourceError, DataSourceFactory,
    DataSourceMetadata, DescriptorSource, EventSpec, Example, SlotSpec, TransformDescriptor,
    TransformError, TransformFactory, TransformMetadata,
};
pub use error::RegistrationError;
pub use registry::{ComponentRegistry, EntityType, RegistryChange};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_builtins() {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);

        // Should have components
        let components = registry.list_components();
        assert!(!components.is_empty());

        // Should be able to search
        let results = registry.search_components("button");
        assert!(!results.is_empty());

        // Should be able to get by category
        let inputs = registry.list_by_category(ComponentCategory::Input);
        assert!(!inputs.is_empty());
    }
}
