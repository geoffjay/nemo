---
name: nemo-component-patterns
description: Reference for Nemo component conventions including NemoComponent derive, RenderOnce vs Render, stateful vs stateless patterns, sizing gotchas, and color resolution. Use when writing or modifying Nemo components.
---

# Nemo Component Patterns

Use this skill when writing, modifying, or debugging Nemo UI components. It covers the conventions, patterns, and common pitfalls for the component system.

## The 4-File Pattern

Every component touches exactly 4 files:

1. `crates/nemo/src/components/<name>.rs` — struct + `RenderOnce` impl
2. `crates/nemo/src/components/mod.rs` — `mod` + `pub use`
3. `crates/nemo-registry/src/builtins.rs` — schema registration
4. `crates/nemo/src/app.rs` — render dispatch match arm

## Component Struct Convention

```rust
use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct MyComponent {
    // Properties extracted from BuiltComponent
    #[property(default = "default_value")]
    my_prop: String,
    #[property]
    optional_prop: Option<i64>,
    #[children]              // Only if accepts children
    children: Vec<AnyElement>,
    // Non-macro fields
    runtime: Option<Arc<NemoRuntime>>,  // Only if has event handlers
    entity_id: Option<EntityId>,         // Only if has event handlers
    #[source]                // MUST be last field
    source: nemo_layout::BuiltComponent,
}
```

## Key Rules

### RenderOnce, Not Render
- All Nemo components implement `RenderOnce` (stateless, consumed on render)
- `Render` is for GPUI views backed by `Entity<T>` — NOT for Nemo components
- `RenderOnce` signature: `fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement`

### `#[source]` Must Be Last
The macro generates code that consumes the `component` parameter. Property extractions borrow it first, so `#[source]` assignment must come after all `#[property]` fields.

### Supported Property Types
- `String`, `i64`, `f64`, `bool`
- `Option<String>`, `Option<i64>`, `Option<f64>`, `Option<bool>`
- Use `Option<T>` for optional properties, bare `T` with `#[property(default = ...)]` for required

### Builder Methods for Runtime/EntityId
Components with event handlers need builder methods (the macro doesn't generate these):
```rust
impl MyComponent {
    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }
    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}
```

### Event Handler Pattern
```rust
// In RenderOnce::render():
let click_handler = self.source.handlers.get("click").cloned();
let component_id = self.source.id.clone();
if let Some(handler) = click_handler {
    if let (Some(runtime), Some(entity_id)) = (self.runtime, self.entity_id) {
        element = element.on_click(move |_event, _window, cx| {
            runtime.call_handler(&handler, &component_id, "click");
            cx.notify(entity_id);
        });
    }
}
```

### Color Resolution
Use `resolve_color()` from `components/mod.rs`:
- Theme colors: `"theme.border"`, `"theme.accent"`, `"theme.danger"`, etc.
- Hex colors: `"#4c566a"`, `"FF0000"`, `"#FF000080"` (with alpha)

### Shadow and Rounded Presets
- `apply_shadow(div, Some("md"))` — sm, md, lg, xl, 2xl
- `apply_rounded(div, Some("lg"))` — sm, md, lg, xl, full

## Stateful Component Pattern

For widgets that need state persistence across re-renders (Table, Tree, Input, Slider):

1. Add a variant to `ComponentState` enum in `components/state.rs`
2. Add a `get_or_create_*_state()` method in `app.rs`
3. Store `Entity<T>` in `ComponentStates`
4. Compare data to detect changes and update state

### Height Gotcha for List Widgets
`Table` and `Tree` use `uniform_list` — they collapse to 0px height without a parent that has definite height. Wrap in `div().w_full().h(px(height))` (default 300px).

## Registry Schema Pattern

```rust
reg(
    registry,
    "my_component",                    // snake_case name (XML uses kebab-case)
    ComponentCategory::Display,        // Category
    "My Component",                    // Display name
    "Description of the component",    // Description
    ConfigSchema::new("my_component")
        .property("label", PropertySchema::string())
        .property("size", PropertySchema::string().with_default("md"))
        .property("count", PropertySchema::integer())
        .property("enabled", PropertySchema::boolean().with_default(true))
        .require("label"),             // Required properties
);
```

## Render Dispatch Patterns

In `app.rs` `render_component()`:

```rust
// Simple (no state, no children, no handlers)
"my_component" => MyComponent::new(component.clone()).into_any_element(),

// With children
"container" => {
    let children = self.render_children(component, components, entity_id, window, cx);
    Container::new(component.clone()).children(children).into_any_element()
}

// With event handlers
"button" => Button::new(component.clone())
    .runtime(Arc::clone(&self.runtime))
    .entity_id(entity_id)
    .into_any_element(),

// Stateful
"table" => {
    let state = self.get_or_create_table_state(component, window, cx);
    Table::new(component.clone()).table_state(state).into_any_element()
}
```
