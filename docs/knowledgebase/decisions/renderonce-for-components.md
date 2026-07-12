---
type: Decision
title: Components implement RenderOnce, not Render
description: UI components are stateless and consumed on render; only shell/stateful views use Render.
tags: [components, gpui, decision]
timestamp: 2026-07-11T00:00:00Z
---

# Decision

Nemo UI components implement GPUI's **`RenderOnce`**
(`fn render(self, &mut Window, &mut App) -> impl IntoElement`) with
`#[derive(IntoElement)]`, not `Render`.

# Rationale

Components are stateless: every input comes from the `BuiltComponent` (resolved
config), so there is nothing to persist between frames. `RenderOnce` consumes
`self` and needs no `Entity<Self>` wrapper, which matches how
`render_component()` in `crates/nemo/src/app.rs` constructs a fresh component
from the config snapshot on each render.

`Render` (`&mut self`, `Context<Self>`) is reserved for stateful views backed by
an `Entity<T>` — the `App` and workspace shell — where GPUI must retain and
mutate state.

# Consequences

* New components follow the `#[derive(IntoElement, NemoComponent)]` +
  `impl RenderOnce` shape. See [Components](../concepts/components.md) and the
  [four-file component workflow](../patterns/four-file-component-workflow.md).
* Widgets that genuinely need runtime state (Table, Tree, Input, …) keep that
  state outside the component, in `ComponentStates` — see
  [stateful widget persistence](../patterns/stateful-widget-entity-persistence.md).
