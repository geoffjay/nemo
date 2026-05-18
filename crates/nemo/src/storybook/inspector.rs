//! Interactive property inspector for the Nemo storybook.
//!
//! Renders a read-only table of property names, types, defaults and constraints
//! derived from a component's ConfigSchema. A future extension can wire the
//! controls to `set_component_property()` on a live runtime.

use gpui::*;
use gpui_component::{h_flex, label::Label, v_flex, ActiveTheme};
use nemo_config::ValueType;
use nemo_registry::ComponentRegistry;
use std::sync::Arc;

/// Displays a property table for a selected component.
#[allow(dead_code)]
pub struct PropertyInspector {
    component_name: Option<String>,
    target_component_id: Option<String>,
    registry: Arc<ComponentRegistry>,
    focus_handle: FocusHandle,
}

#[allow(dead_code)]
impl PropertyInspector {
    pub fn new(
        registry: Arc<ComponentRegistry>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            component_name: None,
            target_component_id: None,
            registry,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Set which component to inspect. `component_id` is the ID of the live
    /// preview instance to update when properties change.
    pub fn inspect(&mut self, component_name: &str, component_id: &str, cx: &mut Context<Self>) {
        self.component_name = Some(component_name.to_string());
        self.target_component_id = Some(component_id.to_string());
        cx.notify();
    }

    fn type_label(vt: &ValueType) -> &'static str {
        match vt {
            ValueType::String => "string",
            ValueType::Integer => "integer",
            ValueType::Float => "float",
            ValueType::Boolean => "boolean",
            ValueType::Array => "array",
            ValueType::Object => "object",
            ValueType::Any => "any",
        }
    }
}

impl Render for PropertyInspector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(ref comp_name) = self.component_name.clone() else {
            return v_flex()
                .p_4()
                .child(Label::new("Select a component to inspect its properties."))
                .into_any();
        };

        let Some(descriptor) = self.registry.get_component(comp_name) else {
            return v_flex()
                .p_4()
                .child(Label::new(format!("Component '{}' not found.", comp_name)))
                .into_any();
        };

        let schema = &descriptor.schema;
        let required_set: std::collections::HashSet<&str> =
            schema.required.iter().map(|s| s.as_str()).collect();

        // Header row
        let header = h_flex()
            .w_full()
            .pb_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w_32().child(Label::new("Property").text_sm()))
            .child(div().w_16().child(Label::new("Type").text_sm()))
            .child(div().flex_1().child(Label::new("Default").text_sm()))
            .child(div().w_16().child(Label::new("Req.").text_sm()));

        // Property rows
        let rows: Vec<AnyElement> = schema
            .properties
            .iter()
            .map(|(prop_name, prop_schema)| {
                let required = required_set.contains(prop_name.as_str());
                let type_str = Self::type_label(&prop_schema.value_type);
                let default_str = prop_schema
                    .default
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let req_str = if required { "yes" } else { "no" };
                let name_display = if required {
                    format!("{} *", prop_name)
                } else {
                    prop_name.clone()
                };

                h_flex()
                    .w_full()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().w_32().child(Label::new(name_display).text_sm()))
                    .child(div().w_16().child(Label::new(type_str).text_sm()))
                    .child(div().flex_1().child(Label::new(default_str).text_sm()))
                    .child(div().w_16().child(Label::new(req_str).text_sm()))
                    .into_any()
            })
            .collect();

        v_flex()
            .p_4()
            .gap_2()
            .child(header)
            .children(rows)
            .into_any()
    }
}
