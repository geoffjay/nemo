[![Documentation][docsrs-badge]][docsrs-url]

[docsrs-badge]: https://docs.rs/nemo-plugin/badge.svg
[docsrs-url]: https://docs.rs/nemo-plugin

# nemo-plugin

A Rust library providing a fluent, type-safe builder API for developing Nemo plugins. This crate builds on top of `nemo-plugin-api` to make creating UI layouts and templates more ergonomic and less error-prone.

## Features

- **Fluent Builder Pattern**: Chain method calls for clean, readable code
- **Type-Safe**: Compile-time checks for component properties
- **Component Builders**: Pre-built helpers for common UI elements
- **Container Builders**: Layout containers like Panel, Stack, Grid
- **Helper Functions**: Utilities for common patterns like input rows

## Installation

Add this to your plugin's `Cargo.toml`:

```toml
[dependencies]
nemo-plugin = "0.5"
```

## Quick Start

```rust
use nemo_plugin::prelude::*;

fn build_ui() -> PluginValue {
    Panel::new()
        .padding(16)
        .border(2)
        .border_color("theme.border")
        .width(300)
        .child("title", Label::new("My Plugin").size("xl"))
        .child("input", Input::new()
            .placeholder("Enter value")
            .on_change("on_input_change"))
        .child("button", Button::new("Submit")
            .variant("primary")
            .on_click("on_submit"))
        .build()
}
```

## Component Builders

### Label

```rust
Label::new("Hello World")
    .size("lg")
    .color("theme.primary")
    .weight("bold")
    .bind_text("data.message")
```

### Input

```rust
Input::new()
    .value("default")
    .placeholder("Type here...")
    .on_change("handle_change")
    .bind_value("data.input")
```

### Button

```rust
Button::new("Click Me")
    .variant("primary")
    .on_click("handle_click")
    .disabled(false)
```

### Switch

```rust
Switch::new()
    .label("Enable Feature")
    .checked(false)
    .on_click("toggle_feature")
    .bind_checked("data.enabled")
```

### Slider

```rust
Slider::new()
    .value(50.0)
    .min(0.0)
    .max(100.0)
    .step(1.0)
    .on_change("handle_slider")
```

## Container Builders

### Panel

A bordered container with optional padding and shadow:

```rust
Panel::new()
    .padding(16)
    .margin(8)
    .border(2)
    .border_color("theme.border")
    .shadow("md")
    .bg_color("theme.surface")
    .child("content", Label::new("Content"))
```

### Stack

Horizontal or vertical layout container:

```rust
// Vertical stack
Stack::vertical()
    .spacing(8)
    .align("start")
    .justify("center")
    .child("item1", Label::new("Item 1"))
    .child("item2", Label::new("Item 2"))

// Horizontal stack
Stack::horizontal()
    .spacing(12)
    .child("left", Label::new("Left"))
    .child("right", Label::new("Right"))
```

### Grid

Grid layout with configurable columns:

```rust
Grid::new(3)  // 3 columns
    .gap(8)
    .padding(16)
    .child("cell1", Label::new("Cell 1"))
    .child("cell2", Label::new("Cell 2"))
    .child("cell3", Label::new("Cell 3"))
```

### ScrollView

Scrollable container:

```rust
ScrollView::new()
    .direction("vertical")
    .width(400)
    .height(300)
    .child("content", Stack::vertical()
        .spacing(8)
        // ... many children
    )
```

## Helper Functions

### Input Row

Create a label + input combo:

```rust
use nemo_plugin::containers::input_row;

let row = input_row(
    "Username",           // label text
    Some(120),           // label width
    Input::new()         // component
        .placeholder("Enter username")
        .on_change("on_username"),
    Some(8),            // spacing
);
```

### Row and Column Helpers

```rust
use nemo_plugin::containers::{row, column};

// Quick horizontal row
let my_row = row(8)
    .child("label", Label::new("Name:"))
    .child("input", Input::new());

// Quick vertical column
let my_column = column(12)
    .child("header", Label::new("Title"))
    .child("body", Label::new("Content"));
```

## Common Patterns

### Form Layout

```rust
fn build_form() -> PluginValue {
    Panel::new()
        .padding(20)
        .child("form", Stack::vertical()
            .spacing(12)
            .child("name_row", input_row(
                "Name", Some(100),
                Input::new().on_change("on_name_change"),
                Some(8)
            ))
            .child("email_row", input_row(
                "Email", Some(100),
                Input::new().on_change("on_email_change"),
                Some(8)
            ))
            .child("submit", Button::new("Submit")
                .variant("primary")
                .on_click("on_submit"))
        )
        .build()
}
```

