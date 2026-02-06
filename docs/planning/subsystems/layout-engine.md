# Layout Engine Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Layout Engine transforms declarative configuration into live GPUI component trees. It is the bridge between the static world of HCL configuration and the dynamic world of interactive UI. The engine handles component instantiation, hierarchical composition, layout management (including gpui-component's sophisticated dock system), and data binding.

## Responsibilities

1. **Component Instantiation:** Create GPUI components from configuration
2. **Hierarchy Construction:** Build parent-child component relationships
3. **Layout Management:** Handle docks, splits, tabs, and resizable panels
4. **Binding Resolution:** Connect components to data sources
5. **Dynamic Updates:** Respond to configuration and data changes
6. **State Management:** Coordinate component state with persistence

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Layout Engine                                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                        LayoutBuilder                                    │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────────┐   │ │
│  │  │ ConfigReader  │─▶│ComponentFactory│─▶│    HierarchyBuilder      │   │ │
│  │  └───────────────┘  └───────────────┘  └───────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                       LayoutManager                                     │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────────┐   │ │
│  │  │ DockController│  │BindingManager │  │   StateCoordinator        │   │ │
│  │  └───────────────┘  └───────────────┘  └───────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                     GPUI Component Tree                                 │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. LayoutBuilder

**Purpose:** Construct the initial component tree from configuration.

```rust
pub struct LayoutBuilder {
    component_registry: Arc<ComponentRegistry>,
    binding_resolver: Arc<BindingResolver>,
}

impl LayoutBuilder {
    /// Build a complete layout from resolved configuration
    pub fn build(
        &self,
        config: &LayoutConfig,
        window: &mut Window,
        cx: &mut Context<Root>,
    ) -> Result<Entity<DockArea>, LayoutError>;
    
    /// Build a single component by type and config
    pub fn build_component(
        &self,
        component_type: &str,
        config: &ComponentConfig,
        window: &mut Window,
        cx: &mut Context<impl Render>,
    ) -> Result<AnyElement, LayoutError>;
}
```

**Build Process:**
1. Parse layout configuration into layout tree representation
2. Resolve component references and validate types
3. Create component instances via ComponentFactory
4. Establish parent-child relationships
5. Apply initial bindings
6. Return root element for rendering

### 2. ComponentFactory

**Purpose:** Create component instances from type names and configuration.

```rust
pub trait ComponentFactory: Send + Sync {
    /// Create a component instance
    fn create(
        &self,
        config: &ComponentConfig,
        window: &mut Window,
        cx: &mut Context<impl Render>,
    ) -> Result<Box<dyn PanelView>, ComponentError>;
    
    /// Get the schema for this component type
    fn schema(&self) -> &ConfigSchema;
    
    /// Get component metadata
    fn metadata(&self) -> &ComponentMetadata;
}

pub struct ComponentMetadata {
    pub name: String,
    pub description: String,
    pub category: ComponentCategory,
    pub icon: Option<IconName>,
    pub bindable_properties: Vec<String>,
    pub emitted_events: Vec<String>,
}

pub enum ComponentCategory {
    Layout,      // Containers, splitters, tabs
    Display,     // Text, images, charts
    Input,       // Buttons, text fields, selects
    Data,        // Tables, lists, trees
    Custom,      // Plugin-provided
}
```

**Factory Registration:**
```rust
// Built-in components registered at startup
registry.register("button", ButtonFactory::new());
registry.register("text-input", TextInputFactory::new());
registry.register("data-table", DataTableFactory::new());
registry.register("dock-area", DockAreaFactory::new());

// Plugin components registered during extension loading
registry.register("custom-chart", plugin.get_factory("custom-chart"));
```

### 3. HierarchyBuilder

**Purpose:** Establish component relationships and nesting.

