//! Nemo Plugin API - Shared interface for native plugins.
//!
//! This crate defines the stable API boundary between the Nemo host and native plugins.
//! Plugins link against this crate to register their capabilities.
//!
//! # Writing a Plugin
//!
//! A Nemo plugin is a dynamic library (`cdylib`) that exports two symbols:
//! - `nemo_plugin_manifest` - returns a [`PluginManifest`] describing the plugin
//! - `nemo_plugin_entry` - called with a [`PluginRegistrar`] to register components,
//!   data sources, transforms, actions, and templates
//!
//! Use the [`declare_plugin!`] macro to generate both exports.
//!
//! ## Minimal Example
//!
//! ```rust,no_run
//! use nemo_plugin_api::*;
//! use semver::Version;
//!
//! fn init(registrar: &mut dyn PluginRegistrar) {
//!     // Register a custom component
//!     registrar.register_component(
//!         "my_counter",
//!         ComponentSchema::new("my_counter")
//!             .with_description("A counter component")
//!             .with_property("initial", PropertySchema::integer())
//!             .require("initial"),
//!     );
//!
//!     // Access host data
//!     if let Some(value) = registrar.context().get_data("app.settings.theme") {
//!         registrar.context().log(LogLevel::Info, &format!("Theme: {:?}", value));
//!     }
//! }
//!
//! declare_plugin!(
//!     PluginManifest::new("my-plugin", "My Plugin", Version::new(0, 1, 0))
//!         .with_description("Example Nemo plugin")
//!         .with_capability(Capability::Component("my_counter".into())),
//!     init
//! );
//! ```
//!
//! ## Registering a Data Source
//!
//! ```rust,no_run
//! # use nemo_plugin_api::*;
//! fn init(registrar: &mut dyn PluginRegistrar) {
//!     let mut schema = DataSourceSchema::new("my_feed");
//!     schema.description = "Streams data from a custom feed".into();
//!     schema.supports_streaming = true;
//!     schema.properties.insert(
//!         "url".into(),
//!         PropertySchema::string().with_description("Feed URL"),
//!     );
//!     registrar.register_data_source("my_feed", schema);
//! }
//! ```
//!
//! ## Plugin Permissions
//!
//! Plugins declare required permissions in their manifest via [`PluginPermissions`].
//! The host checks these before granting access to network, filesystem, or
//! subprocess operations.

