---
type: Pattern
title: Routing
description: The chrome-free page router — host-side router state, deferred navigation, param projection, and lifecycle hooks.
tags: [routing, navigation, containers, rhai, data-flow]
timestamp: 2026-07-20T00:00:00Z
---

The **page router** is a chrome-free switching primitive: a `<router>` mounts
exactly one of its `<route>` children — the one whose `path` pattern matches the
router's current path — and nothing else. It replaces the fragile
visibility-toggle pattern (per-page `<panel visible>` + a hand-maintained
hide-all Rhai handler). It lives in `crates/nemo/src/containers/router.rs`
alongside [`app_shell`](containers.md), but draws no decoration of its own.

```xml
<nav-link router="main" route="/users/42" label="User 42"/>

<router id="main" default="/home" primary="true">
  <route path="/home"> … </route>
  <route path="/users/:id" on-enter="load_user" on-leave="save_scroll"> … </route>
  <route path="*"> <!-- not-found fallback --> </route>
</router>
```

```rhai
navigate("/users/42");          // primary router
navigate("main", "/users/42");  // explicit router
back(); forward();
let id = get_data("route.main.params.id");   // "42"
```

```sh
# Start a router somewhere other than its `default` (this launch only):
nemo dev --route /table examples/components/app.xml   # primary router
nemo dev --route settings=/advanced app.xml           # explicit router id
nemo screenshot --app-config app.xml --route /charts --out out.png
```

# State model

Router state is **host-side and authoritative**, in a `router_states` registry
on `NemoRuntime` (`Arc<RwLock<HashMap<String /*router id*/, RouterState>>>`)
keyed by router id. `RouterState` holds `{ history, index, params }`; the current
path is `history[index]`. `back`/`forward` move `index`; a `navigate` truncates
forward history and pushes. It is host-side (not `ComponentStates`, which `App`
owns and Rhai can't reach) because navigation must be drivable from **both**
declarative `<nav-link>` clicks **and** Rhai `navigate()`.

On each applied navigation the current path + params are **projected into the
`DataRepository`** at `data.route.<id>.path` and `data.route.<id>.params.*`, so
pages read them via `get_data("route.<id>.params.id")` and `<binding>`s can
target `data.route.<id>.params.*`. The params subtree is replaced with one
`repo.set` of a fresh object (not `delete` + per-key set — `delete` leaves a
`Null` tombstone that breaks the nested set), which also clears stale params
from the previous route.

# Deferred navigation (avoids a re-entrancy deadlock)

`NemoRuntime::call_handler` holds the `extension_manager` **write** lock while a
Rhai handler runs. If `navigate()` — called from inside a handler —
synchronously fired `on-enter`/`on-leave` via `call_handler`, it would
re-acquire that lock on the same thread and **deadlock**. So navigation goes
through a **deferred queue**, mirroring the `plugin_dirty_paths` reactivity path:

1. `navigate()`/`back()`/`forward()` and a `<nav-link>` click **enqueue a
   `NavIntent`** (`RuntimeContext`/`NemoRuntime::enqueue_navigation`), set
   `data_dirty`, and `notify_one()` — they never mutate router state or fire
   hooks directly.
2. The `App` poll loop calls `NemoRuntime::apply_pending_navigations()` **before**
   `apply_pending_data_updates()` (so freshly-flagged route paths propagate
   through bindings in the same wake). Running **outside** the extension lock, it
   matches the target against the router's `<route>` patterns, updates
   `RouterState`, projects path+params, fires `on-leave` on the old route then
   `on-enter` on the new route via `call_handler`, and returns `true` so the loop
   re-renders.

`on-enter`/`on-leave` are parsed as normal handlers (kebab→snake `on_*` →
`handlers["enter"]`/`handlers["leave"]`) and fire with `(router_id, "enter" |
"leave")`. They fire only on an actual path change.

# Path matching

`match_route(pattern, path)` (pure, in `router.rs`) splits both on `/`: a
`:name` segment captures a param, a `*` segment matches the remainder, literal
segments must match exactly. `resolve_route(patterns, path)` scans in **document
order**, first match wins; a `path="*"` last is the not-found fallback.

# Render (only the active route mounts)

In `App::render_component`: the `"router"` arm reads the current path (lazily
initialized to `default` via `router_current_path`), resolves the active route,
projects via `sync_route_projection` (a guarded no-op on steady-state
re-renders, so it never loops), and renders **only that route's body** with
`render_children`. The `Router` view renders like the generic container
(`div().flex().flex_col()`, **content-sized** — *not* `size_full`), so it stays
transparent to the surrounding layout: growth and scrolling come from the
`<router>`'s own `flex`/`scroll`/height attributes via `apply_layout_styles`.
Forcing `size_full` would pin the body to the parent's height and defeat an
enclosing `scroll` stack — so to scroll a tall page, wrap the router in a
`scroll` container and leave the router itself content-sized (no `flex`), as
`examples/components` does. Nested routers fall out for free — a `<router>` inside a route
body is reached by the recursion when that route is active. (This "for free" is
the `<router>`-in-`<route>` case only. A `<router>` nested inside an *SFC
`<template>`* is a different, **unhandled** case — its id is scoped per instance
but nav-link `router=` / Rhai `navigate()` targets are not yet resolved to the
scoped id; see [Scope nested routers inside SFCs](../plans/router-in-sfc-scoping.md).) `"nav_link"` reads
`route`/`router`/`label`, computes `is_active` against the router's current path
(a non-initializing peek), and on click enqueues a `NavIntent`. `"route"` is a
no-op marker rendered by its parent router. Registered in
`nemo-registry::builtins` (`register_container_components`). Example:
`examples/router/`.

# Launch override (`--route`)

`--route <path>` (or `--route <router-id>=<path>`) starts a router somewhere
other than its `default`, for that launch only — no config edit. It is a flag on
the run path (`nemo`/`nemo dev`) and on `nemo screenshot` (handy for capturing a
specific page). The CLI value flows `Args`/`DevArgs`/`ScreenshotArgs` →
`WorkspaceArgs.initial_route` → `create_runtime` → `NemoRuntime::set_initial_route`,
which records an `InitialRoute { router, path }` before the first render.
`router_current_path` consults it once on lazy init (`initial_path_for`): an
explicit id matches directly; an unscoped override targets the primary router
(`primary_router_id`). It is **not** reapplied on hot-reload or when opening a
different project via the loader — both pass `None` — so it stays a launch-time
override, equivalent to temporarily changing that router's `default`.
