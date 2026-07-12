---
type: Pattern
title: Four-file component creation workflow
description: Adding a built-in component touches exactly four files.
tags: [components, workflow]
timestamp: 2026-07-11T00:00:00Z
---

Adding a new built-in component to Nemo touches four files.

1. **Implementation** — `crates/nemo/src/components/<name>.rs`. Define the struct
   with `#[derive(IntoElement, NemoComponent)]`, `#[property]`/`#[children]`/
   `#[source]` fields, and `impl RenderOnce`. See
   [Components](../concepts/components.md).
2. **Module registration** — `crates/nemo/src/components/mod.rs`. Add
   `mod <name>;` and `pub use <name>::<Name>;`.
3. **Registry schema** — `crates/nemo-registry/src/builtins.rs`. Register the
   type in the appropriate category function (`register_layout_components`,
   `register_basic_components`, `register_input_components`, …) via the `reg()`
   helper, defining its `ConfigSchema`.
4. **Render dispatch** — `crates/nemo/src/app.rs`. Add a match arm in
   `render_component()`:

   ```rust
   "component_name" => ComponentName::new(component.clone())
       .into_any_element(),
   ```

   For components with children, call `self.render_children(...)` first and pass
   `.children(children)`. For stateful widgets, call the matching
   `self.get_or_create_*_state(...)` and pass the state in — see
   [stateful widget persistence](stateful-widget-entity-persistence.md).

Layout styling (width/height/margin/padding/border/background/shadow/rounded) is
applied uniformly by `apply_layout_styles()` in `app.rs`; individual components
do not need to handle it.
