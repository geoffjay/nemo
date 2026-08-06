---
type: Plan
title: Control-flow directives (`n:for`/`n:if`) in `.nemo` templates
description: Vue-style `n:for`/`n:if` namespaced attributes for iteration and conditionals in `.nemo` SFC templates. `n:if` and static `n:for` are compile-time; `n:for` over live data sources is a runtime list-binding expansion that creates/removes component instances.
tags: [config, sfc, directives, runtime, layout, planning]
timestamp: 2026-08-05T00:00:00Z
---

# Control-flow directives (`n:for`/`n:if`) in `.nemo` templates

Add iteration and conditionals to `.nemo` SFC templates using Vue-style
namespaced attributes. `n:if` toggles visibility via a binding; `n:for` repeats
its element — over static lists at compile time, and over live data sources at
runtime.

```nemo
<!-- n:if: conditionally show a panel -->
<panel n:if="data.api.status == 'error'">
  <label text="Something went wrong" />
</panel>

<!-- n:for over live data: one card per user -->
<stack n:for="user in data.api.users" n:key="user.id">
  <card>
    <label slot="header" text="${user.name}" />
    <text content="${user.email}" />
  </card>
</stack>

<!-- n:for over a static list (compile-time) -->
<tab-item n:for="tab in ['home', 'settings', 'about']" n:key="tab"
          label="${tab}" />
```

# Decisions (settled)

See [control-flow directives decision](../decisions/control-flow-directives.md).

* **Syntax: Vue-style `n:` namespace.** `n:for`/`n:if` are attributes on template
  elements. Parser-compatible today — `kebab_to_snake` (`xml_parser.rs:1715`)
  leaves colons untouched, and `process_component_element` (`:1402`) copies all
  non-internal attributes into the `Value` tree.
* **`n:if` is compile-time.** Emits a `bind-visible` binding to the `visible`
  property. Zero runtime changes.
* **`n:for` over static lists is compile-time.** Expands to N child `Value`
  nodes against a compile-time-known array. Zero runtime changes.
* **`n:for` over live data is runtime.** Creates/removes component instances as
  the array changes. Depends on [runtime component creation](runtime-component-creation.md).
* **Keying:** `n:key` (or `key`) on the loop element provides stable identity.
  Without it, the differ falls back to index matching.

# Why

XML cannot express iteration or conditionals. Today's workarounds:
* **Conditionals:** toggle `visible` manually via Rhai handlers or the
  page-switching pattern (pre-declare every possible component, toggle
  visibility). Scales poorly; state persists across toggles but the config is
  bloated.
* **Iteration over dynamic data:** only `<table>`/`<list>` handle dynamic arrays
  (via `bind-data`/`bind-items`). They are opaque container components — you
  can't compose arbitrary markup per item, bind nested properties, or mix
  component types within the iteration.

`n:for`/`n:if` make the template the unit of control flow: any component tree
can be conditionally rendered or repeated, with per-item bindings, using the
existing binding and component systems.

# Key decision: split compile-time and runtime evaluation

The three evaluation modes have different blast radii:

| Mode | What happens | Runtime changes | Depends on |
|------|-------------|----------------|------------|
| `n:if` | Compile to `bind-visible` | None | Binding system (exists) |
| `n:for` static | Expand to N `Value` nodes | None | Compiler (new) |
| `n:for` live data | Runtime list diff + insert/remove | `LayoutManager` + new `ListBindingManager` | [runtime-component-creation](runtime-component-creation.md) |

This split lets Phase 1 (`n:if`) and Phase 2 (static `n:for`) ship with zero
runtime changes — they are pure compiler features. Phase 3 (live-data `n:for`)
is the runtime architecture change, and it depends on
`runtime-component-creation` landing first.

# Phasing

## Phase 1 — `n:if` (compile-time)

**Status: planned.**

The compiler detects `n:if` on a `Value` node, extracts the condition expression,
and converts it to a `bind-visible` binding on that component. The node stays in
the tree; the binding toggles its `visible` property at runtime.

* New `compile_directives` pass in `nemo-config` (or the runtime's SFC compile
  step): walks the template `Value` tree, finds `n:if` attributes, evaluates
  the condition syntax, and replaces `n:if` with a `bind-visible` entry.
* Condition syntax: a source path (`data.api.status`), a comparison
  (`data.api.status == 'error'`), or a boolean expression. v1 supports source
  paths and `==`/`!=` comparisons against string/bool/number literals. Full
  expression evaluation is a follow-up.
