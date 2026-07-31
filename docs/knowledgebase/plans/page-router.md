---
type: Plan
title: Page router
description: A general chrome-free <router>/<route> primitive with URL-path routes, params, history, lifecycle hooks, nested routers, and a Rhai navigate() API — replacing the fragile visibility-toggle page-switching pattern.
tags: [routing, navigation, components, containers, rhai, layout]
timestamp: 2026-07-19T00:00:00Z
---

# Page router

Today a nemo app changes "pages" by giving every page a `<panel visible="…">` and
writing a Rhai handler that hides every page then shows one:

```rhai
fn on_nav(component_id, event_data) {
    set_component_property("page_dashboard", "visible", false);
    set_component_property("page_monitoring", "visible", false);
    set_component_property("page_settings", "visible", false);
    let page_id = replace(component_id, "nav_", "page_");   // string-coupled ids
    set_component_property(page_id, "visible", true);
}
```

This is fragile and scales poorly:

* **No single source of truth** — the active page is implicit across scattered
  `visible` booleans.
* **O(n) hand-maintained handlers** — `examples/components/scripts/handlers.rhai`
  hard-codes hide-calls for **34** pages; adding a page means editing the list, and any
  omission leaves a stale page visible.
* **String-coupled ids** — navigation relies on the `nav_*` → `page_*` convention.
* **Hidden pages are still built and traversed** every frame (collapsed to an empty div
  by `panel.rs`), not unmounted.

The `app-shell` container (`crates/nemo/src/containers/app_shell.rs`) already solves this
*for its own chrome*: clicking `<sidenav-item target="X">` selects `<page id="X">` via a
shared `ComponentState::SelectedValue` and renders only the active page. But it is welded
to a fixed frame (sidenav + content + footer) and its active-page state lives in
`ComponentStates`, which **Rhai cannot reach**. This plan extracts that switching idea
into a general, chrome-free primitive. `app-shell` is left unchanged and may adopt the
router later.

## Scope

**In scope (v1):** URL-style path routes with params, declarative `<nav-link>`s, a Rhai
`navigate()` / `back()` / `forward()` API, navigation history, route `on-enter`/`on-leave`
lifecycle hooks, nested routers, and a not-found fallback route.

**Out of scope:** refactoring `app-shell` onto the router; deep-linking / persisting the
route across restarts; route guards/redirects (a lifecycle hook can call `navigate`
manually for now).

## XML surface

```xml
<router id="main" default="/home">
  <route path="/home">          <!-- body --> </route>
  <route path="/users/:id" on-enter="load_user" on-leave="save_scroll"> … </route>
  <route path="*">              <!-- not-found fallback --> </route>
</router>

<!-- declarative navigation, no handler needed -->
<nav-link router="main" route="/users/42" label="User 42"/>
```

```rhai
navigate("/users/42");            // primary router
navigate("main", "/users/42");    // explicit router
back(); forward();                // history
let id = get_data("route.main.params.id");   // "42"
```

## State model (the key decision)

Router state is **host-side and authoritative**, in a new `RouterRegistry` on
`NemoRuntime`, keyed by router id:

```rust
struct RouterState { history: Vec<String>, index: usize, params: HashMap<String, String> }
// RouterRegistry: Arc<RwLock<HashMap<String /*router id*/, RouterState>>>
```

* **Current path** = `history[index]`.
* On each applied navigation the current path + params are **projected into the
  `DataRepository`** at `data.route.<id>.path` and `data.route.<id>.params.*`, so pages
  read them via `get_data("route.<id>.params.id")` and `<binding>`s can target them. This
  reuses the existing `set_data` notify/binding machinery for free.
* History is host-side only (not Rhai-visible); `back()`/`forward()` move `index`.

Why host-side rather than `ComponentStates` (as `app-shell` does)? Navigation must be
drivable from **both** declarative clicks **and** Rhai `navigate()`, and `ComponentStates`
(owned by `App`) is unreachable from Rhai. The `DataRepository` projection gives Rhai read
access; the `RouterRegistry` holds history and a single re-entrancy-safe apply point.

## Navigation is deferred (critical — avoids a deadlock)

