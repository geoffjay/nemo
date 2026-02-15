//! Core builder types and traits

use indexmap::IndexMap;
use nemo_plugin_api::PluginValue;

/// A builder that can be converted to a PluginValue
pub trait Builder {
    /// Convert this builder to a PluginValue
    fn build(self) -> PluginValue;
}

/// Base builder for creating UI components and layouts
pub struct LayoutBuilder {
    attributes: IndexMap<String, PluginValue>,
    children: Option<IndexMap<String, PluginValue>>,
}

impl LayoutBuilder {
    /// Create a new layout builder with a component type
    pub fn new(component_type: impl Into<String>) -> Self {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "type".to_string(),
            PluginValue::String(component_type.into()),
        );

        Self {
            attributes,
            children: None,
        }
    }

    /// Set an attribute on this layout
    pub fn attr(mut self, key: impl Into<String>, value: impl Into<PluginValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Set multiple attributes at once
    pub fn attrs(mut self, attrs: &[(&str, PluginValue)]) -> Self {
        for (k, v) in attrs {
            self.attributes.insert(k.to_string(), v.clone());
        }
        self
    }

    /// Add a child component with an ID
    pub fn child(mut self, id: impl Into<String>, child: impl Builder) -> Self {
        let children = self.children.get_or_insert_with(IndexMap::new);
        children.insert(id.into(), child.build());
        self
    }

    /// Add multiple children at once
    pub fn children(mut self, children: impl IntoIterator<Item = (String, PluginValue)>) -> Self {
        let child_map = self.children.get_or_insert_with(IndexMap::new);
        for (id, value) in children {
            child_map.insert(id, value);
        }
        self
    }

    /// Set children from an existing map
    pub fn with_children(mut self, children: IndexMap<String, PluginValue>) -> Self {
        self.children = Some(children);
        self
    }
}

impl Builder for LayoutBuilder {
    fn build(mut self) -> PluginValue {
        if let Some(children) = self.children {
            self.attributes
                .insert("component".to_string(), PluginValue::Object(children));
        }
        PluginValue::Object(self.attributes)
    }
}

/// Extension methods for setting common attributes
impl LayoutBuilder {
    /// Set the width
    pub fn width(self, width: i64) -> Self {
        self.attr("width", PluginValue::Integer(width))
    }

    /// Set the height
    pub fn height(self, height: i64) -> Self {
        self.attr("height", PluginValue::Integer(height))
    }

    /// Set the padding
    pub fn padding(self, padding: i64) -> Self {
        self.attr("padding", PluginValue::Integer(padding))
    }

    /// Set the margin
    pub fn margin(self, margin: i64) -> Self {
        self.attr("margin", PluginValue::Integer(margin))
    }

    /// Set the border width
    pub fn border(self, border: i64) -> Self {
        self.attr("border", PluginValue::Integer(border))
    }

    /// Set the border color
    pub fn border_color(self, color: impl Into<String>) -> Self {
        self.attr("border_color", PluginValue::String(color.into()))
    }

    /// Set the background color
    pub fn bg_color(self, color: impl Into<String>) -> Self {
        self.attr("bg_color", PluginValue::String(color.into()))
    }

    /// Set the shadow
    pub fn shadow(self, shadow: impl Into<String>) -> Self {
        self.attr("shadow", PluginValue::String(shadow.into()))
    }

    /// Set a data binding for a property
    pub fn bind(self, property: impl Into<String>, path: impl Into<String>) -> Self {
        let bind_key = format!("bind_{}", property.into());
        self.attr(bind_key, PluginValue::String(path.into()))
    }

    /// Set an event handler
    pub fn on(self, event: impl Into<String>, handler: impl Into<String>) -> Self {
        let event_key = format!("on_{}", event.into());
        self.attr(event_key, PluginValue::String(handler.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_builder_basic() {
        let layout = LayoutBuilder::new("button")
            .attr("label", PluginValue::String("Click".into()))
            .build();

        match layout {
            PluginValue::Object(map) => {
                assert_eq!(map.get("type"), Some(&PluginValue::String("button".into())));
                assert_eq!(map.get("label"), Some(&PluginValue::String("Click".into())));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_layout_builder_with_children() {
        let child = LayoutBuilder::new("label")
            .attr("text", PluginValue::String("Child".into()))
            .build();

        let mut children = IndexMap::new();
        children.insert("child1".to_string(), child);

        let layout = LayoutBuilder::new("panel").with_children(children).build();

        match layout {
            PluginValue::Object(map) => {
                assert!(map.contains_key("component"));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_layout_builder_common_attrs() {
        let layout = LayoutBuilder::new("panel")
            .width(100)
            .height(200)
            .padding(10)
            .margin(5)
            .border(2)
            .build();

        match layout {
            PluginValue::Object(map) => {
                assert_eq!(map.get("width"), Some(&PluginValue::Integer(100)));
                assert_eq!(map.get("height"), Some(&PluginValue::Integer(200)));
                assert_eq!(map.get("padding"), Some(&PluginValue::Integer(10)));
                assert_eq!(map.get("margin"), Some(&PluginValue::Integer(5)));
                assert_eq!(map.get("border"), Some(&PluginValue::Integer(2)));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_event_handler() {
        let layout = LayoutBuilder::new("button")
            .on("click", "handle_click")
            .build();

        match layout {
            PluginValue::Object(map) => {
                assert_eq!(
                    map.get("on_click"),
                    Some(&PluginValue::String("handle_click".into()))
                );
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_data_binding() {
        let layout = LayoutBuilder::new("label")
            .bind("text", "data.message")
            .build();

        match layout {
            PluginValue::Object(map) => {
                assert_eq!(
                    map.get("bind_text"),
                    Some(&PluginValue::String("data.message".into()))
                );
            }
            _ => panic!("Expected Object"),
        }
    }
}
