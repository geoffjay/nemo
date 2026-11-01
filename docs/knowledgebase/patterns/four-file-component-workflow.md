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

**Wrapping a gpui-kit component** often means a hand-written wrapper struct like
`kbd` (`crates/nemo/src/components/kbd.rs`, 2026-10-31): `#[derive(IntoElement)]`
only, read properties off `BuiltComponent` in `render()`, construct the kit
element (`gpui_kit::component::kbd::Kbd`), and wrap it in
`div().flex_none().child(...)` so it doesn't stretch in flex parents (same shape
as `tag.rs`). Keystroke input uses `Keystroke::parse_with_os(&value,
cx.platform_info().os)` (the dynamic-input path; plain `Keystroke::parse`
requires `&'static str`) and falls back to rendering the raw string unstyled
rather than panicking on unparsable input. Component gallery reference pages
(`examples/components/templates/pages/*.nemo`) are authored per component; the
[/implement-component skill](`.agents/skills/implement-component/SKILL.md`)
automates the full wrap → screenshot-verify → page-update workflow.

When the kit element paints its own background and is meant to fill its
container (skeleton, 2026-11-01), return it directly instead of wrapping in
`div().flex_none()`, and apply any config sizing to the element itself
(`width`/`height` as px, `rounded` via `theme::tokens::radius_for`) — the
universal wrapper's sizing/radius from `apply_layout_styles()` frames a wrapper
div, but cannot reach the kit element's own background.

**Containers** (`crates/nemo/src/containers/`) follow the same four wiring points,
with two differences: the implementation file lives under `containers/` (declared
via `mod containers;` in `main.rs` and re-exported from `containers/mod.rs`), and
because a container routes its children into regions itself it typically uses
hand-written builders (`#[derive(IntoElement)]` only, like `SidenavBar`) instead
of the `NemoComponent` derive, plus no-op standalone arms for its region/leaf
marker types. See [containers](containers.md).