```rust
pub struct HierarchyBuilder;

impl HierarchyBuilder {
    /// Build a nested component structure
    pub fn build_tree(
        &self,
        node: &LayoutNode,
        factory: &ComponentFactory,
        window: &mut Window,
        cx: &mut Context<impl Render>,
    ) -> Result<ComponentTree, LayoutError>;
}

pub struct LayoutNode {
    pub component_type: String,
    pub id: Option<String>,
    pub config: ComponentConfig,
    pub children: Vec<LayoutNode>,
    pub layout_hints: LayoutHints,
}

pub struct LayoutHints {
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub flex: Option<f32>,
    pub alignment: Option<Alignment>,
}
```

### 4. DockController

**Purpose:** Manage gpui-component's DockArea for complex layouts.

```rust
pub struct DockController {
    dock_area: Entity<DockArea>,
    panel_registry: HashMap<String, Entity<dyn PanelView>>,
}

impl DockController {
    /// Set the center content
    pub fn set_center(&mut self, item: DockItem, window: &mut Window, cx: &mut Context<Self>);
    
    /// Add a panel to a dock position
    pub fn add_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        placement: DockPlacement,
        size: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );
    
    /// Toggle dock visibility
    pub fn toggle_dock(&mut self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>);
    
    /// Get panel by ID
    pub fn get_panel(&self, id: &str) -> Option<&Entity<dyn PanelView>>;
    
    /// Serialize current layout
    pub fn save_layout(&self, cx: &Context<Self>) -> DockLayoutState;
    
    /// Restore layout from state
    pub fn restore_layout(&mut self, state: DockLayoutState, window: &mut Window, cx: &mut Context<Self>);
}
```

**Dock Layout Configuration:**
```hcl
layout {
  type = "dock"
  
  center {
    type = "split"
    direction = "horizontal"
    
    item {
      size = 300
      panel "file-explorer" {
        component = "tree-view"
        # ...
      }
    }
    
    item {
      type = "tabs"
      
      panel "editor-1" {
        component = "code-editor"
        # ...
      }
      
      panel "editor-2" {
        component = "code-editor"
        # ...
      }
    }
  }
  
  left {
    open = true
    size = 250
    
    panel "outline" {
      component = "tree-view"
      # ...
    }
  }
  
  bottom {
    open = false
    size = 200
    
    panel "terminal" {
      component = "terminal"
      # ...
    }
  }
}
```

### 5. BindingManager

**Purpose:** Connect component properties to data sources.

```rust
pub struct BindingManager {
    bindings: Vec<ActiveBinding>,
    data_flow: Arc<DataFlowEngine>,
}

impl BindingManager {
    /// Create a binding between data and component
    pub fn bind(
        &mut self,
        source: DataPath,
        target: ComponentProperty,
        mode: BindingMode,
        transform: Option<Box<dyn Transform>>,
    ) -> BindingId;
    
    /// Remove a binding
    pub fn unbind(&mut self, id: BindingId);
    
    /// Process data updates (called by DataFlowEngine)
    pub fn on_data_changed(&mut self, path: &DataPath, value: &Value, cx: &mut Context<Self>);
}

pub struct ActiveBinding {
    pub id: BindingId,
    pub source: DataPath,
    pub target: ComponentProperty,
    pub mode: BindingMode,
    pub transform: Option<Box<dyn Transform>>,
    pub last_value: Option<Value>,
}

pub enum BindingMode {
    OneWay,       // Data → Component
    TwoWay,       // Data ↔ Component
    OneTime,      // Set once, don't update
    Computed,     // Derived from multiple sources
}

pub struct ComponentProperty {
    pub component_id: String,
    pub property_path: String,  // e.g., "config.columns" or "state.selected_row"
}
```

**Binding Configuration:**
```hcl
panel "data-display" {
  component = "data-table"
  
  bind {
    source = data.api_data.records
    target = "rows"
    mode   = "one-way"
  }
  
  bind {
    source = state.selected_id
    target = "selected_row"
    mode   = "two-way"
  }
  
  bind {
    source = data.api_data.loading
    target = "loading"
    transform = "boolean"
  }
}
```

