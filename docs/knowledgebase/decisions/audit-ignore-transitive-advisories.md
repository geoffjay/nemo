---
type: Decision
title: cargo audit ignores transitive advisories we cannot upgrade
description: Why .cargo/audit.toml ignores the RUSTSEC advisories pinned via the gpui-kit/wasmtime stacks, and what remains tracked.
tags: [security, dependencies, audit, ci]
timestamp: 2026-10-27T00:00:00Z
---

`cargo audit` runs as an `hk` check (`hk.pkl`, on pre-commit when `Cargo.lock`
changes and via `hk check`). As of 2026-10-27 (post gpui-kit migration) it
reported 46 vulnerability advisories (plus ~30 non-failing
unmaintained/unsound/yanked warnings); all 46 are ignored, and `cargo audit`
exits 0 with only warnings visible.

# Decision

Ignore the vulnerability advisories in `.cargo/audit.toml`, because **every one
is in a transitive dependency this workspace cannot upgrade on its own**:

* **`wasmtime` / `wasmtime-wasi` 29.0.1** (~30 advisories, incl. the `zstd`
  stack via wasmtime-cache) — pinned by `nemo-wasm`. The fix is a major-version
  bump to 36+/42+, i.e. a host-API port.
* **TLS stack: `quinn-proto` / `rustls` / `rustls-webpki` / `h2`** (~16) — via
  `gpui-pre-reqwest` (the reqwest fork inside the `gpui-pre =0.3.10` snapshot
  behind `gpui-kit` 0.8.1, quinn enabled for HTTP/3); older `rustls` 0.22 /
  `rustls-webpki` 0.102 instances also arrive via `async-nats`/`rumqttc` in
  `nemo-data` (bump-deferred, moves with the TLS stack). No upgrade path
  inside the snapshot pins without forking upstream.
* **`crossbeam-epoch`, `thin-vec`** — via gpui.
* **`quick-xml`** — our direct dep is `0.37`; `0.30`/`0.39` instances are
  transitive. Our own bump to `>=0.41` is feasible code-wise (two API changes:
  `BytesText::unescape`, `Attribute::unescape_value`→`normalized_value`) and —
  with the git pins gone — no longer blocked by gpui resolution; deferred as an
  ordinary (unblocked) future change.

The warnings (unmaintained/unsound/yanked — e.g. anyhow, core2, fxhash,
instant, memmap2, paste, rand, rustls-pemfile, rustybuzz, smartstring, spin,
ttf-parser, tokio, zbus_names/zvariant, zstd-safe) are left **un-ignored** —
cargo-audit does not fail on them and they are useful triage signal.

# Why not fix them

The gpui stack moved to crates.io (see [gpui-kit crates.io
dep](gpui-kit-crates-io-dep.md)), so a blanket `cargo update` is no longer
dangerous — the `gpui-pre` snapshot family is pinned exactly (`=0.3.10`) by the
published `gpui-kit` crates and cannot drift within the lock. But the snapshot
family moves only with a deliberate `gpui-kit` version bump, so the
transitives inside it (TLS stack) still move only in coordinated upgrades.
`wasmtime` stays pinned at 29 until the host-API port.

# Tracked follow-up

The `wasmtime`/`wasmtime-wasi` advisories are the ones that matter most — nemo
executes untrusted WASM plugins, and several are sandbox-escape / host-panic /
data-leak class. Upgrading `wasmtime` 29 → 42+ is tracked separately; when it
lands, prune those IDs (and the zstd group) from the ignore list. The
`quick-xml` direct bump is also now unblocked and can be done any time.

# Maintenance

Treat the ignore list as a snapshot, not a permanent allowlist. **Whenever the
gpui-kit/gpui-pre stack or the `wasmtime` pin is bumped, re-run `cargo audit`
and prune every ID that no longer fires.** New advisories should be triaged
(fix if direct, ignore-with-reason if transitive-and-pinned), not
blanket-added.