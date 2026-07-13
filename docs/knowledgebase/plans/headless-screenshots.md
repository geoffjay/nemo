---
type: Plan
title: Headless renderer and screenshots
description: Spike findings for headless GPUI rendering under Xvfb+lavapipe; deferred — not worth the vendor/fork effort right now.
tags: [screenshots, headless, gpui, linux, ci]
timestamp: 2026-07-12T00:00:00Z
---

**Status: Deferred (2026-07-12).** The spike proved rendering works, but the
capture path needs a compositor experiment or a GPUI fork for offscreen
render-to-texture. Neither justifies the effort right now — revisit when the
value (visual regression, AI-observable renders) demands it. Findings preserved
below.

A `nemo screenshot` capability so automated and AI-assisted workflows can *see*
what an app renders. Combined with hot-reload, this closes an
edit → render → observe loop for AI agents.

Source: phase-2 spike run (2026-07-11). The findings below are durable and
should not be re-derived.

# Spike result: feasible, capture path needs one more iteration

## What works

* **Software Vulkan on Linux.** Mesa `lavapipe` provides a Vulkan 1.4 device
  with `VK_KHR_xcb_surface` under Xvfb — headless GPU rendering is available
  with no physical GPU.
* **Nemo runs fully on Linux.** The app reaches `Runtime initialization
  complete` → project loaded → theme applied, no panic, stable for ~11 s.
* **Linux launch panic fixed** (commit `be2afa0`). The workspace `gpui_platform`
  features now include `x11` + `wayland` (previously `font-kit` only, so
  `gpui_linux::current_platform` hit `unreachable!()` whenever `DISPLAY` or
  `WAYLAND_DISPLAY` was set — i.e. on any real Linux desktop, not just
  headless). This fix also makes the Linux release binaries (`.deb`/`.rpm`/
  tarball) launch correctly.
* **No rev-pin needed for the fix.** Adding `x11`/`wayland` to `gpui_platform`
  was done WITHOUT adding a `rev` to the git dep — a `rev` splits the git
  source vs `gpui-component`'s rev-less refs into two incompatible `gpui`
  instances (298 type errors). The working recipe: keep specs rev-less, restore
  the good `Cargo.lock`, run a plain `cargo build` (adds only the new x11 crates,
  lock diff: +282/-0, no rev change). See
  [pin gpui git dep](../decisions/pin-gpui-git-dep.md).

## What doesn't work yet

* **Screenshot capture is blank.** Nemo reaches full render state, but
  `import -window root` / `xwd -root` under a compositor-less Xvfb capture
  black. GPUI's Vulkan swapchain present (lavapipe WSI) isn't landing in the
  root pixmap the capture tools read.

## Next iteration options

* **Run under a minimal compositor** — nested `weston`/`mutter --headless`, or
  a WM + backing store, then capture the window id directly.
* **True offscreen render-to-texture** — add a headless render path in GPUI:
  create a GPU surface backed by an offscreen texture instead of a window
  swapchain, render one frame, read pixels back, encode PNG. Requires GPUI
  internals work (or a fork branch); highest effort and most uncertain.
* **Scene serialization + external renderer** — rejected (duplicates the
  renderer; too much surface area).

# Proposed shape

`nemo screenshot --app-config app.xml --out out.png [--size WxH] [--settle-ms 500]`
— launches, waits for first paint + settle, captures via the platform mechanism
(Option 1 initially), writes PNG, exits non-zero on failure. Later, swap the
capture backend to offscreen render without changing the CLI surface.

# CI integration (future)

Once capture works: a CI job that screenshots each `examples/*` app and
uploads the images as artifacts — a visual regression baseline and an
AI-observable render result. The `screenshot-spike.yml` workflow exists as the
spike harness.