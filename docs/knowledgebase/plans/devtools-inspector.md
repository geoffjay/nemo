---
type: Plan
title: Devtools inspector
description: What it would take to add a nemo-devtools crate — a live inspector for the component tree, data repository, event bus, and bindings.
tags: [devtools, inspector, tooling, roadmap]
timestamp: 2026-07-14T00:00:00Z
---

# What "devtools" means for nemo

For a config-driven [GPUI](../concepts/architecture.md) app, the browser /
React-DevTools analogue is a **live inspector**: walk the component tree,
view/edit properties on a running app, watch the [data repository](../concepts/data-flow.md)
mutate in real time, tail the [event bus](../concepts/architecture.md), and list
active bindings. The key finding of this research: nemo's architecture is
unusually well-suited to this — nearly every subsystem already exposes an
observable handle reachable from `Arc<NemoRuntime>`. The missing piece is a
**presentation layer**, not new instrumentation.

# The introspection surfaces already exist

| Surface | How it's reached today | State |
|---|---|---|
| **Component tree** | `layout_manager: Arc<RwLock<LayoutManager>>` — snapshot via `read()` → `component_ids()` → `get_component().cloned()`, walk from `root_id()` following `BuiltComponent.children` (exactly what `app.rs:348-361` does for rendering) | read + write both work |
| **Live property edit** | `runtime.set_component_property()` / `get_component_property()` (`runtime.rs:1760`/`1771`) — already lock-safe, already `Value ↔ PluginValue` | used by Rhai today |
| **Data repository** | `DataRepository::subscribe()` → `broadcast::Receiver<RepositoryChange>` carrying **before/after values + timestamp**; full dump via `get(&DataPath::parse("")…)` | Send+Sync |
| **Event bus** | `EventBus::subscribe()` → independent broadcast fan-out; rich `EventFilter` (glob/type/prefix); optional `EventTracer` ring buffer | Send+Sync |
| **Bindings** | `BindingSystem::list_bindings()` + `get_binding(id)` exposes source path, target, mode, last value | poll-only (no change stream) |
| **Component schemas** (for the property-editor form) | `ComponentRegistry` → `ConfigSchema.properties: IndexMap<name, PropertySchema>` with `ValueType`, `default`, and `OneOf` enum rules | types/defaults ready; descriptions/enums sparse in `builtins.rs` |

The reactive signal is reusable too: all data-driven re-renders funnel through
one `tokio::sync::Notify` (`runtime.data_notify`) → `apply_pending_data_updates()`
→ `cx.notify()`. A devtools panel can piggyback on that same signal to stay live.

# The real gaps

These genuinely do not exist yet:

1. **`BuiltComponent` is not `Serialize`** (`manager.rs:28` derives only
   `Debug, Clone`). All field types (`String`, `HashMap`, `Value`) already
   implement serde, so this is a one-line derive — required only for a JSON tree
   dump (an external client), not for an in-process panel.
2. **No `snapshot()` on `DataRepository`** — the empty-path `get()` trick works
   but a real accessor is cleaner. The change channel capacity is only **100**;
   a busy tree drops events to a slow consumer (`Lagged`).
3. **Event tracing is off by default** — the runtime builds the bus with
   `with_default_capacity()` (`runtime.rs:99`), not `with_tracing()`. A history
   panel needs tracing enabled at construction (a startup flag).
4. **No timing/profiling instrumentation anywhere** — no frame timing, render
   counts, or metrics. A "performance" tab would be built from scratch.
5. **No network/IPC surface at all** — nothing exposes state over a socket. An
   external devtools client means building a server transport from zero, and
   `NemoRuntime` is `!Send`, so the server task must marshal layout reads through
   the main-thread `Arc<RwLock>` (the data/event handles are `Send+Sync` and
   stream freely; the layout tree needs care).

# Two architectures

## Option A — In-process panel (recommended)

A `nemo-devtools` crate providing a GPUI view mounted inside the existing
workspace. It holds `Arc<NemoRuntime>` directly, reads the live `LayoutManager`/
registry, and re-renders off `data_notify`.

* **Pros:** no serialization, no transport, no protocol; reuses `gpui_component`
  widgets (Tree, Table, Input) the project already renders; writes go straight
  through `set_component_property()`.
* **Mounting:** the `settings_view` feature is the exact template to clone — an
  `Entity` on the `ActiveProject` global, an action + keybinding (like
  `ctrl-shift-r`), and either a `/app/devtools` route, a docked side-panel in
  `AppLayout` (`workspace/layout.rs:37`), or a floating overlay layer in
  `Workspace::render` (`workspace/mod.rs:554`).
* **Cost:** moderate; most effort is UI. ~1–2 weeks for a solid v1.

## Option B — External client over a socket

A `nemo-devtools` crate running a server (Unix socket / WS / JSON-RPC) on the
tokio side, streaming state to a separate process (even a browser).

* **Pros:** decoupled; survives app crashes; scriptable.
* **Cons:** requires the `Serialize` derive on `BuiltComponent`, a wire protocol,
  a transport server (none exists), and a whole separate frontend; must marshal
  layout reads through the `!Send` runtime.
* **Cost:** high. 3–4+ weeks and a second UI codebase.

# Recommended shape: a `nemo-devtools` crate

Add `crates/nemo-devtools` as a **library crate** (like `nemo-layout`),
depending on `nemo-layout`, `nemo-data`, `nemo-events`, `nemo-registry`,
`nemo-config`, and `gpui`/`gpui-component`. It exposes a GPUI `Entity` view plus
a few small helpers. The `nemo` binary gains a `--devtools` flag (thread through
`Args` → `run_app` → `Workspace`, mirroring `watch`) and/or a keybinding to
toggle the panel.

v1 panels, in build order (each maps to an existing surface):

1. **Tree** — walk the `BuiltComponent` tree; select a node. *(read-only, cheapest)*
2. **Inspector** — for the selected node, render a schema-driven form from
   `ConfigSchema`/`PropertySchema`; write edits via `set_component_property()`.
   *(same idea as the roadmap's proposed `<property-inspector>`)*
3. **Data** — live tree of `data.*/state.*/var.*` + a change log from `subscribe()`.
4. **Events** — tail `EventBus::subscribe()` with the built-in filter bar.
5. **Bindings** — table from `list_bindings()`.

Supporting changes in core crates (all small): `#[derive(Serialize)]` on
`BuiltComponent`/`ActiveBinding` (only for JSON export), a
`DataRepository::snapshot()` accessor, a startup toggle to build the event bus
`with_tracing()`, and bumping the repository broadcast capacity above 100.

# Alignment with the roadmap

This overlaps two existing P2 items in the [roadmap](roadmap.md) — **"Interactive
property playground"** and the proposed **`<property-inspector>` component**. An
open design decision: build devtools as a **separate inspector panel** (Option A)
versus delivering it as **first-class `<property-inspector>` / `<component-tree>`
components** authors embed in their own apps. The component route is more
reusable and dogfoods the framework; the panel route is faster and cleaner as a
pure dev affordance. They share ~80% of the implementation.

# Open questions

* In-app panel vs. external client (Option A vs. B).
* Devtools chrome vs. embeddable components.
* Whether the property inspector should reuse or supersede the roadmap's
  `<property-inspector>` plan.
</content>
</invoke>
