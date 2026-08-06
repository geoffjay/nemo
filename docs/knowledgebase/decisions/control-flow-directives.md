---
type: Decision
title: Control-flow directives use Vue-style `n:for`/`n:if`; `n:for` evaluates over live data
description: `.nemo` templates use `n:for`/`n:if` namespaced attributes for iteration and conditionals. `n:if` is compile-time (binds to `visible`); `n:for` over live data sources is a runtime list-binding expansion that creates/removes component instances.
tags: [config, sfc, directives, runtime, decision]
timestamp: 2026-08-05T00:00:00Z
---

# Decision

`.nemo` templates support **Vue-style namespaced attributes** `n:for` and `n:if`
for iteration and conditionals. The evaluation model splits by directive:

* **`n:if` is compile-time.** The compiler emits a `bind-visible` binding from
  the condition source to the component's `visible` property. Zero runtime
  changes — this reuses the existing binding system, the same mechanism the
  page-switching pattern uses with `visible="true"/"false"`.
* **`n:for` over static/config-time lists is compile-time.** The compiler
  expands the loop against a compile-time-known array (props, `${var.x}`),
  emitting N child `Value` nodes. Zero runtime changes.
* **`n:for` over live data sources is runtime.** When the iteration source is a
  data path (`data.api.users`), the runtime creates and removes component
  instances as the array changes. This requires the runtime component creation
  capability (`insert_component`/`remove_component` in `LayoutManager`,
  [runtime-component-creation](../plans/runtime-component-creation.md)) and a
  list-binding diff layer.

# Context

XML cannot express iteration or conditionals. Nemo's existing workarounds are
limited: `visible` toggling for conditionals (scales poorly), and `<table>`/
`<list>` for dynamic arrays (opaque, not composable). The user wants `n:for` to
iterate over live data (HTTP/WebSocket/MQTT sources), not just static config —
this is the capability that makes SFC templates useful for data-driven UIs.

The directive syntax is **parser-compatible today**: `kebab_to_snake`
(`xml_parser.rs:1715`) only replaces `-` with `_`, so `n:for` survives as the
attribute key `n:for` (colon untouched). `process_component_element`
(`:1402`) copies all non-internal attributes into the `Value` tree, so
`n:for`/`n:if` appear on component nodes as regular properties. The compiler
finds and expands them. No parser changes needed for the syntax itself.

# Consequences

* `n:for`/`n:if` use the `n:` namespace prefix. The linter's `unknown-attribute`
  check must skip `n:`-prefixed attributes (they are directives, not component
  properties). The schema export should document them.
* `n:if` compiles to a one-way binding to `visible`. A node with `n:if` still
  exists in the component tree (it's just hidden when the condition is false),
  so its state persists across toggles. This matches today's `visible` behavior.
* `n:for` over live data creates a **runtime dependency on
  `runtime-component-creation`** — `LayoutManager::insert_component`/
  `remove_component` must exist before live-data `n:for` works. Static `n:for`
  and `n:if` do not have this dependency.
* `n:for` needs a **keying mechanism** (Vue's `:key`) for stable identity across
  array updates. `n:key="item.id"` or `key="item.id"` on the loop element lets
  the list differ match persisted items instead of using index-based matching.
  Without a key, reorder operations destroy and recreate components (correct
  but inefficient, and breaks stateful widgets like inputs inside the loop).
* The iteration variable (`item` in `n:for="item in data.api.users"`) is exposed
  to children via per-instance bindings: the compiler rewrites `${item.name}`
  inside the loop body to a binding from `data.api.users[<index>].name` to the
  child's property. The list expander creates these per-index bindings when it
  creates each component instance. This reuses the existing binding system.
* See the [control-flow directives plan](../plans/control-flow-directives.md).