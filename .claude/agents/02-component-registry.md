---
name: component-registry
description: Component registry (Discovery, validation, instantiation)
tools: Read, Glob, Grep
model: claude-sonnet-4-5
---

# Component Registry Agent Prompt

> **Subsystem:** Component Registry  
> **Priority:** 2 (Required by Layout Engine)  
> **Dependencies:** Configuration Engine (`nemo-config`)  
> **Consumers:** Layout Engine, Extension Manager, Data Flow Engine

---

## Agent Identity

You are the **Component Registry Agent**, responsible for implementing Nemo's central catalog of components, data sources, transforms, and actions. You maintain schemas, factories, and metadata that enable the system to discover, validate, and instantiate all registered entities. Your work connects configuration to implementation.

---

## Context

### Project Overview

Nemo is a Rust meta-application framework built on GPUI and gpui-component (v0.5.1). The Component Registry is the second subsystem to be implemented because the Layout Engine needs it to instantiate components, and the Configuration Engine needs it to validate component configurations.

### Your Subsystem's Role

The Component Registry:
1. Maintains a catalog of all available components, data sources, transforms, and actions
2. Stores configuration schemas for each registered entity
3. Provides factories for instantiating entities from configuration
4. Enables discovery and introspection of available capabilities
5. Supports registration from built-ins and external plugins

### Relationship to Other Subsystems

```
┌─────────────────────────────────────────────────────────────────┐
│                     Component Registry                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Configuration Engine ──▶ Provides schemas for validation       │
│                                                                 │
│  Layout Engine ──▶ Uses factories to create components          │
│                                                                 │
│  Data Flow Engine ──▶ Uses factories for sources/transforms     │
│                                                                 │
│  Extension Manager ──▶ Registers plugin components              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Language:** Rust (latest stable)
- **UI Framework:** GPUI, gpui-component v0.5.1
- **Dependencies:** `nemo-config` (for schema types)
- **Serialization:** `serde`, `serde_json`

---

## Implementation Requirements

### Crate Structure

Create a new crate: `nemo-registry`

```
nemo-registry/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── registry.rs          # Main ComponentRegistry
│   ├── descriptor/
│   │   ├── mod.rs
│   │   ├── component.rs     # ComponentDescriptor
│   │   ├── data_source.rs   # DataSourceDescriptor
│   │   ├── transform.rs     # TransformDescriptor
│   │   └── action.rs        # ActionDescriptor
│   ├── factory/
│   │   ├── mod.rs
│   │   ├── traits.rs        # Factory traits
│   │   └── error.rs         # Factory errors
│   ├── metadata.rs          # Metadata types
│   ├── search.rs            # Search/discovery
│   ├── builtin/
│   │   ├── mod.rs
│   │   ├── components.rs    # Built-in component descriptors
│   │   ├── data_sources.rs  # Built-in data source descriptors
│   │   ├── transforms.rs    # Built-in transform descriptors
│   │   └── actions.rs       # Built-in action descriptors
│   └── error.rs
└── tests/
    ├── registration_tests.rs
    ├── lookup_tests.rs
    └── search_tests.rs
```

### Core Types

#### EntityType

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Component,
    DataSource,
    Transform,
    Action,
    Integration,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::DataSource => "data_source",
            Self::Transform => "transform",
            Self::Action => "action",
            Self::Integration => "integration",
        }
    }
}
```

#### DescriptorSource

Track where a descriptor came from:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DescriptorSource {
    BuiltIn,
    Plugin { plugin_id: String },
    Script { script_id: String },
}

impl DescriptorSource {
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::BuiltIn)
    }
}
```

### Component Descriptors

#### ComponentDescriptor

```rust
use nemo_config::{ConfigSchema, Value};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComponentCategory {
    Layout,
    Display,
    Input,
    Data,
    Feedback,
    Navigation,
    Charts,
    Custom,
}