use indexmap::IndexMap;
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginValue {
    /// Null value.
    #[default]
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
    /// Object (map) of values, preserving insertion order.
    Object(IndexMap<String, PluginValue>),
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
///
/// Passed to the plugin entry point function. Plugins use this to register
/// their capabilities (components, data sources, transforms, actions, and
/// templates) with the Nemo host.
///
/// The registrar is only valid during the plugin entry call and must not be
/// stored or used after the entry function returns.
///
/// # Example
///
/// ```rust,no_run
/// # use nemo_plugin_api::*;
/// fn init(registrar: &mut dyn PluginRegistrar) {
///     registrar.register_component(
///         "widget",
///         ComponentSchema::new("widget")
///             .with_property("title", PropertySchema::string()),
///     );
///     registrar.register_action(
///         "refresh_widget",
///         ActionSchema::new("refresh_widget"),
///     );
/// }
/// ```
pub trait PluginRegistrar {
    /// Registers a component factory with the given name and schema.
    fn register_component(&mut self, name: &str, schema: ComponentSchema);

    /// Registers a data source factory with the given name and schema.
    fn register_data_source(&mut self, name: &str, schema: DataSourceSchema);

    /// Registers a transform with the given name and schema.
    fn register_transform(&mut self, name: &str, schema: TransformSchema);

    /// Registers an action with the given name and schema.
    fn register_action(&mut self, name: &str, schema: ActionSchema);

    /// Registers a UI template that can be referenced in HCL layout configs.
    ///
    /// Templates registered by plugins are merged with HCL-defined templates
    /// during layout expansion. HCL-defined templates take precedence if there
    /// is a name collision.
    fn register_template(&mut self, name: &str, template: PluginValue);

    /// Gets the plugin context for API access during initialization.
    fn context(&self) -> &dyn PluginContext;

    /// Gets the plugin context as an `Arc` for use in background threads.
    ///
    /// The returned `Arc<dyn PluginContext>` is `Send + Sync` and can safely
    /// be moved to spawned tasks.
    fn context_arc(&self) -> std::sync::Arc<dyn PluginContext>;
}

/// Context providing API access to plugins at runtime.
///
/// This trait is `Send + Sync`, allowing plugins to use it from background
/// threads and async tasks. Obtain an `Arc<dyn PluginContext>` via
/// [`PluginRegistrar::context_arc`] for shared ownership.
///
/// # Data Paths
///
/// Data paths use dot-separated notation: `"data.my_source.items"`.
/// Config paths follow the same convention: `"app.settings.theme"`.
///
/// # Example
///
/// ```rust,no_run
/// # use nemo_plugin_api::*;
/// fn read_and_write(ctx: &dyn PluginContext) {
///     // Read a value
///     if let Some(PluginValue::Integer(count)) = ctx.get_data("metrics.request_count") {
///         ctx.log(LogLevel::Info, &format!("Requests: {}", count));
///     }
///
///     // Write a value
///     ctx.set_data("metrics.last_check", PluginValue::String("now".into())).ok();
///
///     // Emit an event for other plugins/components to observe
///     ctx.emit_event("plugin:refresh", PluginValue::Null);
/// }
/// ```
pub trait PluginContext: Send + Sync {
    /// Gets data by dot-separated path (e.g. `"data.source.field"`).
    fn get_data(&self, path: &str) -> Option<PluginValue>;

    /// Sets data at a dot-separated path.
    ///
    /// Returns `Err` if the path is invalid or permission is denied.
    fn set_data(&self, path: &str, value: PluginValue) -> Result<(), PluginError>;

    /// Emits a named event with an arbitrary payload.
    ///
    /// Events are delivered asynchronously via the host's event bus.
    fn emit_event(&self, event_type: &str, payload: PluginValue);

    /// Gets a configuration value by dot-separated path.
    fn get_config(&self, path: &str) -> Option<PluginValue>;

    /// Logs a message at the given severity level.
    fn log(&self, level: LogLevel, message: &str);

    /// Gets a component property by component ID and property name.
    ///
    /// Returns `None` if the component or property does not exist.
    fn get_component_property(&self, component_id: &str, property: &str) -> Option<PluginValue>;

    /// Sets a component property by component ID and property name.
    ///
    /// Returns `Err` if the component does not exist or the property is
    /// read-only.
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
///
/// # Safety
///
/// This type is the signature for native plugin entry points loaded via
/// `dlopen`/`LoadLibrary`. The following invariants must hold:
///
/// - **ABI compatibility**: The plugin must be compiled with the same Rust
///   compiler version and the same `nemo-plugin-api` crate version as the
///   host. Mismatched versions cause undefined behaviour due to differing
///   type layouts.
/// - **Single-threaded call**: The host calls this function on the main
///   thread. The `PluginRegistrar` reference is valid only for the duration
///   of the call and must not be stored or sent to other threads.
/// - **No unwinding**: The entry function must not panic. A panic across
///   the FFI boundary is undefined behaviour. Use `catch_unwind` internally
///   if necessary.
/// - **Library lifetime**: The dynamic library must remain loaded for as
///   long as any symbols (vtables, function pointers) obtained through the
///   registrar are in use.
/// - **No re-entrancy**: The entry function must not call back into the
///   host's plugin loading machinery.
#[allow(improper_ctypes_definitions)]
pub type PluginEntryFn = unsafe extern "C" fn(&mut dyn PluginRegistrar);

/// Macro to declare a plugin entry point.
///
/// Generates two `extern "C"` functions:
/// - `nemo_plugin_manifest() -> PluginManifest` — returns the plugin descriptor
/// - `nemo_plugin_entry(registrar: &mut dyn PluginRegistrar)` — called to
///   register capabilities
///
/// # Example
///
/// ```rust,no_run
/// # use nemo_plugin_api::*;
/// # use semver::Version;
/// declare_plugin!(
///     PluginManifest::new("hello", "Hello Plugin", Version::new(1, 0, 0))
///         .with_description("Greets the user"),
///     |registrar: &mut dyn PluginRegistrar| {
///         registrar.context().log(LogLevel::Info, "Hello from plugin!");
///     }
/// );
/// ```
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
        let value = PluginValue::Object(IndexMap::from([
            ("name".to_string(), PluginValue::String("test".to_string())),
            ("count".to_string(), PluginValue::Integer(42)),
        ]));

        if let PluginValue::Object(obj) = value {
            assert!(obj.contains_key("name"));
        }
    }
}