### Settings Panel

```rust
fn build_settings() -> PluginValue {
    Panel::new()
        .padding(16)
        .border(1)
        .child("settings", Stack::vertical()
            .spacing(16)
            .child("title", Label::new("Settings").size("xl"))
            .child("dark_mode", Switch::new()
                .label("Dark Mode")
                .bind_checked("settings.dark_mode"))
            .child("notifications", Switch::new()
                .label("Enable Notifications")
                .bind_checked("settings.notifications"))
            .child("volume", Stack::vertical()
                .spacing(4)
                .child("label", Label::new("Volume"))
                .child("slider", Slider::new()
                    .min(0.0)
                    .max(100.0)
                    .bind_value("settings.volume"))
            )
        )
        .build()
}
```

### Data Dashboard

```rust
fn build_dashboard() -> PluginValue {
    Panel::new()
        .padding(20)
        .child("dashboard", Grid::new(2)
            .gap(16)
            .child("temp", Panel::new()
                .padding(12)
                .border(1)
                .child("content", Stack::vertical()
                    .spacing(8)
                    .child("label", Label::new("Temperature"))
                    .child("value", Label::new("--")
                        .size("xl")
                        .bind_text("data.temperature"))
                )
            )
            .child("humidity", Panel::new()
                .padding(12)
                .border(1)
                .child("content", Stack::vertical()
                    .spacing(8)
                    .child("label", Label::new("Humidity"))
                    .child("value", Label::new("--")
                        .size("xl")
                        .bind_text("data.humidity"))
                )
            )
        )
        .build()
}
```

## Advanced Usage

### Custom Attributes

If you need to set custom attributes not covered by the builder methods:

```rust
use nemo_plugin::builder::{LayoutBuilder, Builder};

let custom = LayoutBuilder::new("my-custom-component")
    .attr("custom_prop", PluginValue::String("value".into()))
    .attr("numeric_prop", PluginValue::Integer(42))
    .build();
```

### Event Handlers

All components support the `.on()` method for custom events:

```rust
Button::new("Click")
    .on("click", "handle_click")
    .on("hover", "handle_hover")
    .on("focus", "handle_focus")
```

### Data Binding

Use `.bind()` for custom bindings:

```rust
use nemo_plugin::builder::{LayoutBuilder, Builder};

LayoutBuilder::new("label")
    .attr("text", PluginValue::String("Loading...".into()))
    .bind("text", "data.dynamic_value")
    .bind("color", "theme.text_color")
    .build()
```

## Comparison: Before and After

### Before (manual PluginValue construction)

```rust
fn build_old_way() -> PluginValue {
    let mut label_map = IndexMap::new();
    label_map.insert("type".to_string(), PluginValue::String("label".into()));
    label_map.insert("text".to_string(), PluginValue::String("Hello".into()));
    label_map.insert("width".to_string(), PluginValue::Integer(120));
    
    let mut input_map = IndexMap::new();
    input_map.insert("type".to_string(), PluginValue::String("input".into()));
    input_map.insert("value".to_string(), PluginValue::String("default".into()));
    input_map.insert("on_change".to_string(), PluginValue::String("handler".into()));
    
    let mut children = IndexMap::new();
    children.insert("label".to_string(), PluginValue::Object(label_map));
    children.insert("input".to_string(), PluginValue::Object(input_map));
    
    let mut container = IndexMap::new();
    container.insert("type".to_string(), PluginValue::String("stack".into()));
    container.insert("direction".to_string(), PluginValue::String("horizontal".into()));
    container.insert("spacing".to_string(), PluginValue::Integer(8));
    container.insert("component".to_string(), PluginValue::Object(children));
    
    PluginValue::Object(container)
}
```

### After (with nemo-plugin builders)

```rust
fn build_new_way() -> PluginValue {
    Stack::horizontal()
        .spacing(8)
        .child("label", Label::new("Hello").width(120))
        .child("input", Input::new()
            .value("default")
            .on_change("handler"))
        .build()
}
```

## API Reference

All builders implement the `Builder` trait:

```rust
pub trait Builder {
    fn build(self) -> PluginValue;
}
```

This allows them to be used interchangeably as children of container components.

## License

MIT OR Apache-2.0
