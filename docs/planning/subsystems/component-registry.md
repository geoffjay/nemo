# Component Registry Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Component Registry is the central catalog of all available UI components, data sources, transforms, and actions. It maintains schemas for validation, factories for instantiation, and metadata for discovery. This subsystem enables Nemo's "construct any application using any component" capability by providing a consistent interface for registration and lookup.

## Responsibilities

1. **Registration:** Accept component registrations from built-ins and extensions
2. **Schema Storage:** Maintain configuration schemas for each component type
3. **Factory Management:** Provide factories for component instantiation
4. **Discovery:** Enable component listing, searching, and introspection
5. **Versioning:** Track component versions and compatibility
6. **Documentation:** Store and serve component documentation

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Component Registry                                   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Registration API                                   │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────────┐   │ │
│  │  │  Built-ins    │  │  Extensions   │  │      Plugins              │   │ │
│  │  └───────────────┘  └───────────────┘  └───────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Component Store                                    │ │
│  │  ┌─────────────────────────────────────────────────────────────────┐    │ │
│  │  │  Descriptors  │  Schemas  │  Factories  │  Metadata             │    │ │
│  │  └─────────────────────────────────────────────────────────────────┘    │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Query API                                          │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────────┐   │ │
│  │  │    Lookup     │  │    Search     │  │      Introspection        │   │ │
│  │  └───────────────┘  └───────────────┘  └───────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Registered Entity Types

| Type | Purpose | Examples |
|------|---------|----------|
| **Component** | UI elements | Button, Table, Chart, Panel |
| **DataSource** | Data collection | HTTP, WebSocket, File, Timer |
| **Transform** | Data processing | Map, Filter, Aggregate, JQ |
| **Action** | Operations | Notification, UI refresh, HTTP request |
| **Integration** | External protocols | JSON-RPC, MQTT, Redis |

---

## Core Components

### 1. ComponentRegistry

**Purpose:** Central registration and lookup service.

```rust
pub struct ComponentRegistry {
    components: HashMap<String, ComponentDescriptor>,
    data_sources: HashMap<String, DataSourceDescriptor>,
    transforms: HashMap<String, TransformDescriptor>,
    actions: HashMap<String, ActionDescriptor>,
    integrations: HashMap<String, IntegrationDescriptor>,
    
    // Index for search
    search_index: SearchIndex,
    
    // Event notification
    change_notifier: broadcast::Sender<RegistryChange>,
}

impl ComponentRegistry {
    // ---- Registration ----
    
    pub fn register_component(&mut self, descriptor: ComponentDescriptor) -> Result<(), RegistrationError>;
    pub fn register_data_source(&mut self, descriptor: DataSourceDescriptor) -> Result<(), RegistrationError>;
    pub fn register_transform(&mut self, descriptor: TransformDescriptor) -> Result<(), RegistrationError>;
    pub fn register_action(&mut self, descriptor: ActionDescriptor) -> Result<(), RegistrationError>;
    pub fn register_integration(&mut self, descriptor: IntegrationDescriptor) -> Result<(), RegistrationError>;
    
    // ---- Lookup ----
    
    pub fn get_component(&self, name: &str) -> Option<&ComponentDescriptor>;
    pub fn get_data_source(&self, name: &str) -> Option<&DataSourceDescriptor>;
    pub fn get_transform(&self, name: &str) -> Option<&TransformDescriptor>;
    pub fn get_action(&self, name: &str) -> Option<&ActionDescriptor>;
    pub fn get_integration(&self, name: &str) -> Option<&IntegrationDescriptor>;
    
    // ---- Discovery ----
    
    pub fn list_components(&self) -> Vec<&ComponentDescriptor>;
    pub fn list_by_category(&self, category: Category) -> Vec<&ComponentDescriptor>;
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
    
    // ---- Schema Access ----
    
    pub fn get_schema(&self, entity_type: EntityType, name: &str) -> Option<&ConfigSchema>;
    pub fn validate_config(&self, entity_type: EntityType, name: &str, config: &Value) -> ValidationResult;
    
    // ---- Factory Access ----
    
    pub fn create_component(&self, name: &str, config: &ComponentConfig, cx: &mut Context) -> Result<Box<dyn PanelView>, CreateError>;
    pub fn create_data_source(&self, name: &str, config: &DataSourceConfig) -> Result<Box<dyn DataSource>, CreateError>;
    pub fn create_transform(&self, name: &str, config: &TransformConfig) -> Result<Box<dyn Transform>, CreateError>;
    pub fn create_action(&self, name: &str, config: &ActionConfig) -> Result<Box<dyn Action>, CreateError>;
    
    // ---- Events ----
    
    pub fn subscribe(&self) -> broadcast::Receiver<RegistryChange>;
}

pub enum EntityType {
    Component,
    DataSource,
    Transform,
    Action,
    Integration,
}

pub enum RegistryChange {
    Added { entity_type: EntityType, name: String },
    Removed { entity_type: EntityType, name: String },
    Updated { entity_type: EntityType, name: String },
}
```