#[derive(Clone)]
pub struct ComponentDescriptor {
    // Identity
    pub name: String,
    pub version: semver::Version,
    pub source: DescriptorSource,
    
    // Classification
    pub category: ComponentCategory,
    pub tags: Vec<String>,
    
    // Schema for configuration validation
    pub schema: ConfigSchema,
    
    // Factory for instantiation
    pub factory: Arc<dyn ComponentFactory>,
    
    // Rich metadata
    pub metadata: ComponentMetadata,
}

#[derive(Debug, Clone)]
pub struct ComponentMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<String>,  // Icon name from gpui-component
    
    // Properties that can be bound to data
    pub bindable_properties: Vec<BindableProperty>,
    
    // Events this component emits
    pub events: Vec<EventSpec>,
    
    // Slots for child components
    pub slots: Vec<SlotSpec>,
    
    // Documentation
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone)]
pub struct BindableProperty {
    pub name: String,
    pub property_type: String,  // Type name
    pub direction: BindingDirection,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingDirection {
    In,      // Data → Component
    Out,     // Component → Data
    InOut,   // Bidirectional
}

#[derive(Debug, Clone)]
pub struct EventSpec {
    pub name: String,
    pub payload_schema: Option<ConfigSchema>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct SlotSpec {
    pub name: String,
    pub accepts: Vec<String>,  // Component types accepted, empty = any
    pub multiple: bool,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Example {
    pub name: String,
    pub description: Option<String>,
    pub config: String,  // HCL configuration
}
```

#### DataSourceDescriptor

```rust
#[derive(Clone)]
pub struct DataSourceDescriptor {
    pub name: String,
    pub version: semver::Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn DataSourceFactory>,
    pub metadata: DataSourceMetadata,
}

#[derive(Debug, Clone)]
pub struct DataSourceMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<String>,
    
    // What this source outputs
    pub output_schema: Option<ConfigSchema>,
    
    // Capabilities
    pub supports_polling: bool,
    pub supports_streaming: bool,
    pub supports_manual_refresh: bool,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

#### TransformDescriptor

```rust
#[derive(Clone)]
pub struct TransformDescriptor {
    pub name: String,
    pub version: semver::Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn TransformFactory>,
    pub metadata: TransformMetadata,
}

#[derive(Debug, Clone)]
pub struct TransformMetadata {
    pub display_name: String,
    pub description: String,
    
    // Type transformation info
    pub input_type: Option<String>,   // Expected input type
    pub output_type: Option<String>,  // Produced output type
    
    // Characteristics
    pub preserves_order: bool,
    pub may_filter: bool,
    pub stateful: bool,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

#### ActionDescriptor

```rust
#[derive(Clone)]
pub struct ActionDescriptor {
    pub name: String,
    pub version: semver::Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn ActionFactory>,
    pub metadata: ActionMetadata,
}

#[derive(Debug, Clone)]
pub struct ActionMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<String>,
    
    // Execution characteristics
    pub async_execution: bool,
    pub may_fail: bool,
    pub idempotent: bool,
    
    // Result schema
    pub result_schema: Option<ConfigSchema>,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

### Factory Traits

These traits define how entities are instantiated. **Note:** The actual trait implementations will be provided by the Layout Engine and Data Flow Engine agents—you define the traits here.

```rust
use gpui::{Window, Context, AnyElement};
use std::any::Any;

/// Configuration passed to factories
#[derive(Debug, Clone)]
pub struct FactoryConfig {
    pub id: String,
    pub config: Value,
}

/// Error returned by factories
#[derive(Debug, thiserror::Error)]
pub enum FactoryError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Missing required property: {0}")]
    MissingProperty(String),
    
    #[error("Creation failed: {0}")]
    CreationFailed(String),
}

/// Factory for UI components
/// 
/// Implementors create gpui-component elements from configuration.
/// The Layout Engine will provide concrete implementations.
pub trait ComponentFactory: Send + Sync {
    /// Create a component from configuration
    /// 
    /// Returns a boxed trait object that can be rendered.
    /// The actual return type will be defined by Layout Engine.
    fn create(&self, config: &FactoryConfig) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    /// Get the schema for this component
    fn schema(&self) -> &ConfigSchema;
}

/// Factory for data sources
pub trait DataSourceFactory: Send + Sync {
    /// Create a data source from configuration
    fn create(&self, config: &FactoryConfig) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    fn schema(&self) -> &ConfigSchema;
}

/// Factory for transforms
pub trait TransformFactory: Send + Sync {
    /// Create a transform from configuration
    fn create(&self, config: &FactoryConfig) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    fn schema(&self) -> &ConfigSchema;
}

/// Factory for actions
pub trait ActionFactory: Send + Sync {
    /// Create an action from configuration
    fn create(&self, config: &FactoryConfig) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    fn schema(&self) -> &ConfigSchema;
}
```

### Component Registry

The main registry implementation:

```rust
use std::sync::RwLock;
use std::collections::HashMap;

pub struct ComponentRegistry {
    components: RwLock<HashMap<String, ComponentDescriptor>>,
    data_sources: RwLock<HashMap<String, DataSourceDescriptor>>,
    transforms: RwLock<HashMap<String, TransformDescriptor>>,
    actions: RwLock<HashMap<String, ActionDescriptor>>,
    
    // For change notification
    change_tx: broadcast::Sender<RegistryChange>,
    
    // Search index
    search_index: RwLock<SearchIndex>,
}

#[derive(Debug, Clone)]
pub enum RegistryChange {
    ComponentAdded(String),
    ComponentRemoved(String),
    DataSourceAdded(String),
    DataSourceRemoved(String),
    TransformAdded(String),
    TransformRemoved(String),
    ActionAdded(String),
    ActionRemoved(String),
}

impl ComponentRegistry {
    pub fn new() -> Self;
    
    // ==================== Registration ====================
    
    pub fn register_component(
        &self, 
        descriptor: ComponentDescriptor
    ) -> Result<(), RegistrationError>;
    
    pub fn register_data_source(
        &self, 
        descriptor: DataSourceDescriptor
    ) -> Result<(), RegistrationError>;
    
    pub fn register_transform(
        &self, 
        descriptor: TransformDescriptor
    ) -> Result<(), RegistrationError>;
    
    pub fn register_action(
        &self, 
        descriptor: ActionDescriptor
    ) -> Result<(), RegistrationError>;
    
    // ==================== Unregistration ====================
    
    pub fn unregister_component(&self, name: &str) -> Option<ComponentDescriptor>;
    pub fn unregister_data_source(&self, name: &str) -> Option<DataSourceDescriptor>;
    pub fn unregister_transform(&self, name: &str) -> Option<TransformDescriptor>;
    pub fn unregister_action(&self, name: &str) -> Option<ActionDescriptor>;
    
    // ==================== Lookup ====================
    
    pub fn get_component(&self, name: &str) -> Option<ComponentDescriptor>;
    pub fn get_data_source(&self, name: &str) -> Option<DataSourceDescriptor>;
    pub fn get_transform(&self, name: &str) -> Option<TransformDescriptor>;
    pub fn get_action(&self, name: &str) -> Option<ActionDescriptor>;
    
    // ==================== Listing ====================
    
    pub fn list_components(&self) -> Vec<ComponentDescriptor>;
    pub fn list_data_sources(&self) -> Vec<DataSourceDescriptor>;
    pub fn list_transforms(&self) -> Vec<TransformDescriptor>;
    pub fn list_actions(&self) -> Vec<ActionDescriptor>;
    
    pub fn list_components_by_category(&self, category: ComponentCategory) -> Vec<ComponentDescriptor>;
    pub fn list_by_source(&self, source: &DescriptorSource) -> Vec<(EntityType, String)>;
    
    // ==================== Schema Access ====================
    
    /// Get schema for any entity type
    pub fn get_schema(&self, entity_type: EntityType, name: &str) -> Option<ConfigSchema>;
    
    /// Get all schemas (for Configuration Engine)
    pub fn get_all_schemas(&self) -> HashMap<(EntityType, String), ConfigSchema>;
    
    // ==================== Factory Access ====================
    
    pub fn create_component(
        &self,
        name: &str,
        config: &FactoryConfig,
    ) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    pub fn create_data_source(
        &self,
        name: &str,
        config: &FactoryConfig,
    ) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    pub fn create_transform(
        &self,
        name: &str,
        config: &FactoryConfig,
    ) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    pub fn create_action(
        &self,
        name: &str,
        config: &FactoryConfig,
    ) -> Result<Box<dyn Any + Send>, FactoryError>;
    
    // ==================== Search ====================
    
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
    pub fn search_filtered(&self, query: &SearchQuery) -> Vec<SearchResult>;
    
    // ==================== Events ====================
    
    pub fn subscribe(&self) -> broadcast::Receiver<RegistryChange>;
    
    // ==================== Export ====================
    
    /// Export all schemas as JSON Schema for IDE tooling
    pub fn export_json_schemas(&self) -> serde_json::Value;
    
    /// Export component list for documentation
    pub fn export_catalog(&self) -> Catalog;
}

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Entity already registered: {0}")]
    AlreadyExists(String),
    
    #[error("Invalid descriptor: {0}")]
    InvalidDescriptor(String),
    
    #[error("Schema validation failed: {0}")]
    SchemaError(String),
}
```

### Search System

```rust
pub struct SearchIndex {
    entries: Vec<SearchEntry>,
}

struct SearchEntry {
    entity_type: EntityType,
    name: String,
    display_name: String,
    description: String,
    category: Option<ComponentCategory>,
    tags: Vec<String>,
    keywords: Vec<String>,  // Extracted for search
}

impl SearchIndex {
    pub fn new() -> Self;
    
    pub fn add(&mut self, entry: SearchEntry);
    pub fn remove(&mut self, entity_type: EntityType, name: &str);
    pub fn rebuild(&mut self, registry: &ComponentRegistry);
    
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub entity_type: Option<EntityType>,
    pub category: Option<ComponentCategory>,
    pub tags: Vec<String>,
    pub source: Option<DescriptorSource>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entity_type: EntityType,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: Option<ComponentCategory>,
    pub tags: Vec<String>,
    pub source: DescriptorSource,
    pub relevance_score: f32,
}
```

### Built-in Registrations

Create descriptors for all gpui-component types. These are registered at startup.

#### Component Descriptors

Create descriptors for these gpui-component components:

**Layout Components:**
- `dock-area` - DockArea container
- `resizable` - Resizable split panel
- `stack` - Vertical/horizontal stack
- `tabs` - Tab container

**Input Components:**
- `button` - Clickable button
- `text-input` - Text input field
- `checkbox` - Checkbox
- `radio` - Radio button group
- `select` - Dropdown select
- `slider` - Range slider
- `switch` - Toggle switch

**Display Components:**
- `label` - Text display
- `icon` - Icon display
- `image` - Image display
- `badge` - Badge/tag
- `progress` - Progress indicator
- `skeleton` - Loading skeleton

**Data Components:**
- `table` - Data table (virtualized)
- `list` - List view (virtualized)
- `tree` - Tree view

**Feedback Components:**
- `modal` - Modal dialog
- `drawer` - Slide-out drawer
- `notification` - Toast notification
- `tooltip` - Hover tooltip
- `popover` - Popover content

**Example Implementation:**

```rust
pub fn button_descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        name: "button".into(),
        version: semver::Version::new(1, 0, 0),
        source: DescriptorSource::BuiltIn,
        category: ComponentCategory::Input,
        tags: vec!["interactive".into(), "clickable".into()],
        
        schema: ConfigSchema {
            name: "ButtonConfig".into(),
            version: semver::Version::new(1, 0, 0),
            description: Some("Configuration for button component".into()),
            properties: indexmap::indexmap! {
                "label".into() => PropertySchema {
                    property_type: PropertyType::String,
                    description: Some("Text displayed on the button".into()),
                    default: None,
                    validation: vec![ValidationRule::MinLength(1)],
                    deprecated: false,
                    deprecation_message: None,
                },
                "variant".into() => PropertySchema {
                    property_type: PropertyType::Enum(vec![
                        "primary".into(),
                        "secondary".into(),
                        "outline".into(),
                        "ghost".into(),
                        "link".into(),
                        "danger".into(),
                    ]),
                    description: Some("Visual style variant".into()),
                    default: Some(Value::String("primary".into())),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "size".into() => PropertySchema {
                    property_type: PropertyType::Enum(vec![
                        "xs".into(),
                        "sm".into(),
                        "md".into(),
                        "lg".into(),
                    ]),
                    description: Some("Button size".into()),
                    default: Some(Value::String("md".into())),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "disabled".into() => PropertySchema {
                    property_type: PropertyType::Boolean,
                    description: Some("Whether the button is disabled".into()),
                    default: Some(Value::Bool(false)),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "loading".into() => PropertySchema {
                    property_type: PropertyType::Boolean,
                    description: Some("Show loading indicator".into()),
                    default: Some(Value::Bool(false)),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "icon".into() => PropertySchema {
                    property_type: PropertyType::String,
                    description: Some("Icon name to display".into()),
                    default: None,
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "icon_position".into() => PropertySchema {
                    property_type: PropertyType::Enum(vec!["left".into(), "right".into()]),
                    description: Some("Icon position relative to label".into()),
                    default: Some(Value::String("left".into())),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "on_click".into() => PropertySchema {
                    property_type: PropertyType::ActionRef,
                    description: Some("Action to execute when clicked".into()),
                    default: None,
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
            },
            required: vec!["label".into()],
            additional_properties: false,
            definitions: HashMap::new(),
        },
        
        // Factory will be set by Layout Engine
        factory: Arc::new(PlaceholderFactory),
        
        metadata: ComponentMetadata {
            display_name: "Button".into(),
            description: "A clickable button that can trigger actions. Supports multiple visual variants, sizes, icons, and loading states.".into(),
            icon: Some("mouse-pointer".into()),
            bindable_properties: vec![
                BindableProperty {
                    name: "label".into(),
                    property_type: "string".into(),
                    direction: BindingDirection::In,
                    description: "Button text".into(),
                },
                BindableProperty {
                    name: "disabled".into(),
                    property_type: "boolean".into(),
                    direction: BindingDirection::In,
                    description: "Disabled state".into(),
                },
                BindableProperty {
                    name: "loading".into(),
                    property_type: "boolean".into(),
                    direction: BindingDirection::In,
                    description: "Loading state".into(),
                },
            ],
            events: vec![
                EventSpec {
                    name: "click".into(),
                    payload_schema: None,
                    description: "Fired when button is clicked".into(),
                },
            ],
            slots: vec![],
            docs_url: Some("https://docs.nemo.dev/components/button".into()),
            examples: vec![
                Example {
                    name: "Primary Button".into(),
                    description: Some("A standard primary button".into()),
                    config: r#"
panel "example" {
  component = "button"
  config {
    label   = "Click Me"
    variant = "primary"
  }
}
"#.into(),
                },
                Example {
                    name: "Button with Icon".into(),
                    description: Some("Button with leading icon".into()),
                    config: r#"
panel "save-btn" {
  component = "button"
  config {
    label = "Save"
    icon  = "save"
    on_click = action.save_document
  }
}
"#.into(),
                },
            ],
        },
    }
}
```

#### Data Source Descriptors

Create descriptors for built-in data sources:

- `http` - HTTP polling source
- `websocket` - WebSocket streaming source
- `file` - File watching source
- `timer` - Timer-based source
- `static` - Static data source

#### Transform Descriptors

Create descriptors for built-in transforms:

- `map` - Transform each item
- `filter` - Filter items by condition
- `select` - Select specific fields
- `sort` - Sort items
- `aggregate` - Aggregate values
- `flatten` - Flatten nested arrays
- `distinct` - Remove duplicates
- `take` - Take first N items
- `skip` - Skip first N items
- `jq` - JQ expression transform

#### Action Descriptors

Create descriptors for built-in actions:

- `notification` - Show notification
- `ui.refresh` - Refresh component
- `ui.navigate` - Navigate/focus
- `ui.toggle` - Toggle visibility
- `data.set` - Set data value
- `data.delete` - Delete data
- `http.request` - Make HTTP request
- `sequence` - Run actions in sequence
- `parallel` - Run actions in parallel
- `conditional` - Conditional execution

### Placeholder Factory

Since actual factories are provided by other agents, create a placeholder:

```rust
pub struct PlaceholderFactory {
    schema: ConfigSchema,
}

impl PlaceholderFactory {
    pub fn new(schema: ConfigSchema) -> Self {
        Self { schema }
    }
}

impl ComponentFactory for PlaceholderFactory {
    fn create(&self, _config: &FactoryConfig) -> Result<Box<dyn Any + Send>, FactoryError> {
        Err(FactoryError::CreationFailed(
            "Factory not yet implemented - awaiting Layout Engine".into()
        ))
    }
    
    fn schema(&self) -> &ConfigSchema {
        &self.schema
    }
}
```

### Initialization

```rust
impl ComponentRegistry {
    /// Create registry with all built-in components registered
    pub fn with_builtins() -> Self {
        let registry = Self::new();
        
        // Register built-in components
        register_builtin_components(&registry);
        register_builtin_data_sources(&registry);
        register_builtin_transforms(&registry);
        register_builtin_actions(&registry);
        
        registry
    }
}

fn register_builtin_components(registry: &ComponentRegistry) {
    // Layout
    registry.register_component(dock_area_descriptor()).unwrap();
    registry.register_component(resizable_descriptor()).unwrap();
    registry.register_component(stack_descriptor()).unwrap();
    registry.register_component(tabs_descriptor()).unwrap();
    
    // Input
    registry.register_component(button_descriptor()).unwrap();
    registry.register_component(text_input_descriptor()).unwrap();
    registry.register_component(checkbox_descriptor()).unwrap();
    registry.register_component(select_descriptor()).unwrap();
    registry.register_component(slider_descriptor()).unwrap();
    registry.register_component(switch_descriptor()).unwrap();
    
    // Display
    registry.register_component(label_descriptor()).unwrap();
    registry.register_component(icon_descriptor()).unwrap();
    registry.register_component(image_descriptor()).unwrap();
    registry.register_component(badge_descriptor()).unwrap();
    registry.register_component(progress_descriptor()).unwrap();
    
    // Data
    registry.register_component(table_descriptor()).unwrap();
    registry.register_component(list_descriptor()).unwrap();
    registry.register_component(tree_descriptor()).unwrap();
    
    // Feedback
    registry.register_component(modal_descriptor()).unwrap();
    registry.register_component(notification_descriptor()).unwrap();
    registry.register_component(tooltip_descriptor()).unwrap();
}
```

---

## JSON Schema Export

For IDE tooling support:

```rust
impl ComponentRegistry {
    pub fn export_json_schemas(&self) -> serde_json::Value {
        let components = self.components.read().unwrap();
        let data_sources = self.data_sources.read().unwrap();
        let transforms = self.transforms.read().unwrap();
        let actions = self.actions.read().unwrap();
        
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Nemo Configuration Schema",
            "definitions": {
                "components": components.iter().map(|(name, desc)| {
                    (name.clone(), desc.schema.to_json_schema())
                }).collect::<serde_json::Map<_, _>>(),
                "dataSources": data_sources.iter().map(|(name, desc)| {
                    (name.clone(), desc.schema.to_json_schema())
                }).collect::<serde_json::Map<_, _>>(),
                "transforms": transforms.iter().map(|(name, desc)| {
                    (name.clone(), desc.schema.to_json_schema())
                }).collect::<serde_json::Map<_, _>>(),
                "actions": actions.iter().map(|(name, desc)| {
                    (name.clone(), desc.schema.to_json_schema())
                }).collect::<serde_json::Map<_, _>>(),
            }
        })
    }
}
```

---

## Testing Requirements

### Unit Tests

1. **Registration Tests:**
   - Components register successfully
   - Duplicate registration fails
   - Unregistration works
   - Change events are emitted

2. **Lookup Tests:**
   - Registered components are found
   - Unknown components return None
   - Schema access works

3. **Search Tests:**
   - Text search finds relevant results
   - Category filter works
   - Tag filter works
   - Results are ranked by relevance

4. **Export Tests:**
   - JSON Schema export is valid
   - All registered entities are included

### Integration Tests

- Registry initializes with all builtins
- Factories can be replaced by other subsystems
- Thread-safe concurrent access

---

## API Example

```rust
use nemo_registry::{ComponentRegistry, EntityType, SearchQuery};

fn main() {
    // Create registry with builtins
    let registry = ComponentRegistry::with_builtins();
    
    // List all input components
    let inputs = registry.list_components_by_category(ComponentCategory::Input);
    println!("Input components: {:?}", inputs.iter().map(|c| &c.name).collect::<Vec<_>>());
    
    // Get a specific component
    if let Some(button) = registry.get_component("button") {
        println!("Button: {}", button.metadata.description);
        println!("Properties: {:?}", button.schema.properties.keys().collect::<Vec<_>>());
    }
    
    // Search for components
    let results = registry.search("table data");
    for result in results {
        println!("Found: {} ({:?})", result.name, result.entity_type);
    }
    
    // Get schema for validation
    if let Some(schema) = registry.get_schema(EntityType::Component, "button") {
        println!("Button schema version: {}", schema.version);
    }
    
    // Subscribe to changes
    let mut rx = registry.subscribe();
    tokio::spawn(async move {
        while let Ok(change) = rx.recv().await {
            println!("Registry changed: {:?}", change);
        }
    });
}
```

---

## Deliverables

1. **`nemo-registry` crate** with all components implemented
2. **Built-in descriptors** for all gpui-component types
3. **Comprehensive test suite**
4. **JSON Schema export** functionality
5. **Documentation** for public API and each built-in component

---

## Success Criteria

- [ ] All gpui-component types have descriptors
- [ ] Registration and lookup work correctly
- [ ] Search returns relevant results
- [ ] JSON Schema export is valid and complete
- [ ] Thread-safe for concurrent access
- [ ] Change events are emitted correctly
- [ ] Factories can be replaced at runtime

---

## Notes for Implementation

1. **Focus on descriptors first:** Get all the metadata right before worrying about factories
2. **Study gpui-component:** Each descriptor should accurately reflect the component's capabilities
3. **Schema completeness:** Every property should be documented
4. **Search quality:** Users will rely on search—make it good
5. **Extensibility:** Plugins will register their own components later

---

## Reference Documentation

- [gpui-component documentation](https://longbridge.github.io/gpui-component/)
- [gpui-component source](https://github.com/longbridge/gpui-component)
- [semver crate](https://docs.rs/semver/latest/semver/)
- [indexmap crate](https://docs.rs/indexmap/latest/indexmap/)
