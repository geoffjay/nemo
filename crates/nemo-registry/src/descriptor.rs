//! Component and entity descriptors.

use nemo_config::{ConfigSchema, Value};
use semver::Version;
use std::sync::Arc;

/// Source of a descriptor registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorSource {
    /// Built into the framework.
    BuiltIn,
    /// Provided by a plugin.
    Plugin { plugin_id: String },
    /// Provided by a script.
    Script { script_id: String },
}

/// Category of UI component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentCategory {
    /// Layout containers (dock, stack, tabs).
    Layout,
    /// Display elements (label, icon, image).
    Display,
    /// Input elements (button, text field, checkbox).
    Input,
    /// Data display (table, list, tree).
    Data,
    /// Feedback elements (modal, notification, tooltip).
    Feedback,
    /// Navigation elements (menu, tabs, breadcrumbs).
    Navigation,
    /// Charts and visualizations.
    Charts,
    /// Custom plugin-provided components.
    Custom,
}

/// Descriptor for a UI component.
#[derive(Debug, Clone)]
pub struct ComponentDescriptor {
    /// Unique name of the component.
    pub name: String,
    /// Version of the component.
    pub version: Version,
    /// Source of this registration.
    pub source: DescriptorSource,
    /// Component category.
    pub category: ComponentCategory,
    /// Tags for discovery.
    pub tags: Vec<String>,
    /// Configuration schema.
    pub schema: ConfigSchema,
    /// Factory for creating instances.
    pub factory: Option<Arc<dyn ComponentFactory>>,
    /// Component metadata.
    pub metadata: ComponentMetadata,
}

impl ComponentDescriptor {
    /// Creates a new component descriptor.
    pub fn new(name: impl Into<String>, category: ComponentCategory) -> Self {
        let name = name.into();
        ComponentDescriptor {
            name: name.clone(),
            version: Version::new(0, 1, 0),
            source: DescriptorSource::BuiltIn,
            category,
            tags: Vec::new(),
            schema: ConfigSchema::new(name),
            factory: None,
            metadata: ComponentMetadata::default(),
        }
    }

    /// Sets the version.
    pub fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Sets the source.
    pub fn source(mut self, source: DescriptorSource) -> Self {
        self.source = source;
        self
    }

    /// Adds a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Sets the schema.
    pub fn schema(mut self, schema: ConfigSchema) -> Self {
        self.schema = schema;
        self
    }

    /// Sets the factory.
    pub fn factory(mut self, factory: Arc<dyn ComponentFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: ComponentMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Metadata about a component.
#[derive(Debug, Clone, Default)]
pub struct ComponentMetadata {
    /// Human-readable display name.
    pub display_name: String,
    /// Description of the component.
    pub description: String,
    /// Icon name.
    pub icon: Option<String>,
    /// Bindable properties.
    pub bindable_properties: Vec<BindableProperty>,
    /// Events emitted.
    pub events: Vec<EventSpec>,
    /// Slots for child components.
    pub slots: Vec<SlotSpec>,
    /// Documentation URL.
    pub docs_url: Option<String>,
    /// Example configurations.
    pub examples: Vec<Example>,
}

/// A property that can be bound to data.
#[derive(Debug, Clone)]
pub struct BindableProperty {
    /// Property name.
    pub name: String,
    /// Property type description.
    pub property_type: String,
    /// Binding direction.
    pub direction: BindingDirection,
    /// Description.
    pub description: String,
}

/// Direction of data binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingDirection {
    /// Data flows into the component.
    In,
    /// Data flows out of the component.
    Out,
    /// Data flows both ways.
    InOut,
}

/// Specification for an event emitted by a component.
#[derive(Debug, Clone)]
pub struct EventSpec {
    /// Event name.
    pub name: String,
    /// Payload schema (optional).
    pub payload_schema: Option<ConfigSchema>,
    /// Description.
    pub description: String,
}

/// Specification for a slot that can contain child components.
#[derive(Debug, Clone)]
pub struct SlotSpec {
    /// Slot name.
    pub name: String,
    /// Component types accepted.
    pub accepts: Vec<String>,
    /// Whether multiple children are allowed.
    pub multiple: bool,
    /// Whether the slot is required.
    pub required: bool,
}

/// Example configuration for documentation.
#[derive(Debug, Clone)]
pub struct Example {
    /// Example name.
    pub name: String,
    /// Configuration code.
    pub config: String,
    /// Description.
    pub description: Option<String>,
}

/// Factory trait for creating component instances.
pub trait ComponentFactory: Send + Sync + std::fmt::Debug {
    /// Creates a new component instance from configuration.
    fn create(&self, config: &Value) -> Result<Box<dyn std::any::Any>, ComponentError>;
}

/// Error creating a component.
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Missing required property: {0}")]
    MissingProperty(String),
    #[error("Component creation failed: {0}")]
    CreationFailed(String),
}

/// Descriptor for a data source.
#[derive(Debug, Clone)]
pub struct DataSourceDescriptor {
    /// Unique name of the data source.
    pub name: String,
    /// Version.
    pub version: Version,
    /// Source of this registration.
    pub source: DescriptorSource,
    /// Configuration schema.
    pub schema: ConfigSchema,
    /// Factory for creating instances.
    pub factory: Option<Arc<dyn DataSourceFactory>>,
    /// Metadata.
    pub metadata: DataSourceMetadata,
}

