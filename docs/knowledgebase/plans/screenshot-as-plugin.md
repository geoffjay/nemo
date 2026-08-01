---
type: Plan
title: Screenshot as a plugin (CLI-subcommand plugins + a window-bootstrap primitive)
description: Move `nemo screenshot` out of a feature-gated host build into a distributable, OS-specific native plugin — requires two new host capabilities (plugins that register CLI subcommands, and a host primitive that launches an app window and hands back a capture-ready handle).
tags: [screenshots, plugins, cli, native, macos, bootstrap, plan]
timestamp: 2026-07-31T00:00:00Z
---

**Status: Planned / not implemented.** This plan supersedes the deployment
problem noted in [screenshot via test-support feature](../decisions/screenshot-via-test-support-feature.md):
today's `nemo screenshot` only works in a locally-built binary compiled with
`--features screenshot`, so it ships in no release artifact. The goal here is to
make screenshot capture **installable as a plugin** onto a stock release binary,
deployed as an OS-specific component.

# Motivation

The current capture path calls gpui's `Window::render_to_image()`, which is
`#[cfg(any(test, feature = "test-support"))]` **inside the gpui crate** — a
compile-time gate baked into the host binary at link time. The decision doc is
explicit that `test-support` must stay out of `default`/release builds (it can
change gpui codegen). Net effect: the feature exists but is undeployable — you
must rebuild locally to use it.

Reframing screenshot as a plugin lets it be built and distributed on its own
cadence, per-OS, without touching the host's release codegen. It also generalizes
two capabilities the plugin system is missing today, which are useful well beyond
screenshots.

# The core blocker (why a plugin can't just do this today)

A plugin cannot reach the current capture path:

* **WASM / Rhai** — sandboxed; no OS graphics or gpui access. Out.
* **Native `cdylib`** — "full Rust, no sandbox," but it only receives
  `PluginContext` (`get_data`/`set_data`, `emit_event`, component-property
  accessors, `navigate`…; `crates/nemo-plugin-api/src/lib.rs:285`). It never gets
  a `&mut Window` or the Metal context, and it cannot flip a compile-time gpui
  feature that lives in the host binary. A plugin that linked its own
  gpui-with-`test-support` would instantiate a *separate* gpui — it could not
  touch the host's live window.

So "plugin calls `render_to_image()`" only works if the host was already compiled
with the feature, which defeats the purpose. **A plugin-hosted screenshot must
capture at the OS level, not through gpui's render path.**

# Approach: OS-native capture + two new host primitives

## Capture mechanism (per-OS, in the plugin)

Instead of rendering the scene to a texture, the plugin captures the on-screen
window through an OS API:

* **macOS** — ScreenCaptureKit (`SCScreenshotManager` / `SCContentFilter` on
  window id), or legacy `CGWindowListCreateImage`.
* **Linux** (later) — the portal/`XShmGetImage` route; deferred, matching the
  existing Linux-capture "still open" status.

Trade-offs vs. the gpui path (all inherent to OS capture, all acceptable for a
deployable variant):

* The window must be **visible** — no invisible/offscreen capture.
* Needs **screen-recording permission** (macOS TCC prompt). The `render_to_image`
  path deliberately avoided this; the plugin path reintroduces it.
* Output is at native pixel resolution (retina-scaled), same as today.
* Native plugins are unsandboxed and ABI-locked to the host's compiler +
  `nemo-plugin-api` version (`crates/nemo-plugin-api/src/lib.rs:538` safety
  contract).

## Primitive 1 — `Capability::Command`: plugins that register CLI subcommands

Today subcommand dispatch is a static `clap`-derived enum matched in
`main.rs::main` (`match args.command.take()`, `crates/nemo/src/main.rs:66`), and
plugin discovery happens *later*, during app launch (driven by the `<plugins>`
whitelist in config). To let a plugin own a subcommand, discovery must move
**ahead of** (or beside) arg parsing, and dispatch must fall through to plugins.

Sketch — additive to `nemo-plugin-api`:

