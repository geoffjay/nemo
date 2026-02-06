//! Layout node types.

use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the layout tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutNode {
    /// Component type name.
    pub component_type: String,
    /// Optional component ID.
    pub id: Option<String>,
    /// Component configuration.
    pub config: ComponentConfig,
    /// Child nodes.
    pub children: Vec<LayoutNode>,
    /// Layout hints.
    pub layout_hints: LayoutHints,
    /// Event handlers.
    pub handlers: HashMap<String, String>,
}

impl LayoutNode {
    /// Creates a new layout node.
    pub fn new(component_type: impl Into<String>) -> Self {
        Self {
            component_type: component_type.into(),
            id: None,
            config: ComponentConfig::default(),
            children: Vec::new(),
            layout_hints: LayoutHints::default(),
            handlers: HashMap::new(),
        }
    }

    /// Sets the component ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets a config property.
    pub fn with_prop(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.properties.insert(key.into(), value);
        self
    }

    /// Adds a child node.
    pub fn with_child(mut self, child: LayoutNode) -> Self {
        self.children.push(child);
        self
    }

    /// Sets layout hints.
    pub fn with_hints(mut self, hints: LayoutHints) -> Self {
        self.layout_hints = hints;
        self
    }

    /// Adds an event handler.
    pub fn with_handler(mut self, event: impl Into<String>, action: impl Into<String>) -> Self {
        self.handlers.insert(event.into(), action.into());
        self
    }

    /// Returns the effective ID (generated if not set).
    pub fn effective_id(&self) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| format!("{}_{}", self.component_type, uuid_simple()))
    }
}

/// Simple UUID-like generator for IDs.
fn uuid_simple() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("{:08x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Component configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// Configuration properties.
    pub properties: HashMap<String, Value>,
    /// Data bindings.
    pub bindings: Vec<BindingSpec>,
}

impl ComponentConfig {
    /// Creates a new empty config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a property.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.properties.insert(key.into(), value);
    }

    /// Gets a property.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    /// Adds a binding.
    pub fn add_binding(&mut self, binding: BindingSpec) {
        self.bindings.push(binding);
    }
}

/// Specification for a data binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingSpec {
    /// Source data path.
    pub source: String,
    /// Target property.
    pub target: String,
    /// Binding mode.
    pub mode: BindingMode,
    /// Optional transform expression.
    pub transform: Option<String>,
}

impl BindingSpec {
    /// Creates a new one-way binding.
    pub fn one_way(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            mode: BindingMode::OneWay,
            transform: None,
        }
    }

    /// Creates a new two-way binding.
    pub fn two_way(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            mode: BindingMode::TwoWay,
            transform: None,
        }
    }

    /// Sets the transform expression.
    pub fn with_transform(mut self, transform: impl Into<String>) -> Self {
        self.transform = Some(transform.into());
        self
    }
}

/// Binding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingMode {
    /// Data flows from source to target only.
    OneWay,
    /// Data flows both directions.
    TwoWay,
    /// Data is set once at initialization.
    OneTime,
}

impl Default for BindingMode {
    fn default() -> Self {
        Self::OneWay
    }
}

/// Size specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl Size {
    /// Creates a new size.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Creates a square size.
    pub fn square(size: f32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}

/// Alignment specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    /// Align to start.
    Start,
    /// Align to center.
    Center,
    /// Align to end.
    End,
    /// Stretch to fill.
    Stretch,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Start
    }
}

/// Layout hints for a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutHints {
    /// Preferred size.
    pub size: Option<Size>,
    /// Minimum size.
    pub min_size: Option<Size>,
    /// Maximum size.
    pub max_size: Option<Size>,
    /// Flex grow factor.
    pub flex: Option<f32>,
    /// Horizontal alignment.
    pub h_align: Option<Alignment>,
    /// Vertical alignment.
    pub v_align: Option<Alignment>,
    /// Padding in pixels.
    pub padding: Option<f32>,
    /// Margin in pixels.
    pub margin: Option<f32>,
}

impl LayoutHints {
    /// Creates new empty hints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the preferred size.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = Some(Size::new(width, height));
        self
    }

    /// Sets the flex factor.
    pub fn with_flex(mut self, flex: f32) -> Self {
        self.flex = Some(flex);
        self
    }

    /// Sets the alignment.
    pub fn with_alignment(mut self, h: Alignment, v: Alignment) -> Self {
        self.h_align = Some(h);
        self.v_align = Some(v);
        self
    }
}

/// Layout type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutType {
    /// Dock-based layout with panels.
    Dock,
    /// Vertical or horizontal stack.
    Stack,
    /// CSS Grid-like layout.
    Grid,
    /// Free-form tiles.
    Tiles,
}

impl Default for LayoutType {
    fn default() -> Self {
        Self::Stack
    }
}

/// Root layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Layout type.
    pub layout_type: LayoutType,
    /// Root node.
    pub root: LayoutNode,
    /// Variables.
    pub variables: HashMap<String, Value>,
}

impl LayoutConfig {
    /// Creates a new layout config.
    pub fn new(layout_type: LayoutType, root: LayoutNode) -> Self {
        Self {
            layout_type,
            root,
            variables: HashMap::new(),
        }
    }

    /// Adds a variable.
    pub fn with_variable(mut self, name: impl Into<String>, value: Value) -> Self {
        self.variables.insert(name.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_node_creation() {
        let node = LayoutNode::new("button")
            .with_id("btn1")
            .with_prop("label", Value::String("Click me".into()));

        assert_eq!(node.component_type, "button");
        assert_eq!(node.id, Some("btn1".to_string()));
        assert!(node.config.properties.contains_key("label"));
    }

    #[test]
    fn test_layout_node_children() {
        let parent = LayoutNode::new("stack")
            .with_child(LayoutNode::new("button"))
            .with_child(LayoutNode::new("label"));

        assert_eq!(parent.children.len(), 2);
    }

    #[test]
    fn test_binding_spec() {
        let binding = BindingSpec::one_way("data.users", "items")
            .with_transform("item => item.name");

        assert_eq!(binding.source, "data.users");
        assert_eq!(binding.target, "items");
        assert!(binding.transform.is_some());
    }

    #[test]
    fn test_layout_hints() {
        let hints = LayoutHints::new()
            .with_size(100.0, 50.0)
            .with_flex(1.0);

        assert_eq!(hints.size, Some(Size::new(100.0, 50.0)));
        assert_eq!(hints.flex, Some(1.0));
    }
}
