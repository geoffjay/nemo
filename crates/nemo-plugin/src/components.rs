//! Component builders for common UI elements

use crate::builder::{Builder, LayoutBuilder};
use nemo_plugin_api::PluginValue;

/// Builder for a Label component
pub struct Label(LayoutBuilder);

impl Label {
    /// Create a new label with text
    pub fn new(text: impl Into<String>) -> Self {
        Self(LayoutBuilder::new("label").attr("text", PluginValue::String(text.into())))
    }

    /// Set the label text
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.0 = self.0.attr("text", PluginValue::String(text.into()));
        self
    }

    /// Set the label size (xs, sm, md, lg, xl)
    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.0 = self.0.attr("size", PluginValue::String(size.into()));
        self
    }

    /// Set the label color
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.0 = self.0.attr("color", PluginValue::String(color.into()));
        self
    }

    /// Set the font weight
    pub fn weight(mut self, weight: impl Into<String>) -> Self {
        self.0 = self.0.attr("weight", PluginValue::String(weight.into()));
        self
    }

    /// Bind the text to a data path
    pub fn bind_text(mut self, path: impl Into<String>) -> Self {
        self.0 = self.0.bind("text", path);
        self
    }

    /// Set width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }
}

impl Builder for Label {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for an Input component
pub struct Input(LayoutBuilder);

impl Input {
    /// Create a new input
    pub fn new() -> Self {
        Self(LayoutBuilder::new("input"))
    }

    /// Set the input value
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.0 = self.0.attr("value", PluginValue::String(value.into()));
        self
    }

    /// Set the placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0 = self
            .0
            .attr("placeholder", PluginValue::String(placeholder.into()));
        self
    }

    /// Set the on_change handler
    pub fn on_change(mut self, handler: impl Into<String>) -> Self {
        self.0 = self.0.on("change", handler);
        self
    }

    /// Set the on_submit handler
    pub fn on_submit(mut self, handler: impl Into<String>) -> Self {
        self.0 = self.0.on("submit", handler);
        self
    }

    /// Bind the value to a data path
    pub fn bind_value(mut self, path: impl Into<String>) -> Self {
        self.0 = self.0.bind("value", path);
        self
    }

    /// Set width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set whether the input is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.attr("disabled", PluginValue::Bool(disabled));
        self
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder for Input {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a Button component
pub struct Button(LayoutBuilder);

impl Button {
    /// Create a new button with label
    pub fn new(label: impl Into<String>) -> Self {
        Self(LayoutBuilder::new("button").attr("label", PluginValue::String(label.into())))
    }

    /// Set the button label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.0 = self.0.attr("label", PluginValue::String(label.into()));
        self
    }

    /// Set the button variant (primary, secondary, danger, etc.)
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.0 = self.0.attr("variant", PluginValue::String(variant.into()));
        self
    }

    /// Set the on_click handler
    pub fn on_click(mut self, handler: impl Into<String>) -> Self {
        self.0 = self.0.on("click", handler);
        self
    }

    /// Set whether the button is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.attr("disabled", PluginValue::Bool(disabled));
        self
    }

    /// Set width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }
}

impl Builder for Button {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a Switch/Toggle component
pub struct Switch(LayoutBuilder);

impl Switch {
    /// Create a new switch
    pub fn new() -> Self {
        Self(LayoutBuilder::new("switch"))
    }

    /// Set the switch label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.0 = self.0.attr("label", PluginValue::String(label.into()));
        self
    }

    /// Set whether the switch is checked
    pub fn checked(mut self, checked: bool) -> Self {
        self.0 = self.0.attr("checked", PluginValue::Bool(checked));
        self
    }

    /// Set the on_click handler
    pub fn on_click(mut self, handler: impl Into<String>) -> Self {
        self.0 = self.0.on("click", handler);
        self
    }

    /// Bind the checked state to a data path
    pub fn bind_checked(mut self, path: impl Into<String>) -> Self {
        self.0 = self.0.bind("checked", path);
        self
    }

    /// Set whether the switch is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.attr("disabled", PluginValue::Bool(disabled));
        self
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder for Switch {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a Slider component
pub struct Slider(LayoutBuilder);

impl Slider {
    /// Create a new slider
    pub fn new() -> Self {
        Self(LayoutBuilder::new("slider"))
    }

    /// Set the slider value
    pub fn value(mut self, value: f64) -> Self {
        self.0 = self.0.attr("value", PluginValue::Float(value));
        self
    }

    /// Set the minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.0 = self.0.attr("min", PluginValue::Float(min));
        self
    }

    /// Set the maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.0 = self.0.attr("max", PluginValue::Float(max));
        self
    }

    /// Set the step increment
    pub fn step(mut self, step: f64) -> Self {
        self.0 = self.0.attr("step", PluginValue::Float(step));
        self
    }

    /// Set the on_change handler
    pub fn on_change(mut self, handler: impl Into<String>) -> Self {
        self.0 = self.0.on("change", handler);
        self
    }

    /// Bind the value to a data path
    pub fn bind_value(mut self, path: impl Into<String>) -> Self {
        self.0 = self.0.bind("value", path);
        self
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder for Slider {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for an Image component
pub struct Image(LayoutBuilder);

impl Image {
    /// Create a new image with source path
    pub fn new(src: impl Into<String>) -> Self {
        Self(LayoutBuilder::new("image").attr("src", PluginValue::String(src.into())))
    }

    /// Set the image source
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.0 = self.0.attr("src", PluginValue::String(src.into()));
        self
    }

    /// Set the alt text
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.0 = self.0.attr("alt", PluginValue::String(alt.into()));
        self
    }

    /// Set width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }
}

impl Builder for Image {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}
