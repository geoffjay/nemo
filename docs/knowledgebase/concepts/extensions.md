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
  `set_component_property`, and `navigate`/`back`/`forward` (router
  navigation; default impls return `PluginError::Unsupported` so older SDKs
  still compile).
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
functions. `load_script` runs the script's top-level statements once against a
persistent `Scope` (seeding `let`/`const` declarations), and `call` reuses that
scope across invocations.

**Startup hook.** `<script on-load="fn_name" />` names a handler that the runtime
calls exactly once, after scripts are loaded and the layout is built, from
`App::new` (via `NemoRuntime::on_load_handler`). It is invoked as
`fn_name("app", "load")`. This is the only "run once on load" hook nemo exposes;
use it to hydrate the UI from persisted state at startup, so the first paint
already reflects it rather than deferring the sync onto the first user
interaction. The `on-load` attribute is parsed into `scripts.on_load` (attribute
keys are kebab→snake normalized, so `on-load` arrives as `on_load`).

**Router navigation.** `navigate(path)` (primary router) / `navigate(router,
path)`, `back()` / `back(router)`, and `forward()` / `forward(router)` are
registered on the engine (`register_context`) and drive the page router. They
only **enqueue** a navigation intent and wake the poll loop — the actual apply
(history update, param projection, `on-enter`/`on-leave` hooks) happens later,
outside the `extension_manager` write lock a running handler holds, so calling
them from inside a handler can't deadlock. See
[routing](../patterns/routing.md).

**Input value readback.** An `<input>`'s typed text is written back into its
`value` property on every change/blur (and Enter fires its `on-change` handler),
so `get_component_property(id, "value")` returns live text a handler can read.
The render pass also pushes any script-set `value` back into the field — e.g.
`set_component_property(id, "value", "")` clears it — without disturbing the
cursor mid-edit. See `App::get_or_create_input_state` / `App::sync_input_value`.

**Rhai script functions are pure — do not rely on module-level state.** A
top-level `let`/`const` is *not* visible inside a script function: the seeded
scope reaches only the entry function of a `call`, and a function invoked from
another function (the normal handler → helper case) gets a fresh scope. Any
reference to a top-level variable from such a function fails at run time with
`Variable not found` — and because Rhai resolves names at call time, this is
invisible to `load_script` (compilation succeeds). Share values two ways:

* **Shared constants** — expose as a zero-arg function (`fn data_file() {
  "path/to.json" }`). Functions are globally callable from any other function.
* **Shared mutable state** — use the host store via `get_data` / `set_data`
  (backed by the `DataRepository`), or the components themselves via
  `get_component_property` / `set_component_property`. Both persist across
  handler calls and are reachable from every function.

The task-list and dev-dashboard examples follow this: per-item state lives on
disk / in components rather than module variables. The task-list starts empty,
loads `tasks.json` via the `on-load` hook, adds tasks through a modal (reading
the inputs' `value` properties), and renders the list into a data-driven
`<table>` — each handler loads from disk, edits, saves, and re-renders. See
`test_task_list_handlers_end_to_end` in `rhai_engine.rs` for the end-to-end
regression guard.

## Opt-in packages and the `file-io` feature

By default scripts have **no host I/O** — they cannot read or write files,
spawn processes, or access the environment. Opt-in packages are wired behind
the `<script>` element's `features` attribute and, for the heavier packages,
behind Cargo features on `nemo-extension`:

| Feature flag | Package | Cargo feature | What it adds |
|---|---|---|---|
| `file-io` | [`rhai-fs`](https://crates.io/crates/rhai-fs) | always compiled | `open_file`, `read_string`, `write`, `exists`, `create_dir`, `cwd`, `path`, … |
| `system` | [`rhai-env`](https://crates.io/crates/rhai-env) | `pkg-env` | `env(key)`, `envs()`, `set_env(key, value)` |
| `system` | [`rhai-process`](https://crates.io/crates/rhai-process) | `pkg-process` | `cmd([...]).pipe(...).build().run()` — subprocess execution with `Config` policy |
| `science` | [`rhai-sci`](https://crates.io/crates/rhai-sci) | `pkg-sci` | `mean`, `std`, `median`, `linspace`, matrix ops, regression, SVD, … (compiled with `default-features = false` to avoid polars/nalgebra) |
| `network` | _(reserved)_ | — | HTTP is already available via built-in `http_get`/`http_post`/`http_put`/`http_delete` |

`rhai-chrono` (date/time) is **always** registered — it is pure and touches no
host state. `json_parse` / `json_stringify` are also always available.

Enable filesystem access from an app:

```xml
<script src="./scripts" features="file-io" />
```

Enable env + process (requires the `pkg-env` and `pkg-process` Cargo features
on `nemo-extension` at build time):

```xml
<script src="./scripts" features="file-io, system" />
```

Multiple features are comma-separated: `features="file-io, system, science"`.

### Security model

`file-io` grants the script **full filesystem access** with the permissions of
the host process — there is no sandbox root confinement. `system` grants
environment-variable access (rhai-env) and **subprocess spawning** (rhai-process)
— the latter is the most dangerous capability. `science` is pure computation
but gated behind a Cargo feature because it adds a heavy dependency tree.
Only enable these for apps whose scripts you trust. The default (no `features`
attribute) preserves the sandbox: no file, environment, or process access.

### Cargo features

The `pkg-env`, `pkg-sci`, and `pkg-process` Cargo features on `nemo-extension`
control whether the packages are compiled into the binary at all. The `nemo`
crate re-exports them as its own features (`pkg-env`, `pkg-sci`, `pkg-process`,
and `all-packages` for all three). `nemo`'s **default** is `["pkg-env",
"pkg-sci"]`: `rhai-sci` is a light numeric dependency and `rhai-env` is low-risk
and still gated by the script `system` feature. `pkg-process` (arbitrary
subprocess spawning) is deliberately **left out of the default** so a stock
binary physically cannot spawn processes — enable it explicitly for apps that
need it:

```bash
cargo run --features pkg-process -- --app-config examples/dev-dashboard/app.xml
```

**Both layers are required.** A package is registered only when the app opts in
via the `<script features="…">` attribute *and* the binary was compiled with the
matching Cargo feature. (Compiling a package in does not weaken the sandbox — a
stock app with no `features` attribute stays fully sandboxed regardless.) If a script requests `system`/`science` but the Cargo
feature is absent, the package is silently skipped and calls fail at run time
with `Function not found: env` (etc.) — because Rhai resolves names at call time,
compilation still succeeds. To make this obvious, `register_standard_functions`
logs a `warn!` at engine construction when a requested feature has no compiled
package (e.g. "feature 'system' enabled but built without 'pkg-env'").

### Packages considered but not included

- **rhai-ml** — stale (last published Feb 2024), pulls smartcore, no use case.
- **rhai-http** — non-standard license, and nemo already ships built-in
  `http_get`/`http_post`/`http_put`/`http_delete` registered via
  `register_http_functions`.

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
