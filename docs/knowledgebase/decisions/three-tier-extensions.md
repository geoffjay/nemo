---
type: Decision
title: Three-tier extension model with a unified PluginContext
description: Rhai, native cdylib, and WASM Component Model plugins share one host API.
tags: [extensions, plugins, wasm, decision]
timestamp: 2026-07-11T00:00:00Z
---

# Decision

Nemo supports three extension tiers — **Rhai scripts**, **native `cdylib`
plugins**, and **WASM Component Model plugins** — all reaching the host through a
single `PluginContext` API defined in `nemo-plugin-api`.

# Rationale

The tiers trade off power against safety and portability:

* **Rhai** — lightweight in-process logic (event handlers), sandboxed by
  operation/size/stack limits, no I/O. Lowest friction.
* **Native** — full Rust, no sandbox, best performance and reach; loaded via
  `libloading`. Requires matching compiler + `nemo-plugin-api` version (no ABI
  check or signing).
* **WASM** — portable and sandboxed via wasmtime + a WASI subset and
  capability-based host functions; safe to distribute.

A single `PluginContext` (`get_data`/`set_data`, `emit_event`, `get_config`,
`log`, component-property accessors) keeps the mental model uniform across tiers;
`RuntimeContext` implements it once and bridges to the repository, event bus, and
layout manager.

# Consequences

* New host capabilities are added to `PluginContext` (and, for WASM, to the WIT
  interface in `crates/nemo-wasm-guest/wit/nemo-plugin.wit`) so all tiers benefit.
* WASM cannot pass recursive values directly — complex `PluginValue`s cross the
  boundary as JSON (`json-val`). See [Extensions](../concepts/extensions.md).
