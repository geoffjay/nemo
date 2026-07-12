---
type: Decision
title: cargo audit ignores transitive advisories we cannot upgrade
description: Why .cargo/audit.toml ignores 29 RUSTSEC advisories, and what remains tracked.
tags: [security, dependencies, audit, ci]
timestamp: 2026-07-12T00:00:00Z
---

`cargo audit` runs as an `hk` check (`hk.pkl`, on pre-commit when `Cargo.lock`
changes and via `hk check`). As of 2026-07-12 it reported 29 vulnerability
advisories (plus 14 non-failing unmaintained/unsound/yanked warnings).

# Decision

Ignore the 29 vulnerability advisories in `.cargo/audit.toml`, because **every
one is in a transitive dependency this workspace cannot upgrade on its own**:

* **`wasmtime` / `wasmtime-wasi` 29.0.1** (19 advisories) — pinned by `nemo-wasm`.
  The fix is a major-version bump to 36+/42+, i.e. a host-API port.
* **`rustls-webpki` / `quinn-proto`** (6) — via the `zed-reqwest` fork behind
  `gpui-component-assets`; no upgrade path without forking upstream.
* **`crossbeam-epoch`, `thin-vec`** (2) — via `gpui`.
* **`quick-xml`** (2) — our direct dep is `0.37`; `0.30`/`0.39` instances are
  transitive. Our own bump to `>=0.41` is feasible code-wise (two API changes:
  `BytesText::unescape`, `Attribute::unescape_value`→`normalized_value`), **but
  the `cargo update` it requires re-resolves the rev-less gpui git dependency
  and drifts the load-bearing pins** (see [pin gpui git dep](pin-gpui-git-dep.md)),
  breaking the build. Deferred to a coordinated dependency bump.

The 14 warnings (unmaintained: async-std, fxhash, instant, paste,
proc-macro-error2, rustls-pemfile, rustybuzz, ttf-parser, core2; unsound: rand,
memmap2, anyhow) are left **un-ignored** — cargo-audit does not fail on them and
they are useful triage signal.

# Why not fix them

The gpui git dependency is rev-less; any `cargo update` that touches shared
transitive crates re-resolves it and moves the zed/wgpu pins, which breaks the
build. So none of these can be bumped in isolation — they move only when the
whole gpui stack (and the `wasmtime` pin) is deliberately upgraded together.

# Tracked follow-up

The `wasmtime`/`wasmtime-wasi` advisories are the ones that matter most — nemo
executes untrusted WASM plugins, and several are sandbox-escape / host-panic /
data-leak class. Upgrading `wasmtime` 29 → 42+ is tracked separately; when it
lands, prune those IDs from the ignore list.

# Maintenance

Treat the ignore list as a snapshot, not a permanent allowlist. **Whenever the
gpui stack or the `wasmtime` pin is bumped, re-run `cargo audit` and prune every
ID that no longer fires.** New advisories should be triaged (fix if direct,
ignore-with-reason if transitive-and-pinned), not blanket-added.
