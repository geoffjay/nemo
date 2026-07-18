---
type: Decision
title: Screenshots target macOS; Linux best-effort, Windows out of scope
description: `nemo screenshot` is a macOS-first development feature; Linux is best-effort/deferred and Windows is explicitly out of scope.
tags: [screenshots, platforms, macos, linux, windows, decision]
timestamp: 2026-07-18T00:00:00Z
---

# Decision

`nemo screenshot` (see [screenshot via test-support feature](screenshot-via-test-support-feature.md))
is prioritized and supported on **macOS**. Development happens on macOS, and this
is a development-time feature (visual iteration, AI-observable renders), so macOS
is where it must work.

* **macOS** — supported and verified (Metal drawable readback via
  `Window::render_to_image()`).
* **Linux** — best-effort / deferred. The prior Xvfb+lavapipe spike captured
  blank frames without a compositor, and the offscreen `render_to_image` path
  under gpui's blade/Vulkan renderer has not been confirmed to work. Revisit if
  a Linux CI or headless need arises.
* **Windows** — **out of scope.** Not a development target for nemo, and there is
  no near-term need. Documented so the absence is a decision, not an oversight.

# Context

The value of screenshots is closing an edit → render → observe loop for humans
and agents during development, which happens on macOS. Chasing cross-platform
capture parity (compositor experiments on Linux, a Windows path) would cost far
more than it returns right now.

# Consequences

* Keep the `screenshot` feature *compiling* on all platforms (the CLI surface and
  `build_app_window` are platform-neutral), but only guarantee capture on macOS.
* If/when a Linux path is needed, evaluate whether `render_to_image` works under
  the blade/Vulkan renderer before investing in a compositor-based fallback.
* Revisit Windows only if it becomes a development target.
