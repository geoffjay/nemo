//! Layout builder for constructing component trees from configuration.

use crate::error::LayoutError;
use crate::node::{LayoutConfig, LayoutNode, LayoutType};
use nemo_config::Value;
use nemo_registry::ComponentRegistry;
use std::collections::HashMap;
use std::sync::Arc;

/// Result of building a component.
#[derive(Debug)]
pub struct BuildResult {
    /// Component ID.
    pub id: String,
    /// Component type.
    pub component_type: String,
    /// Resolved properties.
    pub properties: HashMap<String, Value>,
    /// Event handlers (event name -> handler string).
    pub handlers: HashMap<String, String>,
    /// Child build results.
    pub children: Vec<BuildResult>,
}

/// Builder for constructing layouts from configuration.
pub struct LayoutBuilder {
    /// Component registry for looking up component definitions.
    registry: Arc<ComponentRegistry>,
}

impl LayoutBuilder {
    /// Creates a new layout builder.
    pub fn new(registry: Arc<ComponentRegistry>) -> Self {
        Self { registry }
    }

    /// Builds a complete layout from configuration.
    pub fn build(&self, config: &LayoutConfig) -> Result<BuildResult, LayoutError> {
        // Build the root node
        self.build_node(&config.root, &config.variables)
    }

    /// Builds a single node and its children.
    pub fn build_node(
        &self,
        node: &LayoutNode,
        variables: &HashMap<String, Value>,
    ) -> Result<BuildResult, LayoutError> {
        // Verify the component type exists
        if !self.registry.has_component(&node.component_type) {
            return Err(LayoutError::UnknownComponent {
                type_name: node.component_type.clone(),
            });
        }

        let id = node.effective_id();

        // Get the component descriptor to validate config
        let descriptor = self
            .registry
            .get_component(&node.component_type)
            .ok_or_else(|| LayoutError::UnknownComponent {
                type_name: node.component_type.clone(),
            })?;

        // Resolve properties with variable substitution
        let mut resolved_props = HashMap::new();
        for (key, value) in &node.config.properties {
            let resolved = self.resolve_value(value, variables)?;
            resolved_props.insert(key.clone(), resolved);
        }

        // Validate required properties
        for required in &descriptor.schema.required {
            if !resolved_props.contains_key(required) {
                return Err(LayoutError::MissingProperty {
                    component_id: id.clone(),
                    property: required.clone(),
                });
            }
        }

        // Build children recursively
        let mut children = Vec::new();
        for child_node in &node.children {
            let child_result = self.build_node(child_node, variables)?;
            children.push(child_result);
        }

        Ok(BuildResult {
            id,
            component_type: node.component_type.clone(),
            properties: resolved_props,
            handlers: node.handlers.clone(),
            children,
        })
    }

    /// Resolves variable references in a value.
    fn resolve_value(
        &self,
        value: &Value,
        variables: &HashMap<String, Value>,
    ) -> Result<Value, LayoutError> {
        match value {
            Value::String(s) => {
                // Check for variable reference: ${var.name}
                if s.starts_with("${var.") && s.ends_with('}') {
                    let var_name = &s[6..s.len() - 1];
                    if let Some(var_value) = variables.get(var_name) {
                        return Ok(var_value.clone());
                    }
                }
                Ok(value.clone())
            }
            Value::Array(arr) => {
                let resolved: Result<Vec<Value>, LayoutError> = arr
                    .iter()
                    .map(|v| self.resolve_value(v, variables))
                    .collect();
                Ok(Value::Array(resolved?))
            }
            Value::Object(obj) => {
                let mut resolved = indexmap::IndexMap::new();
                for (k, v) in obj {
                    resolved.insert(k.clone(), self.resolve_value(v, variables)?);
                }
                Ok(Value::Object(resolved))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Creates a builder for a specific layout type.
    pub fn for_layout_type(&self, layout_type: &LayoutType) -> LayoutTypeBuilder {
        LayoutTypeBuilder {
            layout_type: layout_type.clone(),
            registry: self.registry.clone(),
        }
    }
}

/// Builder specialized for a layout type.
pub struct LayoutTypeBuilder {
    layout_type: LayoutType,
    registry: Arc<ComponentRegistry>,
}

impl LayoutTypeBuilder {
    /// Gets the layout type.
    pub fn layout_type(&self) -> &LayoutType {
        &self.layout_type
    }

    /// Checks if this builder supports a component type.
    pub fn supports_component(&self, component_type: &str) -> bool {
        self.registry.has_component(component_type)
    }

    /// Validates that a node is valid for this layout type.
    pub fn validate_node(&self, node: &LayoutNode) -> Result<(), LayoutError> {
        // Verify the component exists
        if !self.registry.has_component(&node.component_type) {
            return Err(LayoutError::UnknownComponent {
                type_name: node.component_type.clone(),
            });
        }

        // Layout-specific validation
        match self.layout_type {
            LayoutType::Dock => {
                // Dock layouts have specific requirements
                // For now, just basic validation
            }
            LayoutType::Stack => {
                // Stack layouts are flexible
            }
            LayoutType::Grid => {
                // Grid layouts need column/row info
            }
            LayoutType::Tiles => {
                // Tiles need position info
            }
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_node(child)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nemo_registry::{register_all_builtins, ComponentRegistry};

    fn setup_registry() -> Arc<ComponentRegistry> {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        Arc::new(registry)
    }

    #[test]
    fn test_builder_creation() {
        let registry = setup_registry();
        let builder = LayoutBuilder::new(registry);
        assert!(builder.registry.has_component("button"));
    }

    #[test]
    fn test_build_simple_node() {
        let registry = setup_registry();
        let builder = LayoutBuilder::new(registry);

        let node = LayoutNode::new("button")
            .with_id("btn1")
            .with_prop("label", Value::String("Click".into()));

        let result = builder.build_node(&node, &HashMap::new());
        assert!(result.is_ok());

        let build = result.unwrap();
        assert_eq!(build.id, "btn1");
        assert_eq!(build.component_type, "button");
    }

    #[test]
    fn test_build_unknown_component() {
        let registry = setup_registry();
        let builder = LayoutBuilder::new(registry);

        let node = LayoutNode::new("unknown_component");
        let result = builder.build_node(&node, &HashMap::new());

        assert!(matches!(result, Err(LayoutError::UnknownComponent { .. })));
    }

    #[test]
    fn test_variable_resolution() {
        let registry = setup_registry();
        let builder = LayoutBuilder::new(registry);

        let node = LayoutNode::new("button")
            .with_prop("label", Value::String("${var.button_text}".into()));

        let mut variables = HashMap::new();
        variables.insert("button_text".to_string(), Value::String("Resolved!".into()));

        let result = builder.build_node(&node, &variables);
        assert!(result.is_ok());

        let build = result.unwrap();
        assert_eq!(
            build.properties.get("label"),
            Some(&Value::String("Resolved!".into()))
        );
    }
}