impl DataSourceDescriptor {
    /// Creates a new data source descriptor.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        DataSourceDescriptor {
            name: name.clone(),
            version: Version::new(0, 1, 0),
            source: DescriptorSource::BuiltIn,
            schema: ConfigSchema::new(name),
            factory: None,
            metadata: DataSourceMetadata::default(),
        }
    }

    /// Sets the schema.
    pub fn schema(mut self, schema: ConfigSchema) -> Self {
        self.schema = schema;
        self
    }

    /// Sets the factory.
    pub fn factory(mut self, factory: Arc<dyn DataSourceFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: DataSourceMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Metadata for a data source.
#[derive(Debug, Clone, Default)]
pub struct DataSourceMetadata {
    /// Display name.
    pub display_name: String,
    /// Description.
    pub description: String,
    /// Icon name.
    pub icon: Option<String>,
    /// Supports polling.
    pub supports_polling: bool,
    /// Supports streaming.
    pub supports_streaming: bool,
    /// Supports manual refresh.
    pub supports_manual_refresh: bool,
    /// Documentation URL.
    pub docs_url: Option<String>,
    /// Examples.
    pub examples: Vec<Example>,
}

/// Factory trait for creating data source instances.
pub trait DataSourceFactory: Send + Sync + std::fmt::Debug {
    /// Creates a new data source instance from configuration.
    fn create(&self, config: &Value) -> Result<Box<dyn std::any::Any>, DataSourceError>;
}

/// Error creating a data source.
#[derive(Debug, thiserror::Error)]
pub enum DataSourceError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Data source creation failed: {0}")]
    CreationFailed(String),
}

/// Descriptor for a transform.
#[derive(Debug, Clone)]
pub struct TransformDescriptor {
    /// Unique name.
    pub name: String,
    /// Version.
    pub version: Version,
    /// Source.
    pub source: DescriptorSource,
    /// Configuration schema.
    pub schema: ConfigSchema,
    /// Factory.
    pub factory: Option<Arc<dyn TransformFactory>>,
    /// Metadata.
    pub metadata: TransformMetadata,
}

impl TransformDescriptor {
    /// Creates a new transform descriptor.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        TransformDescriptor {
            name: name.clone(),
            version: Version::new(0, 1, 0),
            source: DescriptorSource::BuiltIn,
            schema: ConfigSchema::new(name),
            factory: None,
            metadata: TransformMetadata::default(),
        }
    }

    /// Sets the schema.
    pub fn schema(mut self, schema: ConfigSchema) -> Self {
        self.schema = schema;
        self
    }

    /// Sets the factory.
    pub fn factory(mut self, factory: Arc<dyn TransformFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: TransformMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Metadata for a transform.
#[derive(Debug, Clone, Default)]
pub struct TransformMetadata {
    /// Display name.
    pub display_name: String,
    /// Description.
    pub description: String,
    /// Preserves order.
    pub preserves_order: bool,
    /// May filter items.
    pub may_filter: bool,
    /// Maintains state.
    pub stateful: bool,
    /// Documentation URL.
    pub docs_url: Option<String>,
    /// Examples.
    pub examples: Vec<Example>,
}

/// Factory trait for creating transform instances.
pub trait TransformFactory: Send + Sync + std::fmt::Debug {
    /// Creates a new transform instance.
    fn create(&self, config: &Value) -> Result<Box<dyn std::any::Any>, TransformError>;
}

/// Error creating a transform.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Transform creation failed: {0}")]
    CreationFailed(String),
}

/// Descriptor for an action.
#[derive(Debug, Clone)]
pub struct ActionDescriptor {
    /// Unique name.
    pub name: String,
    /// Version.
    pub version: Version,
    /// Source.
    pub source: DescriptorSource,
    /// Configuration schema.
    pub schema: ConfigSchema,
    /// Factory.
    pub factory: Option<Arc<dyn ActionFactory>>,
    /// Metadata.
    pub metadata: ActionMetadata,
}

impl ActionDescriptor {
    /// Creates a new action descriptor.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        ActionDescriptor {
            name: name.clone(),
            version: Version::new(0, 1, 0),
            source: DescriptorSource::BuiltIn,
            schema: ConfigSchema::new(name),
            factory: None,
            metadata: ActionMetadata::default(),
        }
    }

    /// Sets the schema.
    pub fn schema(mut self, schema: ConfigSchema) -> Self {
        self.schema = schema;
        self
    }

    /// Sets the factory.
    pub fn factory(mut self, factory: Arc<dyn ActionFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: ActionMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Metadata for an action.
#[derive(Debug, Clone, Default)]
pub struct ActionMetadata {
    /// Display name.
    pub display_name: String,
    /// Description.
    pub description: String,
    /// Icon name.
    pub icon: Option<String>,
    /// Executes asynchronously.
    pub async_execution: bool,
    /// May fail.
    pub may_fail: bool,
    /// Idempotent.
    pub idempotent: bool,
    /// Documentation URL.
    pub docs_url: Option<String>,
    /// Examples.
    pub examples: Vec<Example>,
}

/// Factory trait for creating action instances.
pub trait ActionFactory: Send + Sync + std::fmt::Debug {
    /// Creates a new action instance.
    fn create(&self, config: &Value) -> Result<Box<dyn std::any::Any>, ActionError>;
}

/// Error creating an action.
#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Action creation failed: {0}")]
    CreationFailed(String),
}
