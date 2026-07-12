---
type: Plan
title: Roadmap
description: Current capabilities, phase-2 status, remaining roadmap items, and pointers to full planning docs.
tags: [roadmap, planning]
timestamp: 2026-07-12T00:00:00Z
---

This is a short, KB-level orientation. The detailed planning lives under
`docs/planning/` — `nemo-improvement-roadmap.md` is the source for the remaining
items below; `phase-2-build-and-installation.md` is the source for the phase-2
status table. (The older HCL-era planning docs — `nemo-project-vision.md`,
`nemo-system-architecture.md`, `nemo-code-review.md`, and `subsystems/*.md` —
have been removed; the KB concepts supersede them.)

# Where Nemo is

Nemo currently supports:

* Declarative XML UI with 50+ built-in components and a component registry.
  See [Components](../concepts/components.md).
* Live data binding from HTTP, WebSocket, MQTT, Redis, NATS, timer, and file
  sources through a transform pipeline and central repository. See
  [Data flow](../concepts/data-flow.md).
* Three extension tiers — Rhai scripts, native `cdylib` plugins, and WASM
  Component Model plugins — over a unified `PluginContext`. See
  [Extensions](../concepts/extensions.md).
* Built-in themes, workspace shell (header/footer/main view), and a settings UI.

Worked examples live under `examples/` (basic, components, data-binding,
data-streaming, calculator, pid-control, complete). Example configs are
validated in CI via `nemo validate --strict` (`.github/workflows/ci.yml`,
`validate-examples` job).

# Phase 2 — build, installation, and developer experience

Source: `docs/planning/phase-2-build-and-installation.md` (re-baselined
2026-07-10/11). Items marked done are landed on `main` unless noted.

| Item | Status | Notes |
|------|--------|-------|
| CLI subcommand architecture | ✅ Done | `new`/`dev`/`validate` subcommands; bare `nemo --app-config` preserved. |
| `nemo new` scaffold | ✅ Done | 4 templates (`basic`, `calculator`, `data-binding`, `complete`) embedded via `include_str!`; scaffolds validate. |
| Hot-reload dev mode | ✅ Done | `nemo dev` + `--watch`; `notify` watcher drives `Workspace::reload_config` (~0.7 s). |
| `nemo validate` subcommand | ✅ Done | `--strict` lints (`unknown-component`, `missing-required`, `unknown-attribute`, `missing-id`, `unused-template`); `--format human\|json`; `--validate-only` forwards. |
| Cross-platform packaging | ✅ Validated | v0.7.0-rc.1 produced 14 assets across 5 targets (`.tar.gz`/`.zip`/`.app`/`.dmg`/`.deb`/`.rpm` + checksums). Gaps: AppImage, `.msi`, signing. |
| Distribution | ✅ Validated | `install.sh` verified end-to-end; Homebrew formula auto-push wired (gated on `HOMEBREW_TAP_TOKEN`). `brew install geoffjay/tap/nemo`. |
| Headless renderer / screenshots | 🟡 Spiked | Render works under Xvfb+lavapipe; Linux launch panic fixed (`be2afa0`); capture is blank without a compositor. See [headless screenshots](headless-screenshots.md). |

# Remaining roadmap items

From `docs/planning/nemo-improvement-roadmap.md` §1, §3, §4 — not yet started.

## Component storybook (§1)

* **Generated gallery** — derive `examples/components/app.xml` from registry
  schemas so new components appear automatically.
* **Interactive property playground** — a `<property-inspector>` driven by
  `ComponentSchema`, applying changes via `set_component_property()`.
* **`nemo storybook` subcommand** — launch the gallery directly with
  `--component`/`--search` flags.
* **Fix Styling page code snippets** — still show HCL syntax; update to XML.

## Configuration authoring (§3)

* **XML Language Server (`nemo-lsp`)** — completion (components, attributes,
  enum values, data/template/handler refs), hover docs, diagnostics, go-to-def.
  Reuses `nemo-config` + `nemo-registry` as libraries; ~500 lines over
  `tower-lsp`.
* **VS Code extension** — thin wrapper registering `nemo-lsp` + syntax
  highlighting + "Preview in Nemo" command.
* **Expression language improvements** — `${config.section.key}`,
  `${data.source.field}` shorthand, edit-distance "did you mean?" errors.

## LLM configuration generation (§4)

* **`nemo generate` CLI** — natural-language → `app.xml` via an OpenAI-compatible
  endpoint; system prompt auto-generated from schemas; validation-correction loop.
* **Built-in `<chat>` component** — persistent chat UI backed by an HTTP source.
* **Authoring assistant app** — chat + code-editor + launch-preview, as a
  purpose-built `examples/assistant/`.
