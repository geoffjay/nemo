//! Container builders for layout management

use crate::builder::{Builder, LayoutBuilder};
use nemo_plugin_api::PluginValue;

/// Builder for a Panel container
pub struct Panel(LayoutBuilder);

impl Panel {
    /// Create a new panel
    pub fn new() -> Self {
        Self(LayoutBuilder::new("panel"))
    }

    /// Set the padding
    pub fn padding(mut self, padding: i64) -> Self {
        self.0 = self.0.padding(padding);
        self
    }

    /// Set the margin
    pub fn margin(mut self, margin: i64) -> Self {
        self.0 = self.0.margin(margin);
        self
    }

    /// Set the border width
    pub fn border(mut self, border: i64) -> Self {
        self.0 = self.0.border(border);
        self
    }

    /// Set the border color
    pub fn border_color(mut self, color: impl Into<String>) -> Self {
        self.0 = self.0.border_color(color);
        self
    }

    /// Set the background color
    pub fn bg_color(mut self, color: impl Into<String>) -> Self {
        self.0 = self.0.bg_color(color);
        self
    }

    /// Set the shadow
    pub fn shadow(mut self, shadow: impl Into<String>) -> Self {
        self.0 = self.0.shadow(shadow);
        self
    }

    /// Set the width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set the height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }

    /// Add a child component
    pub fn child(mut self, id: impl Into<String>, child: impl Builder) -> Self {
        self.0 = self.0.child(id, child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = (String, PluginValue)>) -> Self {
        self.0 = self.0.children(children);
        self
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder for Panel {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a Stack container (horizontal or vertical)
pub struct Stack(LayoutBuilder);

impl Stack {
    /// Create a new vertical stack
    pub fn vertical() -> Self {
        Self(
            LayoutBuilder::new("stack")
                .attr("direction", PluginValue::String("vertical".to_string())),
        )
    }

    /// Create a new horizontal stack
    pub fn horizontal() -> Self {
        Self(
            LayoutBuilder::new("stack")
                .attr("direction", PluginValue::String("horizontal".to_string())),
        )
    }

    /// Set the spacing between children
    pub fn spacing(mut self, spacing: i64) -> Self {
        self.0 = self.0.attr("spacing", PluginValue::Integer(spacing));
        self
    }

    /// Set the padding
    pub fn padding(mut self, padding: i64) -> Self {
        self.0 = self.0.padding(padding);
        self
    }

    /// Set the alignment (start, center, end, stretch)
    pub fn align(mut self, align: impl Into<String>) -> Self {
        self.0 = self.0.attr("align", PluginValue::String(align.into()));
        self
    }

    /// Set the justification (start, center, end, space-between, space-around)
    pub fn justify(mut self, justify: impl Into<String>) -> Self {
        self.0 = self.0.attr("justify", PluginValue::String(justify.into()));
        self
    }

    /// Set the width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set the height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }

    /// Add a child component
    pub fn child(mut self, id: impl Into<String>, child: impl Builder) -> Self {
        self.0 = self.0.child(id, child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = (String, PluginValue)>) -> Self {
        self.0 = self.0.children(children);
        self
    }
}

impl Builder for Stack {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a Grid container
pub struct Grid(LayoutBuilder);

impl Grid {
    /// Create a new grid with columns
    pub fn new(columns: i64) -> Self {
        Self(LayoutBuilder::new("grid").attr("columns", PluginValue::Integer(columns)))
    }

    /// Set the number of columns
    pub fn columns(mut self, columns: i64) -> Self {
        self.0 = self.0.attr("columns", PluginValue::Integer(columns));
        self
    }

    /// Set the gap between cells
    pub fn gap(mut self, gap: i64) -> Self {
        self.0 = self.0.attr("gap", PluginValue::Integer(gap));
        self
    }

    /// Set the padding
    pub fn padding(mut self, padding: i64) -> Self {
        self.0 = self.0.padding(padding);
        self
    }

    /// Set the width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set the height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }

    /// Add a child component
    pub fn child(mut self, id: impl Into<String>, child: impl Builder) -> Self {
        self.0 = self.0.child(id, child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = (String, PluginValue)>) -> Self {
        self.0 = self.0.children(children);
        self
    }
}

impl Builder for Grid {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Builder for a ScrollView container
pub struct ScrollView(LayoutBuilder);

impl ScrollView {
    /// Create a new scroll view
    pub fn new() -> Self {
        Self(LayoutBuilder::new("scroll"))
    }

    /// Set the direction (vertical, horizontal, both)
    pub fn direction(mut self, direction: impl Into<String>) -> Self {
        self.0 = self
            .0
            .attr("direction", PluginValue::String(direction.into()));
        self
    }

    /// Set the width
    pub fn width(mut self, width: i64) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Set the height
    pub fn height(mut self, height: i64) -> Self {
        self.0 = self.0.height(height);
        self
    }

    /// Add a child component
    pub fn child(mut self, id: impl Into<String>, child: impl Builder) -> Self {
        self.0 = self.0.child(id, child);
        self
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder for ScrollView {
    fn build(self) -> PluginValue {
        self.0.build()
    }
}

/// Helper to create a horizontal row with a label and another component
pub fn input_row(
    label_text: impl Into<String>,
    label_width: Option<i64>,
    component: impl Builder,
    spacing: Option<i64>,
) -> Stack {
    let mut label = crate::components::Label::new(label_text);
    if let Some(width) = label_width {
        label = label.width(width);
    }

    let mut stack = Stack::horizontal()
        .child("label", label)
        .child("component", component);

    if let Some(spacing) = spacing {
        stack = stack.spacing(spacing);
    }

    stack
}

/// Helper to create a row with custom children
pub fn row(spacing: i64) -> Stack {
    Stack::horizontal().spacing(spacing)
}

/// Helper to create a column
pub fn column(spacing: i64) -> Stack {
    Stack::vertical().spacing(spacing)
}