### 2. Descriptor Types

**Component Descriptor:**

```rust
pub struct ComponentDescriptor {
    // Identity
    pub name: String,
    pub version: Version,
    pub source: DescriptorSource,
    
    // Classification
    pub category: ComponentCategory,
    pub tags: Vec<String>,
    
    // Schema
    pub schema: ConfigSchema,
    
    // Factory
    pub factory: Arc<dyn ComponentFactory>,
    
    // Metadata
    pub metadata: ComponentMetadata,
}

pub struct ComponentMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<IconName>,
    
    // Bindable properties for data binding
    pub bindable_properties: Vec<BindableProperty>,
    
    // Events this component emits
    pub events: Vec<EventSpec>,
    
    // Slots for child components
    pub slots: Vec<SlotSpec>,
    
    // Documentation
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}

pub struct BindableProperty {
    pub name: String,
    pub property_type: PropertyType,
    pub direction: BindingDirection,  // In, Out, InOut
    pub description: String,
}

pub struct EventSpec {
    pub name: String,
    pub payload_schema: Option<ConfigSchema>,
    pub description: String,
}

pub struct SlotSpec {
    pub name: String,
    pub accepts: Vec<String>,  // Component types accepted
    pub multiple: bool,
    pub required: bool,
}

pub enum ComponentCategory {
    Layout,      // Containers, splitters
    Display,     // Text, images, indicators
    Input,       // Buttons, text fields
    Data,        // Tables, lists, trees
    Feedback,    // Modals, notifications
    Navigation,  // Menus, tabs, breadcrumbs
    Charts,      // Visualizations
    Custom,      // Plugin-provided
}

pub enum DescriptorSource {
    BuiltIn,
    Plugin { plugin_id: String },
    Script { script_id: String },
}
```

**Data Source Descriptor:**

```rust
pub struct DataSourceDescriptor {
    pub name: String,
    pub version: Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn DataSourceFactory>,
    
    pub metadata: DataSourceMetadata,
}

pub struct DataSourceMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<IconName>,
    
    // Output schema
    pub output_schema: Option<DataSchema>,
    
    // Capabilities
    pub supports_polling: bool,
    pub supports_streaming: bool,
    pub supports_manual_refresh: bool,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

**Transform Descriptor:**

```rust
pub struct TransformDescriptor {
    pub name: String,
    pub version: Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn TransformFactory>,
    
    pub metadata: TransformMetadata,
}

pub struct TransformMetadata {
    pub display_name: String,
    pub description: String,
    
    // Type transformation
    pub input_type: Option<DataSchema>,
    pub output_type: Option<DataSchema>,
    
