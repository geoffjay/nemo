---
type: Decision
title: GPUI comes from crates.io via the gpui-kit umbrella and the gpui-pre snapshots
description: nemo depends on gpui-kit 0.8.x which pins the gpui-pre snapshot crates exactly; no git deps remain and Cargo.lock is no longer load-bearing against git drift.
tags: [build, dependencies, gpui, decision]
timestamp: 2026-10-27T00:00:00Z
supersedes: the former git-pin policy recorded here before 2026-10-27 (see History).
---

# Decision

Nemo consumes GPUI exclusively through the **`gpui-kit`** umbrella crate
(crates.io), which transitively pins the `gpui-pre` snapshot family with exact
(`=x.y.z`) version requirements. The workspace manifests keep the dependency
KEYS `gpui` and `gpui_platform` (via `package = "gpui-pre"` /
`"gpui-pre-platform"`) so all `use gpui::*` source paths are unchanged, and a
`[patch.crates-io]` section is no longer needed.

```toml
[workspace.dependencies]
gpui = { package = "gpui-pre", version = "=0.3.10" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.10", features = ["font-kit", "x11", "wayland"] }
gpui-kit = { version = "0.8.1", features = ["tree-sitter-languages"] }
gpui-router = { version = "0.7", default-features = false, features = ["gpui-pre"] }
```

# Context

Until 2026-10-27 nemo used a rev-less **git** dependency on
`zed-industries/zed` (gpui) and `longbridge/gpui-component`, with a
`[patch.crates-io]` disambiguation pin. Because the deps carried no `rev`,
`Cargo.lock` was load-bearing: any dependency addition or `cargo update`
re-resolved gpui to a newer upstream commit and routinely broke the build.

The component library upstream became **GPUI Kit** (repo
`longbridge/gpui-kit`, site <https://gpui-kit.com>): a layered toolkit where
`gpui-base` (unstyled behavior/state) sits under `gpui-component` (styled UI),
and a single `gpui-kit` crate re-exports GPUI itself plus the layers. GPUI
itself is published as the `gpui-pre`/`gpui-pre-platform` snapshot crates —
republished zed revisions that keep the Rust lib names `gpui`/`gpui_platform`.
`gpui-kit` pins the exact snapshot (`=0.3.10`) because caret requirements let
weekly snapshot releases move applications onto a gpui the component library
does not compile with.

The migration also moved `gpui-router` 0.3 → 0.7: the 0.7 line has a
`gpui-pre` backend feature (`default-features = false, features =
["gpui-pre"]`) so the router shares the single gpui-pre instance instead of
pulling the separate crates.io `gpui` crate into the graph.

# Consequences

* **No git deps remain.** A blanket `cargo update` can no longer drift gpui to
  an incompatible revision — the `=0.3.10` requirements in the published kit
  crates make any drift a deliberate version bump. Normal cargo update hygiene
  applies again.
* **Bump procedure**: to move the GPUI stack, bump `gpui-kit` (and the
  `gpui`/`gpui_platform` snapshot pins to whatever the new kit release
  requires — they must stay version-aligned with each other), then fix any API
  breaks. The kit changelog documents component API changes.
* **Adding new crate dependencies is safe** — no re-resolve hazard. The old
  `include_str!` workaround in `nemo new` remains a fine choice (zero deps) but
  is no longer forced.
* `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` is still required
  for local macOS builds (Metal shader compiler), unless the
  `runtime_shaders` platform feature is enabled.
* The `screenshot` feature still maps to `gpui_platform/test-support`; under
  the new snapshot the Metal `framebuffer_only(false)` escape hatch is
  debug-assertions-only, so prefer a debug-profile build for screenshot runs
  on macOS (see [screenshot via test-support
  feature](screenshot-via-test-support-feature.md)).
* If the build breaks after a *kit bump*, suspect a component API change first;
  the gpui-kit release notes and `git log` in the upstream repo are the map.

# History

* 2026-07-11 – 2026-10-27: GPUI came from a rev-less git dep on zed; Cargo.lock
  pinned the working revision (`zed-industries/zed#3bd9d13…`) and was
  load-bearing. Adding dependencies or running `cargo update` could silently
  drift the pins. Superseded by the crates.io migration above.