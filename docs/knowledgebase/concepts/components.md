---
type: Concept
title: Components
description: The NemoComponent macro, RenderOnce components, and stateful widgets.
tags: [components, gpui, macros]
timestamp: 2026-07-11T00:00:00Z
---

Nemo components are GPUI element wrappers in `crates/nemo/src/components/`. Each
is built from a `BuiltComponent` (resolved config) and rendered by the
`render_component()` dispatch in `crates/nemo/src/app.rs`.

# The NemoComponent macro

`#[derive(NemoComponent)]` (`crates/nemo-macros/src/lib.rs`) generates
`new(component: BuiltComponent) -> Self`, extracting properties from the resolved
config. Field attributes:

* `#[property]` — extract using the field name as the key.
  `#[property(default = "…")]` supplies a fallback; `#[property(name = "key")]`
  uses a different property key.
* `#[children]` — marks a `Vec<AnyElement>` field and generates a `children()`
  builder.
* `#[source]` — stores the whole `BuiltComponent` (for handler/id access). It
  **must be assigned last**, because property extraction borrows `component`
  while `#[source]` consumes it.

Supported property types: `String`, `i64`, `f64`, `bool`, and `Option<T>` of each.

# RenderOnce vs Render

Nemo components implement **`RenderOnce`**
(`fn render(self, &mut Window, &mut App) -> impl IntoElement`): they are
stateless — all input comes from the `BuiltComponent`, and they are consumed on
render, needing no `Entity<Self>` wrapper. `#[derive(IntoElement)]` (from gpui)
supplies the `IntoElement` impl.

`Render` (`&mut self`, `Context<Self>`) is for stateful views backed by an
`Entity<T>`; Nemo uses it for the `App`/workspace shell, not for individual
components.

# Stateful widgets

Widgets that carry runtime state across re-renders (Table, Tree, Input, Slider,
Select, Tabs, Accordion, switches/toggles) persist that state in
`ComponentStates`, a `HashMap<String, ComponentState>` keyed by component ID
(`crates/nemo/src/components/state.rs`). The `ComponentState` enum holds either
GPUI `Entity<T>` handles (Input, Slider, Table, Tree) or `Arc<Mutex<T>>` for
handler-closure state (Accordion, bool/selected-value/selected-index).

`app.rs` `get_or_create_*_state()` methods look up state by ID, create it via
`cx.new(...)` on first render, and perform **data-change detection** — e.g. the
Table compares its current `Vec<Value>` to the stored `last_data` and only calls
`set_rows()`/`refresh()` when it differs (Tree compares `last_items`). See
[stateful widget persistence](../patterns/stateful-widget-entity-persistence.md).

Table and Tree have a sizing gotcha: their `uniform_list` bodies collapse to 0px
without a definite parent height. See
[definite height for lists](../patterns/definite-height-for-lists.md).

# Children and collection data

Container components render their children via `render_children()`; components
that interpret their children's properties render them directly (the
`sidenav-bar` pattern). Some components instead take collection data as a
JSON-string attribute. See
[parent-rendered child components](../patterns/parent-rendered-child-components.md)
and [collection properties as JSON-string attributes](../patterns/json-string-collection-properties.md);
migrating the latter toward the former is tracked in
[declarative children migration](../plans/declarative-children-migration.md).

# Adding a component

Adding a built-in component touches four files. See the
[four-file component workflow](../patterns/four-file-component-workflow.md).
