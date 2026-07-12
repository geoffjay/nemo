---
type: Plan
title: Roadmap
description: Current capabilities and where Nemo is heading; pointer to the full planning docs.
tags: [roadmap, planning]
timestamp: 2026-07-11T00:00:00Z
---

This is a short, KB-level orientation. The detailed planning lives under
`docs/planning/` — notably `nemo-project-vision.md`,
`nemo-improvement-roadmap.md`, `nemo-system-architecture.md`, and
`docs/planning/subsystems/`.

# Where Nemo is

Nemo currently supports:

* Declarative XML UI with 50+ built-in components and a component registry.
* Live data binding from HTTP, WebSocket, MQTT, Redis, NATS, timer, and file
  sources through a transform pipeline and central repository. See
  [Data flow](../concepts/data-flow.md).
* Three extension tiers — Rhai scripts, native `cdylib` plugins, and WASM
  Component Model plugins — over a unified `PluginContext`. See
  [Extensions](../concepts/extensions.md).
* Built-in themes, workspace shell (header/footer/main view), and a settings UI.
* Packaging/distribution (see `docs/planning/phase-2-build-and-installation.md`
  and `docs/public/packaging.md`); v0.7.0-rc validation is in progress.

Worked examples live under `examples/` (basic, components, data-binding,
data-streaming, calculator, pid-control, complete).

# Direction

Refer to `docs/planning/nemo-improvement-roadmap.md` for the maintained list of
planned work and code-review follow-ups. Update this entry (and add
per-initiative plan docs here) as larger efforts are scoped, rather than
duplicating the full roadmap.