```rust
// crates/nemo-plugin-api/src/lib.rs
pub enum Capability {
    Component(String),
    DataSource(String),
    Transform(String),
    Action(String),
    EventHandler(String),
    Settings(String),
    Command(CommandSpec),   // NEW
}

/// Describes a CLI subcommand a plugin contributes. Kept declarative and
/// FFI-safe (no clap types cross the boundary).
pub struct CommandSpec {
    pub name: String,               // e.g. "screenshot"
    pub about: String,              // one-line help
    pub args: Vec<CommandArgSpec>,  // flags/positionals, rendered into clap host-side
}

pub struct CommandArgSpec {
    pub name: String,               // "out"
    pub long: Option<String>,       // "--out"
    pub short: Option<char>,        // 'o'
    pub takes_value: bool,
    pub required: bool,
    pub help: String,
}
```

And a new entry point the host calls once it has matched a plugin command —
runnable from a **background thread** (`context_arc` is already `Send + Sync`),
receiving the parsed argv as `PluginValue::Object`:

```rust
pub trait PluginRegistrar {
    // …existing register_* methods…
    fn register_command(&mut self, spec: CommandSpec);
}

// Invoked when the host dispatches this plugin's subcommand.
// Returns a process exit code. Runs outside the gpui run loop unless the
// command opts into a window via Primitive 2.
pub type PluginCommandFn =
    unsafe extern "C" fn(ctx: &dyn PluginContext, argv: &PluginValue) -> i32;
```

Host-side wiring (`crates/nemo/src/main.rs`, `crates/nemo/src/args.rs`):

* Two-phase parse. Phase A: a permissive pre-parse that pulls global flags
  (`--extension-dirs`, `--verbose`) and the first positional (candidate
  subcommand name) *without* rejecting unknown subcommands
  (`clap`'s `allow_external_subcommands` / `ignore_errors`, or a hand-rolled
  first-token peek before `Args::parse`).
* Discover plugins from the extension dirs (reuse `ExtensionLoader`; no config/app
  needed for the discovery scan). If the candidate name matches a built-in
  `Command` variant, dispatch as today. Otherwise, look for a plugin whose
  manifest declares `Capability::Command { name }`.
* If matched, build a `clap::Command` dynamically from the plugin's `CommandSpec`,
  parse the remaining argv against it, marshal into `PluginValue::Object`, and
  call the plugin's `PluginCommandFn`. Unmatched → the existing clap "unrecognized
  subcommand" error.
* Precedence: **built-ins win** over plugin commands (a plugin cannot shadow
  `run`/`dev`/`validate`/…), and this must be covered by a test alongside the
  existing `args.rs` parse tests.

## Primitive 2 — a window-bootstrap handle exposed to the command

Screenshot is not pure logic: it must (1) launch a real app window, (2) wait for
settle, (3) capture, (4) quit. The host already has the shared bootstrap —
`build_app_window(cx, BootstrapParams)` returning `WindowHandle<Root>`
(`crates/nemo/src/main.rs:265`, `BootstrapParams` at `:246`) — but none of it is
reachable from a plugin, and it must run **inside** `gpui_platform::application().run(...)`.