### 6. StateCoordinator

**Purpose:** Manage component state across sessions.

```rust
pub struct StateCoordinator {
    states: HashMap<String, ComponentState>,
    persistence: Option<Box<dyn StatePersistence>>,
}

impl StateCoordinator {
    /// Get state for a component
    pub fn get_state(&self, component_id: &str) -> Option<&ComponentState>;
    
    /// Update component state
    pub fn set_state(&mut self, component_id: &str, state: ComponentState);
    
    /// Save all states to persistence
    pub fn persist(&self) -> Result<(), PersistError>;
    
    /// Restore states from persistence
    pub fn restore(&mut self) -> Result<(), PersistError>;
}

pub trait StatePersistence {
    fn save(&self, states: &HashMap<String, ComponentState>) -> Result<(), PersistError>;
    fn load(&self) -> Result<HashMap<String, ComponentState>, PersistError>;
}
```

---

## Layout Types

### 1. Dock Layout

Based on gpui-component's DockArea. Supports:
- **Center area:** Main content with splits and tabs
- **Side docks:** Left, right, bottom collapsible panels
- **Drag-and-drop:** Reorder panels between docks
- **Serialization:** Save/restore layout state

### 2. Stack Layout

Simple vertical or horizontal stacking:
```hcl
layout {
  type = "stack"
  direction = "vertical"
  gap = 8
  
  item {
    component = "header"
  }
  
  item {
    flex = 1
    component = "content"
  }
  
  item {
    component = "footer"
  }
}
```

### 3. Grid Layout

CSS Grid-like layout:
```hcl
layout {
  type = "grid"
  columns = "200px 1fr 1fr"
  rows = "auto 1fr auto"
  gap = 8
  
  item {
    column = "1 / 2"
    row = "1 / 4"
    component = "sidebar"
  }
  
  item {
    column = "2 / 4"
    row = "1"
    component = "header"
  }
  
  # ...
}
```

### 4. Tiles Layout

Free-form positioned tiles (like a desktop):
```hcl
layout {
  type = "tiles"
  
  tile {
    id = "tile-1"
    x = 100
    y = 100
    width = 400
    height = 300
    
    component = "chart"
  }
  
  tile {
    id = "tile-2"
    x = 520
    y = 100
    width = 300
    height = 200
    
    component = "stats"
  }
}
```

---

## Component Wrapping

Built-in gpui-component types are wrapped to add Nemo capabilities:

```rust
/// Wrapper that adds Nemo capabilities to any component
pub struct NemoComponent<T: Render> {
    inner: T,
    id: String,
    bindings: Vec<BindingId>,
    event_handlers: Vec<EventHandlerId>,
}

impl<T: Render> NemoComponent<T> {
    /// Wrap an existing component
    pub fn wrap(inner: T, id: String) -> Self;
    
    /// Add a data binding
    pub fn add_binding(&mut self, binding: BindingId);
    
    /// Add an event handler
    pub fn add_event_handler(&mut self, handler: EventHandlerId);
}

impl<T: Render> Render for NemoComponent<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Apply bindings, handle events, delegate to inner
        self.inner.render(window, cx)
    }
}
```

---

## gpui-component Integration

### Supported Components (Initial)

| Category | Components |
|----------|------------|
| **Layout** | DockArea, Resizable, Stack, Tabs |
| **Input** | Button, TextInput, Checkbox, Select, Slider |
| **Display** | Label, Icon, Image, Badge, Progress |
| **Data** | Table, List, Tree |
| **Feedback** | Modal, Notification, Tooltip |
| **Navigation** | Menu, Breadcrumb, Tabs |

### Component Schema Example

