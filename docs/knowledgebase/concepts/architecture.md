---
type: Concept
title: Architecture
description: What Nemo is, the workspace crate layout, and the config → layout → render flow.
tags: [architecture, core, gpui]
timestamp: 2026-07-11T00:00:00Z
---

Nemo is a configuration-driven desktop application framework built on
[GPUI](https://gpui.rs) (Zed's GPU-accelerated UI framework). An application is
declared in XML (component tree, data sources, bindings, handlers, theme). Nemo
parses the XML, builds an internal component tree, and renders it as native GPUI
elements. Data flow is asynchronous and pushed into a central repository;
changes propagate through a binding system that updates component properties and
signals GPUI to re-render.

# Workspace crates

All crates live under `crates/`. `nemo` is the binary; the rest are libraries.

* **nemo** — binary, GPUI app shell, component rendering, window/workspace UI.
  `src/main.rs` (entry/CLI), `src/runtime.rs` (`NemoRuntime` orchestration),
  `src/app.rs` (`App`, render pipeline, `render_component()` dispatch),
  `src/components/` (GPUI component wrappers), `src/workspace/`, `src/theme/`.
* **nemo-config** — XML parsing, expression resolution, schema validation, the
  `Value` AST. See [Configuration](configuration.md).
* **nemo-layout** — builds the `BuiltComponent` tree from config, manages
  bindings, applies data updates (`manager.rs`, `builder.rs`, `binding.rs`, `node.rs`).
* **nemo-data** — data flow engine, sources, repository, transforms. See [Data flow](data-flow.md).
* **nemo-events** — typed pub/sub event bus (tokio broadcast).
* **nemo-extension** — Rhai scripts, native plugins (`libloading`), WASM host. See [Extensions](extensions.md).
* **nemo-integration** — protocol clients (HTTP, WebSocket, MQTT, Redis, NATS).
* **nemo-registry** — catalog of components/data sources/transforms/actions with schemas (`builtins.rs`).
* **nemo-macros** — proc macros (`#[derive(NemoComponent)]`). See [Components](components.md).
* **nemo-plugin** / **nemo-plugin-api** — native plugin SDK and the stable host↔plugin API contract.
* **nemo-wasm** / **nemo-wasm-guest** — WASM Component Model host (wasmtime) and guest SDK (wit-bindgen).

# Startup flow

`main.rs:main()` parses CLI args. Subcommands (`new`, `dev`, `validate`) dispatch
and return; `--headless` runs without GPUI; otherwise GPUI launches. In the
window, `create_runtime()` builds an `Arc<NemoRuntime>`, the theme is applied,
and an `App` entity is created (`cx.new(|cx| App::new(runtime, window, cx))`).

`NemoRuntime::new()` constructs the subsystems (tokio runtime, event bus,
registry + `register_all_builtins()`, layout manager, data flow engine,
extension manager, integration gateway, config loader). `initialize()` discovers
and loads extensions, expands templates, calls
`apply_layout_from_config()` (→ `LayoutManager::apply_layout()` builds the
`BuiltComponent` tree and registers bindings), then `setup_data_sources()` and
`setup_data_sinks()`.

# Render pipeline

`App::render()` → `render_layout()` snapshots all components from the
`LayoutManager` under a single lock, then `render_component()` dispatches on
`component_type` (50+ types) and wraps the result with `apply_layout_styles()`
(width/height/margin/padding/border/background/shadow/rounded). See
`crates/nemo/src/app.rs` (`render_layout()` ~343, `render_component()` ~618).

# Key constraints

* **`NemoRuntime` is `!Send`/`!Sync`** — `ExtensionManager` holds a Rhai engine
  and `IntegrationGateway` holds a `rumqttc::EventLoop`, both non-`Send`. All
  runtime access is from the main/UI thread; async I/O is dispatched to tokio via
  `Send + Sync` handles. `NemoRuntime` is wrapped in `Arc` before launch.
* **Data reactivity** is the only cross-thread path: sources push to the
  `DataRepository`, set `data_dirty`, and `notify_one()` a tokio `Notify`; the
  `App` async task wakes, calls `apply_pending_data_updates()`, and `cx.notify()`
  triggers a re-render. See [Data flow](data-flow.md).
* **Stateful widgets** (Table, Tree, Input, …) need `Entity<T>` persistence
  across re-renders — see [stateful widget persistence](../patterns/stateful-widget-entity-persistence.md).