* The binding target is `visible`; the binding source is the condition's left
  side. For comparisons, the binding evaluates the comparison at apply time —
  this may need a small extension to `BindingManager` to support "computed
  bindings" (a binding whose `apply` runs a predicate, not just a field
  extraction). Alternatively, compile to a Rhai handler that sets `visible` in
  response to `on-data-changed` — heavier but reuses the existing handler path.
  **Design note:** prefer the binding approach; add a `BindingMode::Computed` or
  a `transform` that can return a bool.
* **Verify:** a `.nemo` with `n:if` compiles to a `Value` tree with a
  `bind-visible` binding; toggling the source data hides/shows the component at
  runtime with no handler code.

## Phase 2 — `n:for` over static lists (compile-time)

**Status: planned.**

The compiler detects `n:for` on a `Value` node where the iteration source is a
compile-time-known array (a literal `['a', 'b', 'c']`, a `${var.list}` that
resolves to an array, or a `<props>` array prop). It expands the node into N
copies, each with the loop variable substituted into `${item}` placeholders,
and strips the `n:for` attribute.

* Extend `compile_directives`: detect `n:for`, classify the source as static
  or dynamic. A static source is a literal array, a resolved `${var.x}` array,
  or a prop with a known array default. A dynamic source is a `data.*` path.
