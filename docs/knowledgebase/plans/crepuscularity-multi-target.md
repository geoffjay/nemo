---
type: Plan
title: Multi-target output via crepuscularity View IR
description: Reach mobile/TUI/web targets by lowering Nemo's runtime IR into crepuscularity's View IR (an export target), NOT by embedding the .crepus builder or swapping the GPUI runtime.
tags: [multi-target, view-ir, crepuscularity, mobile, tui, web, export]
timestamp: 2026-08-02T00:00:00Z
---

# Why

We want Nemo apps to reach more than GPUI desktop — SwiftUI/Compose mobile,
Ratatui TUI, and HTML/WASM web — without the end user building anything.
[crepuscularity](https://github.com/tschk/crepuscularity) is a UI toolkit whose
`.crepus` DSL already targets exactly those backends, so the question is whether
it can plug into Nemo's runtime-config model.

**Verdict: do not embed crepuscularity's builder/parser, and do not swap its GPUI
runtime in. Instead reuse crepuscularity's View IR + backend shells as an *export
target*, lowering Nemo's existing runtime IR into it.** Nemo already owns the part
`.crepus` would duplicate; crepuscularity owns the part Nemo lacks (a
platform-neutral IR + the shells that render it on each platform).

# The structural symmetry that makes this work

Both systems independently converge on: *parsed source → GPUI-agnostic node tree
→ per-target renderer.*

- **Nemo:** `app.xml` → `nemo_config::Value` → `nemo_layout::LayoutNode` →
  `LayoutManager`'s flat `HashMap<id, BuiltComponent>` (the persistent runtime IR,
  mutated in place by data bindings) → per-frame `match component_type` in
  `nemo/src/app.rs::render_component` builds `gpui_component` widgets via
  `#[derive(NemoComponent)]` structs implementing `RenderOnce`.
- **Crepuscularity:** `.crepus`/jsx/svelte/vue → `crepuscularity-core` AST →
  either `render_nodes` (runtime GPUI, primitive `div`/`span`/`button` +
  Tailwind only) **or** `crepuscularity-native` lowering to **View IR** JSON
  (`ViewIr`/`ViewNode`/`ViewStyle`, `IR_VERSION = 7`) consumed by SwiftUI/Compose
  shells, plus `crepuscularity-tui` (Ratatui) and `crepuscularity-web`
  (HTML/WASM).

The valuable seam is IR↔IR: `BuiltComponent` tree → `crepuscularity_native::ViewIr`.

# Why NOT embed the builder / swap the runtime

- **`.crepus` is strictly less expressive than Nemo's XML for our purposes.**
  Crepuscularity's frontends compile the *template only* — `<script>` is extracted
  and never executed. So `.crepus` has nowhere to carry Nemo's `bind_*` data
  bindings, data sources (timer/HTTP/WS/MQTT/Redis/NATS), or `on_*` RHAI handlers.
  Adopting it as an input language is a second config dialect that expresses less.
- **crepuscularity's GPUI runtime is primitive-only.** `render_nodes`
  (`crepuscularity-runtime/src/renderer.rs`) emits `div`/`span`/`button` styled
  with Tailwind classes and nothing else. Nemo's ~50 semantic components
  (`select`, `tabs`, `area_chart`, `sidenav_bar`, `modal`, `dropdown_button`…) are
  backed by the **gpui_component** widget library (e.g. `label.rs` →
  `gpui_component::Label`). Swapping in crepuscularity's runtime means rebuilding
  every widget as a primitive template and losing gpui_component, plus two
  incompatible styling models (Nemo's per-attribute `apply_layout_styles` vs
  Tailwind strings).

Both alternatives cost a second config language and/or a second component model
and buy nothing Nemo doesn't already have. The IR export is additive and leaves
the desktop path untouched.

# Design: `nemo → View IR` exporter

- New gpui-free crate (e.g. `nemo-viewir`) depending on `crepuscularity-native`
  for the `ViewIr`/`ViewNode`/`ViewStyle` serde types. Mirrors the gpui-free
  boundary already used by `xtask`/`nemo-tokens`.
- **Node mapping** `component_type → ViewNode`: `label→text`,
  `panel`/`stack`/layout→`stack`, `button→button`, `select→picker`, `tabs→tabs`,
  `list→list`, text input→`input`, `image→image`, `divider`/`spacer`/`slider`/
  `progress`/`badge`/`checkbox`/`toggle` map 1:1. Unmapped components → explicit
  "unsupported on target" (or primitive-stack fallback), never a silent drop.
