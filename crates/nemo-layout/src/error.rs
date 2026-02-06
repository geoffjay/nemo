//! Error types for the layout engine.

use thiserror::Error;

/// Error during layout building.
#[derive(Debug, Error)]
pub enum LayoutError {
    /// Component type not found in registry.
    #[error("Unknown component type: {type_name}")]
    UnknownComponent { type_name: String },

    /// Invalid component configuration.
    #[error("Invalid configuration for component '{component_id}': {reason}")]
    InvalidConfig { component_id: String, reason: String },

    /// Binding error.
    #[error("Binding error for '{binding_id}': {reason}")]
    BindingError { binding_id: String, reason: String },

    /// Invalid layout structure.
    #[error("Invalid layout structure: {reason}")]
    InvalidStructure { reason: String },

    /// Component render error.
    #[error("Render error for component '{component_id}': {reason}")]
    RenderError { component_id: String, reason: String },

    /// Missing required property.
    #[error("Missing required property '{property}' for component '{component_id}'")]
    MissingProperty {
        component_id: String,
        property: String,
    },

    /// Invalid property value.
    #[error("Invalid value for property '{property}' in component '{component_id}': {reason}")]
    InvalidPropertyValue {
        component_id: String,
        property: String,
        reason: String,
    },

    /// Circular reference detected.
    #[error("Circular reference detected involving component '{component_id}'")]
    CircularReference { component_id: String },

    /// Registry error.
    #[error("Registry error: {0}")]
    Registry(String),
}

/// Error during state operations.
#[derive(Debug, Error)]
pub enum StateError {
    /// State not found.
    #[error("State not found for component '{component_id}'")]
    NotFound { component_id: String },

    /// Failed to serialize state.
    #[error("Failed to serialize state: {0}")]
    SerializationFailed(String),

    /// Failed to deserialize state.
    #[error("Failed to deserialize state: {0}")]
    DeserializationFailed(String),

    /// Persistence error.
    #[error("Persistence error: {0}")]
    PersistenceFailed(String),
}

/// Error during binding operations.
#[derive(Debug, Error)]
pub enum BindingError {
    /// Source path not found.
    #[error("Source path not found: {0}")]
    SourceNotFound(String),

    /// Target not found.
    #[error("Target not found: {0}")]
    TargetNotFound(String),

    /// Transform error.
    #[error("Transform error: {0}")]
    TransformFailed(String),

    /// Type conversion error.
    #[error("Type conversion error: cannot convert {from} to {to}")]
    TypeConversion { from: String, to: String },
}
