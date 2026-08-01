---
type: Concept
title: Data flow
description: Data sources, transforms, the repository, bindings, and the integration gateway.
tags: [data, async, bindings]
timestamp: 2026-07-11T00:00:00Z
---

Data flows from external sources through a transform pipeline into a central
repository, then propagates to component properties via bindings:

```
Sources → DataFlowEngine.process_update() → DataRepository → BindingSystem → LayoutManager → GPUI re-render
```

`nemo-data` owns the engine, sources, repository, transforms, and bindings;
`nemo-integration` owns the protocol clients.

# Data sources

Sources implement the async `DataSource` trait
(`crates/nemo-data/src/source.rs:118`) and emit `DataUpdate`s over a tokio
`broadcast` channel. Built-ins (`crates/nemo-data/src/sources/`, factory in
`sources/mod.rs`):

* **Polling** — `timer` (periodic ticks), `http` (one-shot, or polling when
  `interval` is set; reqwest).
* **Streaming** — `websocket` (tokio-tungstenite, auto-reconnect), `mqtt`
  (rumqttc), `redis` (pub/sub), `nats` (subjects).
* **Hybrid** — `file` (JSON/YAML/TOML/CSV/lines/raw, optional `notify` watch).

Sources are configured under `<data><source name="…" type="…" /></data>`,
created via `nemo_data::create_source()`, registered and started by the
`DataFlowEngine`, then consumed by tokio tasks the runtime spawns
(`crates/nemo/src/runtime.rs`, source setup ~520, update loop ~594).

**Startup ordering is load-bearing.** A source's `start()` broadcasts its
initial `full` value *immediately*, and a tokio `broadcast` channel drops
messages sent while no receiver is attached. So the runtime subscribes the
update loops (`start_data_update_loop()`) **before** `data_engine.start_all()`;
subscribing afterward loses each source's first value, leaving the repository
unseeded until some later event (e.g. a file-watcher change) re-delivers it.
Interactive runs usually get such a follow-up and appear to recover, but a
one-shot `nemo screenshot` does not — that was the root cause of issue #82
(bound values rendered as placeholders in captures).

# Transforms

The `Transform` trait (`crates/nemo-data/src/transform.rs:31`) and a `Pipeline`
apply transforms sequentially before storage. Built-ins: `MapTransform` (dot-path
field extraction), `FilterTransform`, `SelectTransform`, `SortTransform`,
`TakeTransform`, `SkipTransform`. A `TransformContext` carries `source_id`,
`timestamp`, and `variables`.

# DataRepository

`crates/nemo-data/src/repository.rs` — a thread-safe in-memory store: a single
`RwLock<Value>` root plus a `broadcast` change channel. Dot-separated paths
(`data.api.users`, `state.count`, `items[0]`, wildcard `data.*`) under three
namespaces: `data.*` (source outputs, by convention `data.<source_id>`),
`state.*`, `var.*`. `set`/`delete`/`update_from_source` broadcast a
`RepositoryChange`.

# Bindings

`crates/nemo-data/src/binding.rs` — a `Binding` maps a source `DataPath` to a
`BindingTarget` (component id + property) with a mode (`OneWay`/`TwoWay`/
`OneTime`) and optional transform. The `BindingSystem` indexes bindings by source
path; `on_data_changed()` produces `BindingUpdate`s (skipping unchanged values
and post-init `OneTime` bindings), and `on_ui_changed()` propagates two-way edits
back to the repository.

# Integration gateway

`crates/nemo-integration/src/lib.rs` — `IntegrationGateway` is a registry of
protocol clients (HTTP, WebSocket, MQTT, Redis, NATS) used by the data sources.
It is `!Send` (MQTT's `rumqttc::EventLoop` is `!Send`), so it lives on the main
thread wrapped in `Arc`. See [Architecture](architecture.md) for the threading
model.

# Reactivity signal

The only cross-thread path: a source update runs
`DataFlowEngine::process_update()` (apply transforms → `DataRepository::set()`),
sets `data_dirty` (`AtomicBool`), and calls `data_notify.notify_one()`. The
`App` async task wakes, runs `NemoRuntime::apply_pending_data_updates()` (read
repository → `LayoutManager::on_data_changed()` → `apply_updates()` sets
`BuiltComponent` properties), then `cx.notify()` re-renders.

The **page router** reuses this same signal. A `navigate()`/`back()`/`forward()`
call or a `<nav-link>` click enqueues a `NavIntent`, sets `data_dirty`, and
notifies; the poll loop runs `apply_pending_navigations()` (just before
`apply_pending_data_updates()`), which **projects** the active route's path +
params into the repository at `data.route.<id>.path` and
`data.route.<id>.params.*` and flags those paths dirty — so pages read them via
`get_data("route.<id>.params.x")` and `<binding source="data.route.<id>.params.x">`
propagates them like any other data change. See [routing](../patterns/routing.md).