    // Characteristics
    pub preserves_order: bool,
    pub may_filter: bool,
    pub stateful: bool,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

**Action Descriptor:**

```rust
pub struct ActionDescriptor {
    pub name: String,
    pub version: Version,
    pub source: DescriptorSource,
    
    pub schema: ConfigSchema,
    pub factory: Arc<dyn ActionFactory>,
    
    pub metadata: ActionMetadata,
}

pub struct ActionMetadata {
    pub display_name: String,
    pub description: String,
    pub icon: Option<IconName>,
    
    // Execution characteristics
    pub async_execution: bool,
    pub may_fail: bool,
    pub idempotent: bool,
    
    // Result schema
    pub result_schema: Option<DataSchema>,
    
    pub docs_url: Option<String>,
    pub examples: Vec<Example>,
}
```

### 3. Factory Traits

```rust
pub trait ComponentFactory: Send + Sync {
    fn create(
        &self,
        config: &ComponentConfig,
        window: &mut Window,
        cx: &mut Context<impl Render>,
    ) -> Result<Box<dyn PanelView>, ComponentError>;
}

pub trait DataSourceFactory: Send + Sync {
    fn create(&self, config: &DataSourceConfig) -> Result<Box<dyn DataSource>, DataSourceError>;
}

pub trait TransformFactory: Send + Sync {
    fn create(&self, config: &TransformConfig) -> Result<Box<dyn Transform>, TransformError>;
}

pub trait ActionFactory: Send + Sync {
    fn create(&self, config: &ActionConfig) -> Result<Box<dyn Action>, ActionError>;
}
```

### 4. ConfigSchema

**Purpose:** Define the configuration structure for any registered entity.

```rust
pub struct ConfigSchema {
    pub name: String,
    pub version: Version,
    pub properties: IndexMap<String, PropertySchema>,
    pub required: Vec<String>,
    pub additional_properties: bool,
    
    // For complex schemas
    pub definitions: HashMap<String, ConfigSchema>,
}

pub struct PropertySchema {
    pub property_type: PropertyType,
    pub description: Option<String>,
    pub default: Option<Value>,
    pub validation: Vec<ValidationRule>,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
}

pub enum PropertyType {
    // Primitives
    String,
    Integer,
    Float,
    Boolean,
    
    // Complex types
    Array { items: Box<PropertyType> },
    Object { schema: Box<ConfigSchema> },
    Map { keys: Box<PropertyType>, values: Box<PropertyType> },
    
    // Type union
    OneOf(Vec<PropertyType>),
    
    // Constrained types
    Enum(Vec<String>),
    Const(Value),
    
    // References
    Ref(String),  // Reference to $definitions
    ComponentRef,  // Reference to component ID
    DataRef,       // Reference to data path
    ActionRef,     // Reference to action name
}

pub enum ValidationRule {
    // String validations
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Format(StringFormat),
    
    // Numeric validations
    Minimum(f64),
    Maximum(f64),
    ExclusiveMinimum(f64),
    ExclusiveMaximum(f64),
    MultipleOf(f64),
    
    // Array validations
    MinItems(usize),
    MaxItems(usize),
    UniqueItems,
    
    // Custom
    Custom { name: String, params: Value },
}

pub enum StringFormat {
    DateTime,
    Date,
    Time,
    Duration,
    Email,
    Uri,
    Regex,
    Color,
    Path,
}
```

---

## Built-in Registration

### Component Registration at Startup

```rust
pub fn register_builtin_components(registry: &mut ComponentRegistry) {
    // Layout components
    registry.register_component(dock_area_descriptor()).unwrap();
    registry.register_component(resizable_descriptor()).unwrap();
    registry.register_component(stack_descriptor()).unwrap();
    registry.register_component(tabs_descriptor()).unwrap();
    
    // Input components
    registry.register_component(button_descriptor()).unwrap();
    registry.register_component(text_input_descriptor()).unwrap();
    registry.register_component(checkbox_descriptor()).unwrap();
    registry.register_component(select_descriptor()).unwrap();
    registry.register_component(slider_descriptor()).unwrap();
    
    // Display components
    registry.register_component(label_descriptor()).unwrap();
    registry.register_component(icon_descriptor()).unwrap();
    registry.register_component(image_descriptor()).unwrap();
    registry.register_component(badge_descriptor()).unwrap();
    registry.register_component(progress_descriptor()).unwrap();
    
    // Data components
    registry.register_component(table_descriptor()).unwrap();
    registry.register_component(list_descriptor()).unwrap();
    registry.register_component(tree_descriptor()).unwrap();
    
    // Feedback components
    registry.register_component(modal_descriptor()).unwrap();
    registry.register_component(notification_descriptor()).unwrap();
    registry.register_component(tooltip_descriptor()).unwrap();
}

pub fn register_builtin_data_sources(registry: &mut ComponentRegistry) {
    registry.register_data_source(http_source_descriptor()).unwrap();
    registry.register_data_source(websocket_source_descriptor()).unwrap();
    registry.register_data_source(file_source_descriptor()).unwrap();
    registry.register_data_source(timer_source_descriptor()).unwrap();
}

pub fn register_builtin_transforms(registry: &mut ComponentRegistry) {
    registry.register_transform(map_transform_descriptor()).unwrap();
    registry.register_transform(filter_transform_descriptor()).unwrap();
    registry.register_transform(select_transform_descriptor()).unwrap();
    registry.register_transform(aggregate_transform_descriptor()).unwrap();
    registry.register_transform(sort_transform_descriptor()).unwrap();
    registry.register_transform(jq_transform_descriptor()).unwrap();
}

pub fn register_builtin_actions(registry: &mut ComponentRegistry) {
    registry.register_action(notification_action_descriptor()).unwrap();
    registry.register_action(ui_refresh_action_descriptor()).unwrap();
    registry.register_action(ui_navigate_action_descriptor()).unwrap();
    registry.register_action(data_set_action_descriptor()).unwrap();
    registry.register_action(http_request_action_descriptor()).unwrap();
    registry.register_action(sequence_action_descriptor()).unwrap();
}
```

### Example Descriptor Definition

```rust
fn button_descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        name: "button".into(),
        version: Version::new(1, 0, 0),
        source: DescriptorSource::BuiltIn,
        
        category: ComponentCategory::Input,
        tags: vec!["interactive".into(), "clickable".into()],
        
        schema: ConfigSchema {
            name: "ButtonConfig".into(),
            version: Version::new(1, 0, 0),
            properties: indexmap! {
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
                        "ghost".into(),
                        "danger".into(),
                    ]),
                    description: Some("Visual style of the button".into()),
                    default: Some(Value::String("primary".into())),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "size".into() => PropertySchema {
                    property_type: PropertyType::Enum(vec![
                        "xs".into(), "sm".into(), "md".into(), "lg".into(),
                    ]),
                    description: Some("Size of the button".into()),
                    default: Some(Value::String("md".into())),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "disabled".into() => PropertySchema {
                    property_type: PropertyType::Boolean,
                    description: Some("Whether the button is disabled".into()),
                    default: Some(Value::Boolean(false)),
                    validation: vec![],
                    deprecated: false,
                    deprecation_message: None,
                },
                "icon".into() => PropertySchema {
                    property_type: PropertyType::String,
                    description: Some("Optional icon name".into()),
                    default: None,
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
        
        factory: Arc::new(ButtonFactory),
        
        metadata: ComponentMetadata {
            display_name: "Button".into(),
            description: "A clickable button that triggers actions".into(),
            icon: Some(IconName::MousePointer),
            bindable_properties: vec![
                BindableProperty {
                    name: "label".into(),
                    property_type: PropertyType::String,
                    direction: BindingDirection::In,
                    description: "Button text".into(),
                },
                BindableProperty {
                    name: "disabled".into(),
                    property_type: PropertyType::Boolean,
                    direction: BindingDirection::In,
                    description: "Disabled state".into(),
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
                    name: "Basic button".into(),
                    config: r#"
                        button "submit" {
                          label = "Submit"
                          variant = "primary"
                          on_click = action.submit_form
                        }
                    "#.into(),
                },
            ],
        },
    }
}
```

---

## Schema Export

### JSON Schema Generation

For IDE tooling and external validation:

```rust
impl ConfigSchema {
    /// Export as JSON Schema
    pub fn to_json_schema(&self) -> serde_json::Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": self.name,
            "type": "object",
            "properties": self.properties.iter().map(|(k, v)| {
                (k.clone(), v.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
            "required": self.required,
            "additionalProperties": self.additional_properties,
            "$defs": self.definitions.iter().map(|(k, v)| {
                (k.clone(), v.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
        })
    }
}

impl ComponentRegistry {
    /// Export all schemas as JSON Schema
    pub fn export_schemas(&self) -> serde_json::Value {
        json!({
            "components": self.components.iter().map(|(k, v)| {
                (k.clone(), v.schema.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
            "dataSources": self.data_sources.iter().map(|(k, v)| {
                (k.clone(), v.schema.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
            "transforms": self.transforms.iter().map(|(k, v)| {
                (k.clone(), v.schema.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
            "actions": self.actions.iter().map(|(k, v)| {
                (k.clone(), v.schema.to_json_schema())
            }).collect::<serde_json::Map<_, _>>(),
        })
    }
}
```

### HCL Schema Hints

For HCL configuration assistance:

```rust
impl ComponentRegistry {
    /// Generate HCL language server hints
    pub fn generate_hcl_hints(&self) -> HclHints {
        HclHints {
            block_types: vec![
                BlockHint {
                    name: "panel",
                    labels: vec!["id"],
                    attributes: self.components.keys().cloned().collect(),
                },
                BlockHint {
                    name: "data",
                    labels: vec!["type", "id"],
                    attributes: self.data_sources.keys().cloned().collect(),
                },
                // ...
            ],
            // ...
        }
    }
}
```

---

## Discovery API

### Search Capabilities

```rust
impl ComponentRegistry {
    /// Full-text search across all entities
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.search_index.search(query)
    }
    
    /// Search with filters
    pub fn search_filtered(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let mut results = self.search_index.search(&query.text);
        
        if let Some(entity_type) = &query.entity_type {
            results.retain(|r| &r.entity_type == entity_type);
        }
        
        if let Some(category) = &query.category {
            results.retain(|r| r.category.as_ref() == Some(category));
        }
        
        if !query.tags.is_empty() {
            results.retain(|r| {
                r.tags.iter().any(|t| query.tags.contains(t))
            });
        }
        
        results
    }
}

pub struct SearchQuery {
    pub text: String,
    pub entity_type: Option<EntityType>,
    pub category: Option<ComponentCategory>,
    pub tags: Vec<String>,
    pub source: Option<DescriptorSource>,
}

pub struct SearchResult {
    pub entity_type: EntityType,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: Option<ComponentCategory>,
    pub tags: Vec<String>,
    pub relevance_score: f32,
}
```

---

## Versioning

### Version Compatibility

```rust
pub struct VersionConstraint {
    pub operator: VersionOperator,
    pub version: Version,
}

pub enum VersionOperator {
    Exact,       // = 1.0.0
    GreaterThan, // > 1.0.0
    LessThan,    // < 1.0.0
    Gte,         // >= 1.0.0
    Lte,         // <= 1.0.0
    Compatible,  // ^1.0.0 (semver compatible)
}

impl ComponentRegistry {
    /// Get specific version of component
    pub fn get_component_version(&self, name: &str, constraint: &VersionConstraint) -> Option<&ComponentDescriptor>;
    
    /// Check if version satisfies constraint
    pub fn check_version(&self, version: &Version, constraint: &VersionConstraint) -> bool;
}
```

---

## Agent Prompt Considerations

When creating an agent to implement the Component Registry:

- **Type safety:** Schemas are critical for validation—ensure completeness
- **Performance:** Registry is queried frequently—optimize lookups
- **Documentation:** Metadata should be useful for IDE tooling
- **Versioning:** Plan for schema evolution from the start
- **Testing:** Validate all built-in descriptors are complete
- **Extensibility:** Plugin registration should be seamless

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