A plugin cannot own the gpui run loop (gpui types can't cross the FFI boundary).
So the host drives the loop and calls back into the plugin at defined lifecycle
points. Add a host-side capture hook the command declares it needs:

```rust
// Host-owned; the plugin never sees gpui types. The plugin asks the host to
// "launch this app config, settle N ms, then hand me raw pixels to encode."
pub trait PluginContext: Send + Sync {
    // …existing methods, all with Unsupported default impls…

    /// Launch an app window, wait `settle_ms`, and return the captured frame as
    /// raw RGBA + dimensions. The host owns the gpui run loop and window
    /// lifecycle; capture backend is host- and OS-defined. Default: Unsupported.
    fn capture_app_window(
        &self,
        _spec: &PluginValue,   // { app_config, size, settle_ms, theme, mode, route }
    ) -> Result<CapturedFrame, PluginError> {
        Err(PluginError::Unsupported("capture_app_window".into()))
    }
}

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,   // row-major, width*height*4
}
```

Two viable splits of responsibility — decide during implementation:

* **(a) Host captures, plugin encodes/writes.** `capture_app_window` returns
  `CapturedFrame`; the plugin does PNG encoding, path handling, and any OS
  permission prompting it wants. Keeps gpui + window lifecycle entirely
  host-side; the plugin stays a thin, portable consumer. **Recommended** — it
  also means the OS-capture backend can live in the host behind a small
  always-compiled surface (no `test-support`), and different capture backends
  (OS-native now, gpui-`render_to_image` later) are swappable without changing
  the plugin.
* **(b) Host launches + hands the plugin a window id; plugin captures.** The host
  exposes only the native window handle/id; the plugin owns the OS capture call.
  Maximizes plugin control (custom regions, multi-window) but leaks a platform
  handle across FFI and duplicates capture code per plugin.

If (a) is chosen, note the OS-native capture backend added to the host does **not**
need `gpui/test-support` at all — it reads pixels from the OS compositor, not the
Metal drawable — so it can ship in the **default release binary**. That is the
crux of the whole plan: the *host* gains a permissionful OS-capture path in
release, and the *plugin* supplies the CLI surface + policy on top.

# End-to-end flow (recommended shape: Command plugin + `capture_app_window`)

1. User installs `nemo-screenshot` (a native `cdylib`) into an extension dir.
2. `nemo screenshot --app-config app.xml --out out.png --size 1200x800`.
3. Host pre-parses, discovers the plugin, matches `Capability::Command("screenshot")`,
   builds the clap command from `CommandSpec`, marshals argv → `PluginValue`.
4. Host calls the plugin's `PluginCommandFn(ctx, argv)`.
5. Plugin calls `ctx.capture_app_window({app_config, size, settle_ms, …})`.
6. Host runs `gpui_platform::application().run(...)`, `build_app_window`, waits
   `settle_ms` on the real dispatcher, captures via the OS backend, quits, and
   returns `CapturedFrame`.
7. Plugin encodes PNG, writes `--out`, returns exit code.

# What this removes / keeps

* **Removes** the `screenshot` cargo feature and the `test-support` gate from the
  screenshot path (the `dispatch_screenshot` feature-split in
  `crates/nemo/src/main.rs:86-98` goes away, or becomes a thin built-in that
  delegates to the plugin if present).
* **Keeps** the option to *also* offer the gpui `render_to_image` backend behind
  the host capture surface for callers who want the no-permission/offscreen path
  in a purpose-built binary — the plugin doesn't care which backend the host uses.

# Open questions

* **Permission UX.** ScreenCaptureKit triggers a TCC prompt on first use; a
  headless/CI run needs the permission pre-granted (tccutil / MDM profile). Document
  and, if possible, detect-and-explain rather than hang.
* **Discovery cost.** Scanning extension dirs before every command adds startup
  work to *all* invocations; gate the plugin-command lookup to the
  "candidate name is not a built-in" branch so the common path is unaffected.
* **ABI fragility.** Native plugins have no ABI-version check
  (`crates/nemo-plugin-api/src/lib.rs:538`). A screenshot plugin shipped as a
  binary must be pinned to a host version; consider adding a version handshake to
  `nemo_plugin_manifest` as part of this work.
* **WASM parity.** `capture_app_window` and `Command` would need WIT additions to
  reach WASM guests; likely out of scope (capture is inherently native), but the
  `Command` capability could be WASM-friendly for non-graphical subcommands.

# Files this touches (when built)

* `crates/nemo-plugin-api/src/lib.rs` — `Capability::Command`, `CommandSpec`,
  `register_command`, `PluginCommandFn`, `capture_app_window`/`CapturedFrame`.
* `crates/nemo/src/args.rs` + `crates/nemo/src/main.rs` — two-phase parse,
  plugin-command discovery/dispatch, built-in precedence, host OS-capture backend
  wired to `build_app_window`/`BootstrapParams`.
* `crates/nemo-extension/src/plugin.rs` — load + expose the new entry point;
  extend `RuntimeContext` (`crates/nemo/src/runtime.rs`) to implement
  `capture_app_window`.
* `plugins/screenshot/` (new) — the macOS OS-native capture plugin (example +
  reference).
* Update [screenshot via test-support feature](../decisions/screenshot-via-test-support-feature.md),
  [Extensions](../concepts/extensions.md), and
  [three-tier extensions](../decisions/three-tier-extensions.md) once implemented.