- **Style mapping** is *easier for Nemo than for crepuscularity itself*: Nemo
  styling is already explicit per-attribute properties, so map straight into
  `ViewStyle` fields (padding/margin/width/height/colors/radius/…) instead of
  parsing Tailwind. Reuse `nemo-tokens` for spacing/radius/color-role resolution
  (same source-of-truth pattern as design-system-export).
- **Entry point:** wire into `crates/nemo/src/commands/build.rs` as
  `nemo build --target ios|android|tui|web <app.xml>`, emitting View IR JSON and/or
  invoking the crepuscularity shells.

```
BuiltComponent tree ──lower──► crepuscularity_native::ViewIr (JSON v7)
                                     │
        ┌────────────────────────────┼────────────────────────────┐
   SwiftUI/Compose shells       Ratatui (crepus-tui)        HTML/WASM (crepus-web)
```

# Load-bearing caveats

1. **The dynamic half of Nemo does not cross the IR.** View IR is a static
   primitive tree; handlers are opaque action strings the host shell interprets.
   Data sources + RHAI must run somewhere. Two options:
   - **(a) Headless Nemo core on-device** streaming IR patches. Crepuscularity
     ships the transport for exactly this: `crepuscularity_native::hot_reload`
     (`HotReloadEnvelope`) + `IrMutation` patch paths. Nemo's existing
     dirty-flag/`Notify` poll loop that mutates `BuiltComponent.properties` maps
     cleanly onto emitting `IrMutation` patches — but this means porting/embedding
     Nemo's core per platform or exposing it over a socket. This is the real "big
     change".
   - **(b) Static/limited export** — layout + one-shot context, no live data.
     Cheap; weaker for Nemo's data-dashboard use cases.
2. **Component coverage gap.** Charts (`area_chart`), `sidenav_bar`, and complex
   composites have no View IR equivalent; triage which components are portable.
3. **Stability / versioning.** crepuscularity is explicitly pre-1.0 and unstable;
   `IR_VERSION` is already 7 and bumps are breaking. Pin an exact version.
4. **License.** crepuscularity is ISC (permissive; compatible with Nemo's
   MIT/Apache-2.0).
5. **`crepuscularity-abi`** (in-process C session) is for IR *consumers*, not
   producers — only relevant if we later embed a crepuscularity shell inside Nemo.

# Status & next steps

- **Not implemented.** This is a research/design plan.
- **Next (phase 1, bounded, low-risk):** static IR exporter (`nemo-viewir` crate +
  `nemo build --target …`). Additive; does not touch the desktop runtime;
  empirically validates node/style coverage.
- **Next (phase 2, optional, large):** live data — run Nemo's data + RHAI core
  headless and stream `IrMutation` patches to shells. Decide only after phase 1
  shows the target set is worth it.
- **Explicitly rejected:** adopting `.crepus` as an input language; replacing
  `render_component` with `crepuscularity-runtime`.

# Verification

- Phase 1: golden-file tests lowering representative `examples/*/app.xml` to View
  IR JSON (deterministic output, like design-system-export). Round-trip one
  example through a crepuscularity shell (e.g. `crepus native ir` consumers or the
  SwiftUI/Ratatui example shells) and confirm it renders.
- The per-component property→`ViewStyle` coverage matrix falls out of building the
  exporter and is the concrete first artifact.

# Evidence base

- crepuscularity: README, `docs/{runtime,native,polyglot,view-ir-contract}.md`;
  `crepuscularity-runtime/src/renderer.rs` (primitive div/Tailwind runtime);
  `crepuscularity-native/src/ir.rs` (`ViewIr`/`ViewNode`/`ViewStyle`,
  `IR_VERSION=7`).
- Nemo pipeline: `nemo-config/xml_parser.rs`, `nemo-layout/{node,builder,manager,
  binding}.rs`, `nemo/src/{runtime.rs,app.rs}`, `nemo/src/components/{label,panel,
  button}.rs`, `nemo-macros`, `nemo-registry` (ComponentFactory is vestigial;
  tag→GPUI is a compile-time `match` in `app.rs`), `nemo-data`, `nemo-events`,
  `nemo-extension` (RHAI).
