---
description: Scaffold a new Nemo UI component across all required files
---

# Add Nemo Component

Scaffold a new UI component for the Nemo framework. This command creates all necessary code across the 4 required files and optionally registers it in the component registry.

## Arguments

The user should provide:
- **Component name** (e.g., `calendar`, `breadcrumb`, `stepper`) — snake_case
- **Category** — one of: Layout, Display, Input, Data, Feedback, Navigation, Charts
- **Properties** — list of properties with types (string, integer, float, boolean)
- **Has children** — whether the component accepts child elements
- **Has event handlers** — whether it needs runtime/entity_id for click/change handlers
- **Is stateful** — whether it needs Entity<T> persistence (rare — only for widgets backed by gpui-component state)

If any of these are not provided, ask the user before proceeding.

## Steps

### 1. Read existing patterns

Read these files to understand current conventions:
- `crates/nemo/src/components/button.rs` — example with event handlers
- `crates/nemo/src/components/label.rs` — simple stateless example
- `crates/nemo/src/components/stack.rs` — example with children
- `crates/nemo/src/components/mod.rs` — for insertion point
- `crates/nemo-registry/src/builtins.rs` — for schema registration pattern
- `crates/nemo/src/app.rs` — for render_component() match location

### 2. Create the component file

Create `crates/nemo/src/components/<name>.rs` with:

```rust
use gpui::*;
use nemo_macros::NemoComponent;
// Add: use gpui_component::... if wrapping a gpui-component widget
// Add: use std::sync::Arc; and use crate::runtime::NemoRuntime; if has event handlers
// Add: use super::resolve_color; if the component uses colors

/// Brief description of the component.
///
/// # XML Configuration
///
/// ```xml
/// <component_name id="example" prop="value" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | ... | ... | ... |
#[derive(IntoElement, NemoComponent)]
pub struct ComponentName {
    #[source]
    source: nemo_layout::BuiltComponent,
    // #[property] fields here
    // #[children] children: Vec<AnyElement>, if has children
    // runtime: Option<Arc<NemoRuntime>>, if has handlers
    // entity_id: Option<EntityId>, if has handlers
}

// Add builder methods for runtime/entity_id if has handlers:
// impl ComponentName {
//     pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self { ... }
//     pub fn entity_id(mut self, entity_id: EntityId) -> Self { ... }
// }

impl RenderOnce for ComponentName {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Implementation
    }
}
```

### 3. Update mod.rs

Edit `crates/nemo/src/components/mod.rs`:
- Add `mod <name>;` in alphabetical order with the other mod declarations
- Add `pub use <name>::<Name>;` in alphabetical order with the other pub use statements

### 4. Register in builtins.rs

Edit `crates/nemo-registry/src/builtins.rs`:
- Add a `reg()` call in the appropriate category function (e.g., `register_display_components()`)
- Define the `ConfigSchema` with all properties and required fields
- Follow the pattern of existing registrations in that category

### 5. Add render dispatch in app.rs

Edit `crates/nemo/src/app.rs` in the `render_component()` method:
- Add a match arm for `"<name>"` in alphabetical position
- Pattern depends on component type:
  - **Simple**: `"name" => Name::new(component.clone()).into_any_element(),`
  - **With children**: render children first, then `.children(children)`
  - **With handlers**: add `.runtime(Arc::clone(&self.runtime)).entity_id(entity_id)`
  - **Stateful**: create/get state first, then pass to component

### 6. Verify

Run `cargo check -p nemo` to verify compilation.

## Important Notes

- Component names in XML use kebab-case (`line-chart`) but Rust uses snake_case (`line_chart`)
- The XML parser converts kebab-case to snake_case automatically
- `#[source]` field must be listed LAST in the struct (it consumes the BuiltComponent)
- Use `Option<T>` for optional properties, bare `T` with `#[property(default = ...)]` for required ones
- If wrapping a gpui-component widget, check its API for required state types
