---
type: Decision
title: The gpui git dependency is load-bearing in Cargo.lock
description: Nemo depends on gpui/gpui-component via a rev-less git dep; Cargo.lock pins the working revision.
tags: [build, dependencies, gpui, decision]
timestamp: 2026-07-11T00:00:00Z
---

# Decision

Nemo depends on GPUI (and `gpui-component`) as a **git dependency without a
pinned `rev`**. The exact working revision is held by `Cargo.lock`, which is
therefore load-bearing and committed.

# Context

GPUI is not published to crates.io in the form Nemo needs, so it is pulled from
git. Because the dependency has no explicit `rev`, adding any new crate
dependency or running `cargo update` re-resolves the git dep to its latest
upstream commit, which routinely drifts to an incompatible GPUI and breaks the
build.

# Consequences

* Do **not** run a blanket `cargo update`. To add a dependency, add it narrowly
  and avoid letting the gpui entry re-resolve; verify `Cargo.lock`'s gpui
  revision is unchanged afterward.
* If the build suddenly breaks after a dependency change, suspect gpui drift in
  `Cargo.lock` first.
* Local macOS builds also require `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`
  (the Metal shader compiler).
* **Adding any new crate dependency** can trigger a re-resolve that drifts the
  rev-less gpui/gpui-component pins. This bit the `nemo new` scaffold: the plan
  called for `include_dir`, but adding it drifted the pins and broke the build.
  The workaround is to embed templates via `include_str!` (a compile-time
  builtin, zero new dependencies) instead of `include_dir`. See
  `crates/nemo/src/commands/new.rs`.
* **Enabling a gpui *feature* is safe if it stays additive.** Turning on the
  `screenshot` feature (→ `gpui_platform/test-support`) added only `proptest`
  (a git dep of test-support), `proptest-macro`, and `convert_case` to
  `Cargo.lock`, leaving the pinned `zed-industries/zed#3bd9d13…` revision
  unchanged. A feature toggle must **not** drift the rev — verify `git diff
  Cargo.lock` after the first build and restore the lock if it does. See
  [screenshot via test-support feature](screenshot-via-test-support-feature.md).