```rust
pub fn button_schema() -> ConfigSchema {
    ConfigSchema {
        name: "button".into(),
        version: Version::new(1, 0, 0),
        properties: hashmap! {
            "label".into() => PropertySchema {
                property_type: PropertyType::String,
                description: Some("Button text".into()),
                default: None,
                validation: None,
            },
            "variant".into() => PropertySchema {
                property_type: PropertyType::Enum(vec![
                    "primary".into(),
                    "secondary".into(),
                    "ghost".into(),
                    "danger".into(),
                ]),
                description: Some("Visual style".into()),
                default: Some(Value::String("primary".into())),
                validation: None,
            },
            "size".into() => PropertySchema {
                property_type: PropertyType::Enum(vec![
                    "xs".into(), "sm".into(), "md".into(), "lg".into()
                ]),
                description: Some("Button size".into()),
                default: Some(Value::String("md".into())),
                validation: None,
            },
            "disabled".into() => PropertySchema {
                property_type: PropertyType::Boolean,
                description: Some("Whether button is disabled".into()),
                default: Some(Value::Boolean(false)),
                validation: None,
            },
            "on_click".into() => PropertySchema {
                property_type: PropertyType::Reference("action".into()),
                description: Some("Action to execute on click".into()),
                default: None,
                validation: None,
            },
        },
        required: vec!["label".into()],
        additional_properties: false,
    }
}
```

---

## Hot Reload Support

When configuration changes:

1. **Diff Detection:** Identify what changed (new panels, removed panels, property changes)
2. **Targeted Updates:** Only rebuild affected components
3. **State Preservation:** Maintain component state through rebuilds where possible
4. **Binding Updates:** Rebind data sources to new component instances
5. **Animation:** Smooth transitions for layout changes

```rust
pub struct LayoutDiff {
    pub added: Vec<LayoutNode>,
    pub removed: Vec<String>,        // Component IDs
    pub modified: Vec<PropertyDiff>,
    pub moved: Vec<MoveDiff>,        // Position changes
}

impl LayoutManager {
    pub fn apply_diff(&mut self, diff: LayoutDiff, window: &mut Window, cx: &mut Context<Self>) {
        // Remove components
        for id in diff.removed {
            self.remove_component(&id, cx);
        }
        
        // Add new components
        for node in diff.added {
            self.add_component(node, window, cx);
        }
        
        // Update properties
        for prop_diff in diff.modified {
            self.update_property(prop_diff, cx);
        }
        
        // Handle moves
        for move_diff in diff.moved {
            self.move_component(move_diff, window, cx);
        }
    }
}
```

---

## Error Handling

### Error Types

```rust
pub enum LayoutError {
    /// Component type not found in registry
    UnknownComponent { type_name: String, location: SourceLocation },
    
    /// Invalid component configuration
    InvalidConfig { component_id: String, errors: Vec<ValidationError> },
    
    /// Binding target doesn't exist
    BindingError { binding_id: String, reason: String },
    
    /// Layout structure invalid
    InvalidStructure { reason: String, location: SourceLocation },
    
    /// Component failed to render
    RenderError { component_id: String, error: Box<dyn std::error::Error> },
}
```

### Error Recovery

- **Missing components:** Render placeholder with error message
- **Invalid config:** Use defaults where possible, warn user
- **Binding failures:** Display stale data with indicator
- **Render errors:** Isolate failing component, don't crash app

---

## Performance Considerations

1. **Lazy Rendering:** Only render visible components (virtualization)
2. **Memoization:** Cache component instances when possible
3. **Batched Updates:** Coalesce multiple binding updates
4. **Background Building:** Build complex layouts off main thread where possible

---

## Agent Prompt Considerations

When creating an agent to implement the Layout Engine:

- **GPUI knowledge required:** Agent needs deep familiarity with GPUI patterns
- **Component wrapping:** Must understand how to extend gpui-component
- **Binding complexity:** Data binding is subtle—consider edge cases
- **State management:** Coordinate with Data Flow Engine carefully
- **Testing:** Visual testing is challenging—consider snapshot testing

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