`NemoRuntime::call_handler` holds the `extension_manager` **write** lock while a Rhai
handler runs (`runtime.rs:508-511`). If `navigate()` — called from inside a handler —
synchronously fired `on-enter`/`on-leave` via `call_handler`, it would re-acquire that
write lock on the same thread and **deadlock**.

So navigation goes through a **deferred queue**, mirroring the existing
`plugin_dirty_paths` → `apply_pending_data_updates` reactivity path:

1. `navigate()` / `back()` / `forward()` / a `nav-link` click **enqueue a `NavIntent`**,
   set `data_dirty`, and call `data_notify.notify_one()` — they never mutate router state
   or fire hooks directly.
2. The `App` async poll loop (`crates/nemo/src/app.rs:55`) calls a new
   `NemoRuntime::apply_pending_navigations()` alongside `apply_pending_data_updates`.
   Running **outside** the extension lock, it matches the target path against the router's
   `<route>` patterns, updates `RouterState` (history/index/params), projects path+params
   into the `DataRepository`, fires `on-leave` on the old route then `on-enter` on the new
   route via `call_handler`, and returns `true` so the loop `cx.notify()`s a re-render.

This is the architecture the codebase already uses for data updates, so it is
cross-thread-safe (`RuntimeContext` is `Send + Sync`) and needs no new wake mechanism.

## Path matching

A small pure function in the new container module:

```rust
/// Split both on '/'. ':name' captures a param; a trailing '*' matches the rest;
/// literal segments must match exactly. Returns captured params on match.
fn match_route(pattern: &str, path: &str) -> Option<HashMap<String, String>>
```

The router render arm iterates `<route>` children **in document order**; first match
wins. `path="*"` (or no match) selects the fallback route. Unit-tested in the style of
`app_shell.rs`'s `resolve_active` tests.

## Render (only the active route mounts)

New arms in `App::render_component` (`crates/nemo/src/app.rs`, near the `app_shell` arm at
~1099):

* `"router"` — resolve `<route>` children by `component_type`, read the current path from
  the `RouterRegistry` (lazily initialized to `default`), find the matching route (or the
  fallback), and render **only that route's body** via `render_children`. Nested routers
  fall out for free: a `<router>` inside a route body is just another component the
  recursion reaches when that route is active. Chrome-free — the router renders the active
  body full-size with no decoration of its own.
* `"nav_link"` — a clickable (button-like) element; reads `route`/`router`/`label`,
  computes `is_active` by comparing `route` to the router's current path for active
  styling, and on click enqueues a `NavIntent` + `cx.notify(entity_id)` — the same closure
  shape as `button.rs:76-82` and the `app_shell` sidenav click (`app_shell.rs:184-190`).
* `"route"` — a no-op marker (`div().into_any_element()`), rendered by its parent router,
  exactly like the `app_sidenav`/`page` markers at `app.rs:1170`.

## Files to change

Follow the four-file component workflow that `app-shell` used, plus the reactivity wiring:

1. **`crates/nemo/src/containers/router.rs`** *(new)* — `Router` and `NavLink`
   `#[derive(IntoElement)]` view structs (modeled on `app_shell.rs`), the pure
   `match_route` matcher, and unit tests (matcher + active-route selection + fallback).
2. **`crates/nemo/src/containers/mod.rs`** — `pub use router::{Router, NavLink};`.
3. **`crates/nemo-registry/src/builtins.rs`** — in `register_container_components`
   (~141-203) register `router` (`default`, `primary` props), `route` (`path`, `on_enter`,
   `on_leave`), and `nav_link` (`route`, `router`, `label`), via the existing `reg(...)`
   helper.
4. **`crates/nemo/src/app.rs`** — add the `"router"` / `"nav_link"` arms and the `"route"`
   no-op marker to `render_component` (~1099); call `apply_pending_navigations()` in the
   poll loop at ~55.
5. **`crates/nemo/src/runtime.rs`** — add the `RouterRegistry` + a `nav_intents` queue to
   `NemoRuntime`; add `enqueue_navigation` / `apply_pending_navigations` (match, state
   update, param projection, `on-enter`/`on-leave` dispatch, history push) and
   `back`/`forward`. Give `RuntimeContext` (~1700) a handle to push `NavIntent`s plus the
   `data_dirty`/`data_notify` it already holds.
