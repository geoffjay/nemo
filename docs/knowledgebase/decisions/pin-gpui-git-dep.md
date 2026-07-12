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
