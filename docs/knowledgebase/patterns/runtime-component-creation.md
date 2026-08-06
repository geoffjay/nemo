---
type: Pattern
title: Runtime component creation
description: Create and remove built-in component instances at runtime via Rhai and PluginContext.
tags: [components, runtime, rhai, plugins, layout]
timestamp: 2026-08-04T00:00:00Z
---

# Runtime component creation

Handlers and scripts can create and remove *built-in* component instances at
runtime — structural UI mutation, not just property toggling on pre-declared
components. This enables form builders, list-of-records editors, and
conditionally-composed dashboards without pre-declaring every possible
component and toggling `visible`.

## API surface

Four functions are exposed to Rhai (`nemo-extension/src/rhai_engine.rs
register_context`) and `PluginContext` (`nemo-plugin-api/src/lib.rs`):

```rhai
create_component(parent_id, type, props)            // returns generated __dyn_N id
create_component_with_id(parent_id, id, type, props)
update_component(id, props)                          // bulk property set
remove_component(id)                                  // recursive subtree + binding teardown
```

`props` is a Rhai `Map` / `PluginValue::Object`, converted to
`HashMap<String, Value>` via `plugin_value_to_config_value`. A `"handlers"`
sub-object (event name → handler string) is extracted and stripped from
properties before insertion.

## How it works

`LayoutManager::insert_component` (`nemo-layout/src/manager.rs`):

1. Validates the component type via `LayoutBuilder::has_component_type` (same
   gate as `build_node` — rejects unknown types).
2. Validates the parent exists (when given).
3. Rejects duplicate IDs (document-wide uniqueness invariant).
4. Inserts a `BuiltComponent` and pushes the child ID onto the parent's
   `children` list.

`LayoutManager::remove_component`:

1. Refuses to remove the root (the component returned by `root_id()`).
2. Collects the subtree breadth-first (component + all descendants).
3. Detaches from the parent's `children` list.
4. Removes bindings for every subtree member via
   `BindingManager::unbind_component`.
5. Removes all subtree components from the `components` map.

`RuntimeContext` (`nemo/src/runtime.rs`) implements each method by taking the
`LayoutManager` `try_write()` lock, delegating, then setting `data_dirty` +
`data_notify.notify_one()` — the same re-render path as `set_component_property`.

## ID generation

`create_component` without an explicit ID uses `LayoutManager::generate_dynamic_id`
— a monotonic counter with prefix `__dyn_N`. This avoids colliding with the
parser's `__anon_N` anonymous IDs (`xml_parser.rs`). IDs are document-wide
unique.

## Caveats

* **No `<binding>` support.** A dynamically created component cannot react to
  data sources via `<binding>`. Use an explicit Rhai handler calling
  `set_component_property` in response to events, or set props at creation time.
* **State leak (v1).** `ComponentStates` (owned by `App`, not `LayoutManager`)
  is not cleaned up on removal. State entries are keyed by ID and harmless (never
  re-rendered), but unbounded under heavy create/remove churn. A follow-up
  would emit a signal the `App` task drains.
* **Removing mid-render is safe.** The render path snapshots the component map
  under a read lock and releases it before rendering (`app.rs`); the removal
  takes effect next frame.
* **Lock contention.** If the render path holds the read lock, `try_write()`
  fails and the caller gets `PluginError::ComponentFailed("Layout manager is
  locked")` — consistent with `set_component_property`.

## WASM

The same four functions are exposed as WIT host functions
(`nemo-wasm-guest/wit/nemo-plugin.wit`) and implemented in
`nemo-wasm/src/host_impl.rs`, delegating to `PluginContext`.

## Example

A click handler creates a label under the root; a second handler removes it:

```rhai
let created_id = "";

fn on_add_click(component_id, event_data) {
    let props = #{text: "Created at runtime"};
    created_id = create_component("root", "label", props);
}

fn on_remove_click(component_id, event_data) {
    if created_id != "" {
        remove_component(created_id);
    }
}
```

See `examples/sfc` for a working demo.

## Plan

Full design rationale and future work (Layer 2 plugin-declared types, Layer 3
native rendering, runtime bindings) are in
[the plan](../plans/runtime-component-creation.md).