6. **`crates/nemo-plugin-api/src/lib.rs`** — add `navigate(router, path)` / `back(router)`
   / `forward(router)` to the `PluginContext` trait (~310) with default
   `Err(PluginError::Unsupported)` impls so existing plugin SDKs don't break; implement on
   `RuntimeContext` (enqueue only).
7. **`crates/nemo-extension/src/rhai_engine.rs`** — in `register_context` (~389-504)
   register `navigate` (1-arg → primary router, 2-arg → explicit), `back`, `forward`,
   mirroring the `set_data` registration at ~399-406.

**Reused mechanisms (do not reinvent):** `render_children` (`app.rs:498`), the
`get_or_create_selected_value`/state pattern (`state.rs:126`), `call_handler`
(`runtime.rs:494`), the `set_data` notify path (`runtime.rs:1742-1756`),
`plugin_value_to_config_value`, `map_icon_name`, and the theme colors + click-closure
shape from `app_shell.rs`.

## Example + docs

* **`examples/router/`** *(new)* — a small app: a top-level `<router>`, a few
  `<nav-link>`s, a `/users/:id` route reading `data.route.main.params.id`, a nested
  router, a `*` fallback, and one `on-enter` hook. Also **migrate one existing
  visibility-toggle example** (e.g. `examples/complete`) to the router to demonstrate the
  before/after and delete its `on_nav` handler.
* **Knowledge base** (per policy, when implemented): new pattern doc
  `patterns/routing.md` (router model, deferred-nav apply point, param projection); update
  `concepts/extensions.md` (new `navigate`/`back`/`forward` Rhai fns), `concepts/data-flow.md`
  (`data.route.*` projection + nav queue as a re-render trigger), and `roadmap.md`; add a
  `log.md` line.

## Verification

1. **Unit tests** (`router.rs`): `match_route` — literal match, `:param` capture, trailing
   `*`, no-match; active-route selection defaults to `default` and falls back to `*` when
   unmatched. Run `cargo test -p nemo` (build needs
   `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` — Metal compiler).
2. **Integration test** (in `runtime.rs` tests, near
   `test_set_component_property_nonexistent_component`): enqueue a navigation, run
   `apply_pending_navigations()`, assert the `RouterState` current path + the
   `data.route.<id>.params` projection, and assert `on-enter`/`on-leave` fired (a Rhai
   handler writing a sentinel via `set_data`). This is the regression guard that
   navigation is not re-entrant.
3. **End-to-end** via the `nemo-run` skill / `nemo screenshot`: launch `examples/router/`,
   click a `<nav-link>` and confirm the route body swaps; drive a Rhai `on-click` calling
   `navigate("/users/42")` and confirm the param renders; exercise `back()`/`forward()`.
4. `nemo validate --strict examples/router/app.xml` passes (schemas registered in file 3
   above, so `router`/`route`/`nav-link`/`on-enter`/`on-leave` do not warn).

## Relationship to the SFC plan

This plan is **independent** of [single-file components](sfc-components.md) —
neither blocks the other and either can land first. They edit disjoint code (the
router never touches `xml_parser.rs`, the template-expansion pipeline, or the
`call_handler` routing SFCs rely on; SFCs never touch `app.rs`/`render_component`).
The only shared file is `runtime.rs`, in different functions (a mechanical merge).
The one interaction is composition: a `<router>` nested inside an SFC `<template>`
gets its `id` scoped per instance (`main` → `foo_main`), so nav references inside
that SFC must resolve to the scoped id. This is its **own plan**,
[Scope nested routers inside SFCs](router-in-sfc-scoping.md), and is **not
implemented** — it landed with neither plan and is a latent break until an SFC
nests a router. It is two mechanisms, not one (static `router=` rewrite at scope
time + instance-relative runtime resolution for Rhai `navigate()`); see that plan.

## Suggested sequencing

1. Pure `match_route` + tests (no wiring) — cheapest, de-risks matching.
2. `RouterRegistry` + `enqueue_navigation` / `apply_pending_navigations` + poll-loop call +
   integration test — proves the deferred-nav core with no UI.
3. Registry entries + `"router"`/`"route"` render arms — static routing renders.
4. `"nav_link"` arm + Rhai `navigate`/`back`/`forward` — interactive navigation.
5. Lifecycle hooks, nested-router example, `*` fallback, param-projection polish.
6. Example app + example migration + KB updates.
