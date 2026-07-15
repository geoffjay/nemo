---
type: Plan
title: Runtime component creation
description: Let handlers/scripts create and remove built-in component instances at runtime, via Rhai and PluginContext.
tags: [components, runtime, rhai, plugins, layout]
timestamp: 2026-07-15T00:00:00Z
---

# Runtime component creation

Let handlers and scripts create and remove *built-in* component instances at
runtime, via Rhai and `PluginContext`. Today the component tree is built once
from XML at startup (`apply_layout_from_config`, `runtime.rs:507`) and the only
live-UI mutation is `set_component_property` (`runtime.rs:1773`) — toggling
pre-declared components. Apps that need structurally dynamic UI (form builders,
list-of-records editors, conditionally-composed dashboards) must pre-declare
every possible component and toggle `visible`, which scales poorly.

## Scope

**In scope:** Layer 1 — runtime insertion and removal of *built-in* component
instances via Rhai and `PluginContext`. Static properties only (no `<binding>`
support). Full lifecycle: create + remove + bulk property update.

**Out of scope (future work, recorded at the end):**

* **Layer 2** — plugin-declared new component types with declarative render
  callbacks (parameterized templates rendered by the host).
* **Layer 3** — a native FFI render ABI for plugins that draw their own pixels.
* Runtime `<binding>` setup on dynamically created components.

## Why this is viable

The architecture is highly amenable to runtime component creation:

* `BuiltComponent` (`crates/nemo-layout/src/manager.rs:29`) is fully dynamic:
  string-keyed `HashMap<String, Value>` properties, flat ID-keyed storage. No
  per-type struct — a dynamic instance is just another map entry.
* `LayoutManager` is `Arc<RwLock<LayoutManager>>` and is already mutated
  incrementally by `apply_updates` / `set_property` (`manager.rs:175`, `:186`).
  An `insert_component` is structurally trivial: insert into `components`, push
  the child ID onto the parent's `children`.
* The render path snapshots the map under a read lock and releases it before
  rendering (`crates/nemo/src/app.rs:349-358`), so mutations between renders are
  safe; no render-time locking concerns.
* State is lazy and ID-keyed (`crates/nemo/src/components/state.rs:83`): a
  dynamically-inserted stateful widget (Input, Slider) allocates its `Entity<T>`
  on first render via the existing `get_or_create_*` helpers — no special
  handling, as long as the component has a stable unique ID.
* `render_children` (`app.rs:398`) already tolerates missing IDs via
  `filter_map`, so partial/dynamic subtrees degrade gracefully.

## The gap this plan closes

The component system is closed at the render layer but open by design at the
registry layer. Four layers, only one wired end-to-end today:

| Layer | State | Extensible? |
|---|---|---|
| Schema registry (`ComponentRegistry`, `ComponentDescriptor`) | Built; `DescriptorSource::Plugin` + `ComponentFactory` trait (`descriptor.rs:194`) exist but are **dead code** | In principle, yes |
| Plugin declaration (`PluginRegistrar::register_component`) | Built; collects `ComponentSchema` into `PluginInitResult.components` | `ExtensionManager::init_plugins` (`nemo-extension/src/lib.rs:244`) **silently drops them** |
| Build pipeline (`LayoutBuilder::build_node`) | Checks `has_component()` — rejects unknown types (`builder.rs:50`) | Gate exists; no factory invocation |
| Render dispatch (`app.rs:render_component`) | Monolithic 50+ arm `match`; `_` → plain `div` (`app.rs:1031`) | **Hard blocker — no trait, no callback, no registry lookup** |

