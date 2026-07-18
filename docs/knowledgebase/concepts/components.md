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

# Layout containers (stack / panel)

Layout is **flexbox-native**: `<stack>` is content-sized by default and grows
only when it opts in (truthy `flex`, `scroll="true"`, or being the layout root);
horizontal stacks center children by default, and `align`/`justify` control the
axes. `<panel>` owns its own decoration (padding/border/border-color/rounded/
shadow/bg) — `apply_layout_styles` skips those props for panels to avoid
double-decorating — and can also grow with `flex`; it draws a subtle 1px hairline
border by default (`border="0"` opts out). See
[layout sizing and centering](../patterns/layout-sizing-and-centering.md).

Visual style values (spacing, radius, typography, semantic color roles) come
from the centralized **design tokens** in `crates/nemo/src/theme/tokens.rs` —
`resolve_theme_color` resolves role aliases like `theme.surface`/`theme.text_muted`,
and `apply_rounded` reads the radius tokens. See
[design tokens and active redesign](../plans/design-tokens.md).

# Containers

**Containers** are higher-level layout components that package a common
application layout so authors describe intent instead of assembling primitives.
They live in `crates/nemo/src/containers/` (separate from `components/`) but wire
in through the same registry + render-dispatch points as any component. The first
is **`app-shell`** — a standard frame with a left `<app-sidenav>` of
`<sidenav-item icon=".." label=".." target="..">`, an `<app-content>` of
`<page id="..">` children, and a full-width `<app-footer>`. Clicking a sidenav
item selects the matching page (its `target` = the page `id`) and highlights the
active item — page switching is built in, needing no handler; `on-click` remains
optional. It follows the typed parent-rendered-children pattern (the shell
collects region markers and their children by `component_type` in the `app_shell`
arm of `render_component`, and the region/leaf markers get no-op standalone arms)
and stores the active target in `ComponentState::SelectedValue` keyed by the
shell's id. See [containers](../patterns/containers.md).

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
