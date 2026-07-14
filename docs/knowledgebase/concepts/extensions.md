---
type: Concept
title: Extensions
description: The three-tier extension model — Rhai scripts, native plugins, and WASM plugins.
tags: [extensions, plugins, wasm, rhai]
timestamp: 2026-07-11T00:00:00Z
---

Nemo extensions come in three tiers of increasing power, all reaching the host
through one unified `PluginContext` API:

1. **Rhai scripts** — lightweight, sandboxed, in-process.
2. **Native plugins** — `cdylib` dynamic libraries, full Rust, no sandbox.
3. **WASM plugins** — Component Model modules, portable and sandboxed.

`ExtensionManager` (`crates/nemo-extension/src/lib.rs`) coordinates all three:
it owns a `RhaiEngine`, a `PluginHost` (native), and a `WasmHost`, plus an
`ExtensionLoader` that discovers extensions on disk.

# The API contract

`nemo-plugin-api` (`crates/nemo-plugin-api/src/lib.rs`) is the stable contract
both host and plugins depend on:

* `PluginValue` — FFI-safe value enum (null/bool/i64/f64/String/Array/Object).
* `PluginManifest`, `Capability` (Component/DataSource/Transform/Action/
  EventHandler/Settings), `PluginPermissions`.
* `PluginRegistrar` — passed to a plugin's entry point to register capabilities.
* `PluginContext` (`Send + Sync`) — runtime API: `get_data`/`set_data`,
  `emit_event`, `get_config`, `log`, `get_component_property`/
  `set_component_property`.
* `declare_plugin!(manifest, init_fn)` — generates the `extern "C"`
  `nemo_plugin_manifest` and `nemo_plugin_entry` symbols.

`RuntimeContext` in `crates/nemo/src/runtime.rs` implements `PluginContext`,
bridging plugins to the `DataRepository`, event bus, config, and `LayoutManager`
(~lines 1683-1759). `nemo-plugin` provides fluent builders on top of the API.

# Rhai scripts

`RhaiEngine` (`crates/nemo-extension/src/rhai_engine.rs`) compiles `.rhai`
scripts to ASTs and exposes math/string/conversion/logging helpers, JSON
helpers (`json_parse` / `json_stringify`, backed by `serde_json`), and the
`rhai-chrono` package for date/time arithmetic. Sandboxed by `RhaiConfig`
limits (operations, string/array/map sizes, call-stack depth); no eval.

Handlers referenced from XML (e.g. `on-click="handler"`) resolve to script
functions.

## Opt-in packages and the `file-io` feature

By default scripts have **no host I/O** — they cannot read or write files,
spawn processes, or access the environment. Two opt-in packages are wired
behind the `<script>` element's `features` attribute:

| Feature flag | Package | What it adds |
|---|---|---|
| `file-io` | [`rhai-fs`](https://crates.io/crates/rhai-fs) | `open_file`, `read_string`, `write`, `exists`, `create_dir`, `cwd`, `path`, … |
| `network` | _(reserved — HTTP is provided by the built-in `http_get`/`http_post`/`http_put`/`http_delete`)_ | — |
| `system` | _(reserved — for future `rhai-env` / `rhai-process` opt-in)_ | — |

`rhai-chrono` (date/time) is **always** registered — it is pure and touches no
host state. `json_parse` / `json_stringify` are also always available.

Enable filesystem access from an app:

```xml
<script src="./scripts" features="file-io" />
```

Multiple features are comma-separated: `features="file-io, system"`.

### Security model

`file-io` grants the script **full filesystem access** with the permissions of
the host process — there is no sandbox root confinement. Only enable it for
apps whose scripts you trust. The default (no `features` attribute) preserves
the sandbox: no file, environment, or process access. The `network` and
`system` flags are reserved for future opt-in packages and currently no-ops.

### Packages considered but not included

- **rhai-ml** — stale (last published Feb 2024), pulls smartcore, no use case.
- **rhai-http** — non-standard license, and nemo already ships built-in
  `http_get`/`http_post`/`http_put`/`http_delete` registered via
  `register_http_functions`.
- **rhai-env**, **rhai-sci**, **rhai-process** — available to add behind
  feature flags if a use case arises; `rhai-process` in particular is
  dangerous (subprocess spawning) and must remain opt-in.

# Native plugins

`PluginHost` (`crates/nemo-extension/src/plugin.rs`) loads `cdylib` libraries via
`libloading` (`.dylib`/`.so`/`.dll`), reads the manifest, and calls
`nemo_plugin_entry` with a registrar. **No ABI-version check or signing** — a
plugin must be built with the same Rust compiler and `nemo-plugin-api` version as
the host. Example: `plugins/mock-data/`.

# WASM plugins

`WasmHost` (`crates/nemo-wasm/src/lib.rs`) runs Component Model modules on
wasmtime with a WASI subset and capability-based host functions. The WIT
interface (`crates/nemo-wasm-guest/wit/nemo-plugin.wit`) defines host imports
(`get-data`, `set-data`, `emit-event`, `get-config`, `log`, component-property
accessors) and guest exports (`get-manifest`, `init`, `tick() -> u64`, where the
return is ms until the next tick, 0 = stop). `tick_all()` drives the tick loop.
Complex values cross the boundary as JSON (`json-val`) since WIT lacks recursive
types (`convert.rs`). Guests use `nemo-wasm-guest` (re-exports `wit-bindgen`).
Example: `plugins/mock-data-wasm/`.

# Discovery

`ExtensionLoader` scans an extension dir's `scripts/` (`.rhai`), `plugins/`
(platform-specific dynamic libs), and `wasm-plugins/` (`.wasm`). The file stem
becomes the plugin ID. The `<plugins>` block in config whitelists which
discovered native/WASM plugins actually load.
