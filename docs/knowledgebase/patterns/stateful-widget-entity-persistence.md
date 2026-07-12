---
type: Pattern
title: Stateful widget Entity persistence
description: Persist widget state in ComponentStates keyed by ID, with data-change detection.
tags: [components, state, gpui]
timestamp: 2026-07-11T00:00:00Z
---

Nemo components are stateless `RenderOnce` values rebuilt every frame, but some
gpui-component widgets (Table, Tree, Input, Slider, Select, Tabs, Accordion,
switches/toggles) carry runtime state that must survive re-renders. That state
lives in `ComponentStates` — a `HashMap<String, ComponentState>` keyed by
component ID on the `App` (`crates/nemo/src/components/state.rs`).

`ComponentState` holds either a GPUI `Entity<T>` (Input, Slider, Table, Tree) or
an `Arc<Mutex<T>>` for state captured by handler closures (Accordion, bool,
selected-value, selected-index).

# Pattern

In `render_component()`, call a `get_or_create_*_state()` helper
(`crates/nemo/src/app.rs`) before building the widget:

1. Look up state by `component.id`.
2. If present, reuse it (clone the `Entity<T>`).
3. If absent, create it via `cx.new(|cx| StateType::new(...))` and insert it.

For data-backed widgets, also do **data-change detection**: store the last data
alongside the state and only push updates when it differs. Example (Table):

```rust
if let Some(ComponentState::Table { state, last_data }) =
    self.component_states.get_mut(&component.id)
{
    if *last_data != current_data {
        let new = current_data.clone();
        state.update(cx, |s, cx| { s.delegate_mut().set_rows(new); s.refresh(cx); });
        *last_data = current_data;
    }
    return state.clone();
}
```

Tree does the same against `last_items`. This avoids rebuilding widget state (and
losing scroll/selection) on every re-render while still reflecting new data.
