//! Nemo Plugin API - Shared interface for native plugins.
//!
//! This crate defines the stable API boundary between the Nemo host and native plugins.
//! Plugins link against this crate to register their capabilities.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error from plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin initialization failed.
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    /// Component creation failed.
    #[error("Component creation failed: {0}")]
    ComponentFailed(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// A configuration value (simplified for FFI safety).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginValue {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// Array of values.
    Array(Vec<PluginValue>),
    /// Object (map) of values.
    Object(HashMap<String, PluginValue>),
}

impl Default for PluginValue {
    fn default() -> Self {
        Self::Null
    }
}

/// Plugin manifest describing capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Plugin version.
    pub version: Version,
    /// Description.
    pub description: String,
    /// Author information.
    pub author: Option<String>,
    /// Capabilities provided.
    pub capabilities: Vec<Capability>,
    /// Required permissions.
    pub permissions: PluginPermissions,
}

impl PluginManifest {
    /// Creates a new plugin manifest.
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: Version) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version,
            description: String::new(),
            author: None,
            capabilities: Vec::new(),
            permissions: PluginPermissions::default(),
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Adds a capability.
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }
}

/// Plugin capability type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// Provides a UI component.
    Component(String),
    /// Provides a data source.
    DataSource(String),
    /// Provides a transform.
    Transform(String),
    /// Provides an action.
    Action(String),
    /// Provides an event handler.
    EventHandler(String),
}

/// Permissions requested by a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// Can make network requests.
    pub network: bool,
    /// Can access filesystem.
    pub filesystem: bool,
    /// Can spawn subprocesses.
    pub subprocess: bool,
    /// Allowed data paths.
    pub data_paths: Vec<String>,
    /// Allowed event types.
    pub event_types: Vec<String>,
}

/// Trait for plugin registration.
pub trait PluginRegistrar {
    /// Registers a component factory.
    fn register_component(&mut self, name: &str, schema: ComponentSchema);

    /// Registers a data source factory.
    fn register_data_source(&mut self, name: &str, schema: DataSourceSchema);

    /// Registers a transform.
    fn register_transform(&mut self, name: &str, schema: TransformSchema);

    /// Registers an action.
    fn register_action(&mut self, name: &str, schema: ActionSchema);

    /// Gets the plugin context for API access.
    fn context(&self) -> &dyn PluginContext;
}

/// Context providing API access to plugins.
pub trait PluginContext: Send + Sync {
    /// Gets data by path.
    fn get_data(&self, path: &str) -> Option<PluginValue>;

    /// Sets data at path.
    fn set_data(&self, path: &str, value: PluginValue) -> Result<(), PluginError>;

    /// Emits an event.
    fn emit_event(&self, event_type: &str, payload: PluginValue);

    /// Gets configuration value.
    fn get_config(&self, path: &str) -> Option<PluginValue>;

    /// Logs a message.
    fn log(&self, level: LogLevel, message: &str);

    /// Gets a component property by component ID and property name.
    fn get_component_property(&self, component_id: &str, property: &str) -> Option<PluginValue>;

    /// Sets a component property by component ID and property name.
    fn set_component_property(
        &self,
        component_id: &str,
        property: &str,
        value: PluginValue,
    ) -> Result<(), PluginError>;
}

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level.
    Debug,
    /// Info level.
    Info,
    /// Warning level.
    Warn,
    /// Error level.
    Error,
}

/// Schema for a component.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Component name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Configuration properties.
    pub properties: HashMap<String, PropertySchema>,
    /// Required properties.
    pub required: Vec<String>,
}

impl ComponentSchema {
    /// Creates a new component schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Adds a property.
    pub fn with_property(mut self, name: impl Into<String>, schema: PropertySchema) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    /// Marks a property as required.
    pub fn require(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }
}

/// Schema for a property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Property type.
    pub property_type: PropertyType,
    /// Description.
    pub description: Option<String>,
    /// Default value.
    pub default: Option<PluginValue>,
}

impl PropertySchema {
    /// Creates a string property schema.
    pub fn string() -> Self {
        Self {
            property_type: PropertyType::String,
            description: None,
            default: None,
        }
    }

    /// Creates a boolean property schema.
    pub fn boolean() -> Self {
        Self {
            property_type: PropertyType::Boolean,
            description: None,
            default: None,
        }
    }

    /// Creates an integer property schema.
    pub fn integer() -> Self {
        Self {
            property_type: PropertyType::Integer,
            description: None,
            default: None,
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the default value.
    pub fn with_default(mut self, default: PluginValue) -> Self {
        self.default = Some(default);
        self
    }
}

/// Property type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    /// String type.
    String,
    /// Boolean type.
    Boolean,
    /// Integer type.
    Integer,
    /// Float type.
    Float,
    /// Array type.
    Array,
    /// Object type.
    Object,
    /// Any type.
    Any,
}

/// Schema for a data source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataSourceSchema {
    /// Data source name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Supports polling.
    pub supports_polling: bool,
    /// Supports streaming.
    pub supports_streaming: bool,
    /// Configuration properties.
    pub properties: HashMap<String, PropertySchema>,
}

impl DataSourceSchema {
    /// Creates a new data source schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Schema for a transform.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransformSchema {
    /// Transform name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Configuration properties.
    pub properties: HashMap<String, PropertySchema>,
}

impl TransformSchema {
    /// Creates a new transform schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Schema for an action.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionSchema {
    /// Action name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Whether action is async.
    pub is_async: bool,
    /// Configuration properties.
    pub properties: HashMap<String, PropertySchema>,
}

impl ActionSchema {
    /// Creates a new action schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Plugin entry point function type.
pub type PluginEntryFn = unsafe extern "C" fn(&mut dyn PluginRegistrar);

/// Macro to declare a plugin entry point.
#[macro_export]
macro_rules! declare_plugin {
    ($manifest:expr, $init:expr) => {
        #[no_mangle]
        pub extern "C" fn nemo_plugin_manifest() -> $crate::PluginManifest {
            $manifest
        }

        #[no_mangle]
        pub extern "C" fn nemo_plugin_entry(registrar: &mut dyn $crate::PluginRegistrar) {
            $init(registrar)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest() {
        let manifest = PluginManifest::new("test-plugin", "Test Plugin", Version::new(1, 0, 0))
            .with_description("A test plugin")
            .with_capability(Capability::Component("my-component".into()));

        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.capabilities.len(), 1);
    }

    #[test]
    fn test_component_schema() {
        let schema = ComponentSchema::new("button")
            .with_description("A button component")
            .with_property("label", PropertySchema::string())
            .require("label");

        assert!(schema.required.contains(&"label".to_string()));
    }

    #[test]
    fn test_plugin_value() {
        let value = PluginValue::Object(HashMap::from([
            ("name".to_string(), PluginValue::String("test".to_string())),
            ("count".to_string(), PluginValue::Integer(42)),
        ]));

        if let PluginValue::Object(obj) = value {
            assert!(obj.contains_key("name"));
        }
    }
}