This plan does **not** open the render dispatch (that's Layer 2). It only
inserts/removes instances of types that already have a match arm — built-in
components — which is the immediately useful capability and requires no changes
to `render_component` or the `NemoComponent` macro.

## Design

### API (Rhai + PluginContext)

Expose four functions to Rhai (`crates/nemo-extension/src/rhai_engine.rs:~451`
registration) and `PluginContext` (`crates/nemo-plugin-api/src/lib.rs:~310`):

```rhai
create_component(parent_id, type, props)            // returns generated id
create_component_with_id(parent_id, id, type, props)
update_component(id, props)                          // bulk property set (reuses set_property)
remove_component(id)                                  // recursive child + binding + state teardown
```

`props` is a Rhai `Map` / `PluginValue::Object`, converted to
`HashMap<String, Value>` via the existing `plugin_value_to_config_value`.

### LayoutManager additions (`crates/nemo-layout/src/manager.rs`)

```
insert_component(&mut self, id, component_type, parent: Option<String>,
                 properties, handlers) -> Result<(), LayoutError>
  // validate type via registry.has_component (gate: rejects unknown types,
  //   mirroring build_node at builder.rs:50)
  // if parent given, push id onto parent.children; else treat as new root
  // insert BuiltComponent into self.components

remove_component(&mut self, id: &str)
  // recursive: remove children first (walk child ids, recurse)
  // remove bindings targeting this id (BindingManager::remove_for_component — new method)
  // remove ComponentStates entry for this id (see "State cleanup" below)
  // remove from self.components
```

**Validation:** `insert_component` must call `self.builder.registry.has_component()`
to reject unknown types (same gate as `build_node`, `builder.rs:50`). Required-
property validation is skipped for runtime insertion — props arrive
programmatically, not from XML, and partial initialization is a legitimate use
case (set defaults now, update via bindings later).

### Binding cleanup

`BindingManager` (`crates/nemo-layout/src/binding.rs`) currently has no
`remove_for_component`. Add one: scan bindings, drop any whose
`target.component_id == id`. Bindings are indexed by source path, so a linear
scan over a small set is fine for v1.

### State cleanup

`ComponentStates` (`crates/nemo/src/components/state.rs:83`) is owned by `App`,
not `LayoutManager`, so `LayoutManager::remove_component` cannot tear it down
directly. Two options:

1. **Lazy leak (v1)** — leave orphaned state entries. They are keyed by ID and
   harmless (never re-rendered). Simplest. Risk: unbounded growth for
   create/remove churn. Acceptable for v1.
2. **Notify App (follow-up)** — `remove_component` emits a signal the `App`
   async task drains on its next tick, calling `ComponentStates::remove(id)`.

**Recommend option 1 for v1** with a follow-up to wire option 2. State entries
are tiny; churn is bounded by app behavior.

### Re-render trigger

After `insert_component` / `remove_component`, the caller must trigger a
re-render. Reuse the existing data-reactivity path: `RuntimeContext` sets
`data_dirty` and calls `data_notify.notify_one()` (same pattern as `set_data`
at `runtime.rs:1735-1736`). The `App` async task wakes, snapshots
`LayoutManager`, and `cx.notify()` re-renders. No new wake-up mechanism needed.

### Concurrency

`RuntimeContext` already takes `layout_manager.try_write()` for
`set_component_property` (`runtime.rs:1779`). `insert_component` /
`remove_component` use the same pattern. If the write lock is contended (render
is holding a read lock), return `PluginError::InvalidConfig("layout busy,
retry")` — consistent with existing behavior.

### Threading

`NemoRuntime` is `!Send`/`!Sync` (see
[Architecture](../concepts/architecture.md)); `LayoutManager` mutation happens
on the main/UI thread via `RuntimeContext`. Plugin/WASM calls arrive via
`RuntimeContext` (which is `Send + Sync`) but mutate `LayoutManager` through the
`Arc<RwLock>`. This matches the existing `set_component_property` contract — no
new threading model.

## Implementation steps

1. **`LayoutManager::insert_component` / `remove_component`**
   (`nemo-layout/src/manager.rs`). Unit tests mirroring `test_apply_simple_layout`
   but using insert; test recursive remove; test unknown-type rejection.
2. **`BindingManager::remove_for_component`** (`nemo-layout/src/binding.rs`).
   Unit test.
3. **`RuntimeContext` methods** (`nemo/src/runtime.rs:~1773`):
   `create_component`, `create_component_with_id`, `update_component`,
   `remove_component`. Each takes the `try_write` lock and delegates to
   `LayoutManager`, then sets `data_dirty` + `data_notify.notify_one()`.
4. **`PluginContext` trait** (`nemo-plugin-api/src/lib.rs:~310`): add the four
   methods with default `Err(PluginError::Unsupported)` impls so existing plugin
   SDKs don't break.
5. **Rhai registration** (`nemo-extension/src/rhai_engine.rs:~451`): register
   the four as Rhai functions taking `&mut PluginContext`. Convert Rhai maps to
   `PluginValue::Object`.
6. **WASM host** (`nemo-wasm/src/host_impl.rs:~74`): add host functions
   mirroring `set_component_property`.
7. **Tests:**
   * Unit: `LayoutManager` insert/remove (no parent, with parent, recursive
     remove, unknown type rejected).
   * Integration (in `runtime.rs` tests near
     `test_set_component_property_nonexistent_component` at `:3446`):
     `RuntimeContext::create_component` then assert the component is present in
     the layout snapshot; `remove_component` cleans up.
   * Rhai end-to-end: a script that calls `create_component` on a click handler,
     assert the new component appears in the layout snapshot.

## Risks / open questions

* **ID generation.** `create_component` without an explicit id needs a unique
  generator. Use a monotonic counter on `LayoutManager` (prefix `__dyn_N`) to
  avoid colliding with `__anon_N` from the parser (`xml_parser.rs:676`). IDs
  must remain document-wide unique (already an invariant — see `log.md`
  2026-07-15 anonymous-id fix).
* **Removing the root or a currently-rendering component.** Removing the root
  would blank the app. Guard: refuse to remove the component returned by
  `root_id()`. Removing a component mid-render is safe because render snapshots
  first (`app.rs:349-358`); the removal takes effect next frame.
* **State leak** — discussed above; acceptable for v1.
* **No `<binding>` support** — by scope decision. A dynamic component that
  needs reactive data uses an explicit Rhai handler calling
  `set_component_property` in response to events, or sets `props` at creation
  time. Document this limitation in the XML reference skill.
* **`ExtensionManager::init_plugins` dropping plugin component schemas**
  (`nemo-extension/src/lib.rs:244`) — out of scope, but noted here as the Layer 2
  entry point so a future effort doesn't re-derive the gap.

## Future work (not in scope, recorded here)

* **Layer 2 — plugin-declared component types.** Bridge
  `nemo-plugin-api::ComponentSchema` → `nemo_config::ConfigSchema`, register
  into `ComponentRegistry` with `DescriptorSource::Plugin`, and add a
  render-callback registry (`HashMap<String, Arc<dyn Fn(&BuiltComponent,
  &Runtime) -> Value>>`) consulted in the `_` arm of `render_component`
  (`app.rs:1031`). Callbacks return a declarative `Value` tree of built-in types
  that the host recursively renders — essentially first-class parameterized
  templates, with no new FFI render ABI. The dead `ComponentFactory` trait
  (`descriptor.rs:194`) and the `init_result.components` drop site
  (`nemo-extension/src/lib.rs:244`) are the entry points. Enables plugins/Rhai
  to define new component types as parameterized templates.
* **Layer 3 — native custom rendering.** A new FFI render ABI for plugins that
  draw their own pixels (canvas, custom charts). Hardest; deferred.
* **Runtime bindings.** Incremental `<binding>` setup on dynamically created
  components, so a dynamic component can react to data sources without an
  explicit handler. Requires `BindingManager::bind_for_component` and re-running
  `on_data_changed` for affected paths.

## Knowledgebase updates required when implemented

* [Components](../concepts/components.md) — add a "Runtime component creation"
  section.
* [Data flow](../concepts/data-flow.md) — note `create_component` as a re-render
  trigger alongside data updates.
* A new [pattern](../patterns/index.md) "Runtime component creation" capturing
  the API surface + the ID-generation + state-leak caveats.
* [Roadmap](roadmap.md) — move this item out of "remaining" once landed.