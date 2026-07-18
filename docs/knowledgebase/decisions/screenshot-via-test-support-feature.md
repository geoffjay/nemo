---
type: Decision
title: nemo screenshot uses gpui's test-support render-to-image path
description: The `screenshot` build feature enables `gpui_platform/test-support` for offscreen `Window::render_to_image`; macOS-first, opt-in, additive to Cargo.lock.
tags: [screenshots, gpui, macos, build, feature, decision]
timestamp: 2026-07-18T00:00:00Z
---

# Decision

`nemo screenshot --app-config app.xml --out out.png` renders an app and captures
the frame to a PNG using gpui's built-in offscreen render path,
`Window::render_to_image()`. That method is compiled in only when the **non-default
`screenshot` cargo feature** is enabled on the `nemo` crate, which turns on
`gpui_platform/test-support` (cascading to `gpui/test-support` +
`gpui_macos/test-support`).

Build/run:

```
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  cargo run -p nemo --features screenshot -- \
  screenshot --app-config examples/app-shell/app.xml --out out.png --size 1200x800
```

# Context

An earlier spike (see [headless screenshots](../plans/headless-screenshots.md))
pursued capture on Linux under Xvfb+lavapipe and got blank images — the
compositor-less window's swapchain present never landed in the captured pixmap.
Research on the pinned gpui then found a first-class path that sidesteps that
entirely: gpui already ships `Window::render_to_image()`
(`gpui/src/window.rs:2125`, gated `#[cfg(any(test, feature = "test-support"))]`),
which on macOS renders the current frame's `Scene` to a Metal texture and reads
the pixels back (`gpui_macos/src/metal_renderer.rs`) **without presenting** —
no screen-recording permission, and the window need not be visible.

# How it works

* The `screenshot` command reuses `main.rs::build_app_window` (the shared window
  bootstrap) so the captured frame is identical to the real app, then spawns a
  task on the **real** dispatcher that waits `--settle-ms` (default 500) so
  async data bindings / first paint / animations land, calls
  `window.render_to_image()`, and `RgbaImage::save`s the PNG before `cx.quit()`.
* We use the production dispatcher (not `VisualTestAppContext`, whose
  `TestDispatcher` would freeze real timers/HTTP/polling). A fixed settle delay
  is the pragmatic v1; data-heavy apps may need a larger `--settle-ms`.
* `--theme`/`--mode` are applied *after* `build_app_window` so they win over the
  app's own XML-configured theme; `--size WxH` forces fixed windowed bounds.

# Consequences

* **Opt-in only — keep it out of `default`.** `test-support` can change gpui
  codegen/behavior, so release and normal builds must stay feature-free.
* **Additive to Cargo.lock, no rev drift.** Enabling the feature added only
  `proptest` (a **git** dep of gpui's test-support), `proptest-macro`, and
  `convert_case`; the pinned `zed-industries/zed#3bd9d13…` revision is unchanged.
  This is a feature-only change and must stay that way — verify `git diff
  Cargo.lock` after building. See [pin gpui git dep](pin-gpui-git-dep.md).
* **Output is at drawable resolution.** The PNG is `logical size × window scale
  factor` — a `--size 1200x800` capture is 2400×1600 on a retina Mac. `--size`
  is logical, not exact pixels.
* **The window briefly appears on screen.** v1 opens a normal (centered/windowed)
  window rather than positioning it off-screen; capture still works. Positioning
  it off-screen (à la `VisualTestAppContext::open_offscreen_window`) is a possible
  future refinement.
* Verified on macOS 2026-07-18 against `examples/app-shell` (nord dark) and
  `examples/components` (`--theme gruvbox --mode light`): faithful, non-blank
  renders. Linux/Windows: see [screenshots Windows out of scope](screenshots-windows-out-of-scope.md).

# Files

* `crates/nemo/Cargo.toml` — `screenshot` feature.
* `crates/nemo/src/args.rs` — `Screenshot(ScreenshotArgs)` (feature-gated).
* `crates/nemo/src/commands/screenshot.rs` — the command.
* `crates/nemo/src/main.rs` — `build_app_window` / `BootstrapParams` + dispatch arm.