* For static sources: expand the loop. Each copy gets a unique id (the loop
  element's id + `_<index>`, or `_<key>` when `n:key` is present). `${item.name}`
  inside the loop body is substituted with the literal value.
* The output is ordinary `Value` nodes — `parse_layout_config` and everything
  downstream is unchanged.
* **Verify:** a `.nemo` with `n:for` over a literal array compiles to N child
  nodes with correct id suffixing and `${item}` substitution; `nemo validate`
  passes on the expanded tree.

## Phase 3 — `n:for` over live data (runtime)

**Status: planned.** Depends on [runtime-component-creation](runtime-component-creation.md).

When `n:for`'s source is a `data.*` path, the compiler marks the node as a
**list container**: it extracts the loop template (the element's subtree with
`${item.*}` placeholders left intact), records the source path, and emits a
`ListBinding` spec instead of expanded children. At runtime, a
`ListBindingManager` watches the source path, diffs the array, and
creates/removes component instances.

### Compile-time: emit a `ListBinding` spec

* The compiler converts an `n:for="item in data.api.users"` node into:
  * a **list container** component (the parent that holds the repeated
    children — today the `n:for` element itself becomes the container), with
    `n:for`/`n:key` stripped and a `list_binding` metadata field recording
    `{ source: "data.api.users", item_var: "item", key: "user.id",
    template: <loop body Value> }`.
  * the loop body `Value` is stored as the template for per-instance expansion.
* `LayoutBuilder::build_node` needs a new arm for list-container nodes: it
  builds the container component (with no children initially) and registers
  the `ListBinding` with the new `ListBindingManager`.

### Runtime: `ListBindingManager`

* New `ListBindingManager` (in `nemo-layout/src/binding.rs` or a new module),
  owned by `LayoutManager`. Tracks active list bindings by source path.
* On `on_data_changed(source_path, value)`: if `source_path` matches a list
  binding's source, diff the new array against the current instances:
  * **Added items:** `insert_component` for each new item, expanding the loop
    template with the item's data. The per-instance `${item.name}` placeholders
    become per-instance bindings from `data.api.users[<index>].name` to the
    child's property.
  * **Removed items:** `remove_component` (recursive) for each removed item.
  * **Persisted items:** update per-instance bindings (or recreate if key
    changed).
* Keying: when `n:key` is present, match items by key value (stable identity).
  Without a key, match by index (destroy/recreate on reorder).
* The `apply_pending_data_updates` loop (`runtime.rs:877`) gains a call to
  `list_bindings.on_data_changed` after the existing `bindings.on_data_changed`
  pass, so structural changes happen before re-render.

### Reusing `runtime-component-creation`

The `insert_component`/`remove_component` from
[runtime-component-creation](runtime-component-creation.md) are the primitives
the list expander calls. That plan already established:
* `BuiltComponent` is fully dynamic (`HashMap<String, Value>` properties, flat
  ID-keyed storage).
* `LayoutManager` is `Arc<RwLock>` and mutated incrementally.
* The render path snapshots under read lock before rendering.
* State is lazy and ID-keyed — dynamically inserted widgets allocate state on
  first render.
* Re-render trigger: existing `data_dirty` + `data_notify` + `cx.notify()`.

The list expander is a new *caller* of those primitives — it automates what
the `runtime-component-creation` plan exposes manually via Rhai.

### Per-instance bindings

When the list expander creates an instance for `data.api.users[3]`, the
`${item.name}` placeholders in the loop template become bindings:
`bind-label: data.api.users[3].name`. The compiler pre-computes the binding
template (which placeholders map to which child properties); the expander
instantiates it per index. This reuses `BindingManager::bind` — no new
binding mode, just per-index source paths.

### Edge cases to test

* **Empty array:** the container renders with zero children (not an error).
* **Array grows:** new components appear with correct bindings.
* **Array shrinks:** removed components are cleaned up (children, bindings,
  state — matching `runtime-component-creation`'s teardown).
* **Reorder with key:** persisted instances keep their state (an input inside
  the loop keeps its focus/caret).
* **Reorder without key:** instances are destroyed and recreated (state lost).
* **Nested `n:for`:** an `n:for` inside an `n:for` (e.g. rows of columns) —
  the inner list binding is registered when the outer instance is created.
* **`n:for` + `n:if` on the same node:** `n:for` wins (the loop template
  carries the `n:if`, evaluated per instance).
* **Rapid updates:** the list diff is idempotent and debounced (the existing
  `data_dirty` flag already debounces).

# Critical files

| File | Role |
|---|---|
| `crates/nemo-config/src/xml_parser.rs` | `compile_directives` pass (new) — walks template `Value` tree, expands `n:if`/static `n:for`, emits `ListBinding` specs for live-data `n:for` |
| `crates/nemo-layout/src/binding.rs` | `ListBindingManager` (new) — watches source paths, diffs arrays, emits insert/remove/update ops |
| `crates/nemo-layout/src/manager.rs` | `insert_component`/`remove_component` (from runtime-component-creation), `list_bindings` field, `on_data_changed` extension for list paths |
| `crates/nemo-layout/src/builder.rs` | `build_node` arm for list-container nodes — registers `ListBinding` |
| `crates/nemo/src/runtime.rs` | `apply_pending_data_updates` — call `list_bindings.on_data_changed` after binding updates |
| `crates/nemo/src/commands/validate.rs` | Linter: skip `n:`-prefixed attributes in `unknown-attribute` |

# Reuse (avoid new code)

* `BindingManager::bind` / `on_data_changed` / `apply_updates` — per-instance
  bindings reuse the existing binding system.
* `insert_component`/`remove_component` (from runtime-component-creation) —
  the list expander calls them; it doesn't reimplement insertion.
* `data_dirty` / `data_notify` / `cx.notify()` re-render loop — unchanged.
* `process_component_element` already preserves `n:for`/`n:if` as attributes —
  no parser change for the syntax.
* SFC `vars`/`${prop}` interpolation (`interpolate_variables`, `runtime.rs`) —
  the loop variable `item` uses the same `${}` runtime interpolation path,
  fed by per-instance bindings instead of SFC props.

# Verification

* **Phase 1 (`n:if`):** a `.nemo` with `n:if="data.api.status == 'error'"`
  compiles to a `bind-visible` binding; toggling the source data hides/shows
  the component at runtime. No handler code.
* **Phase 2 (static `n:for`):** a `.nemo` with `n:for` over `['a','b','c']`
  compiles to 3 child nodes with correct id suffixing and `${item}`
  substitution; `nemo validate --strict` passes on the expanded tree.
* **Phase 3 (live-data `n:for`):**
  * Unit: `ListBindingManager` diff — grow, shrink, reorder with/without key,
    nested loops, empty array.
  * Integration: a `.nemo` with `n:for="user in data.api.users"` backed by a
    mock HTTP source; adding an item to the source creates a new component;
    removing one cleans it up; reordering with `n:key` preserves input state.
  * E2e: `nemo dev` on a project with a live-data `n:for`; the list updates in
    real time as the data source changes.

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — document `n:for`/`n:if` in
  the SFC section.
* [Single-file components](../patterns/single-file-components.md) — add a
  "Control-flow directives" section with `n:for`/`n:if`/`n:key` examples.
* [Data flow](../concepts/data-flow.md) — note list bindings alongside
  scalar bindings.
* This plan — mark phases as implemented.
* The `nemo-xml-reference` skill — add `n:for`/`n:if`/`n:key` documentation.

# Relationship to other plans

* **Depends on** [raw-text `.nemo` parser](sfc-raw-text-parser.md) — directives
  are authored in `.nemo` files, which need raw-text blocks to be comfortable.
* **Depends on** [runtime component creation](runtime-component-creation.md)
  — Phase 3 (live-data `n:for`) calls `insert_component`/`remove_component`.
* **Independent of** the [build system](build-system.md) — directives are a
  template feature, orthogonal to build/load.
* **Feeds into** [app.nemo SFC entry](app-nemo-sfc-entry.md) — `app.nemo` uses
  the same directive syntax in its `<template>`.