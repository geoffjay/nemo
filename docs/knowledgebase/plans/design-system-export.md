---
type: Plan
title: Design-system export (pencil.dev intermediate)
description: `cargo xtask design-export` emits nemo's tokens + themes + component structure as a faithful, gpui-free JSON intermediate that maps onto pencil.dev .pen concepts.
tags: [design-system, export, pencil, xtask, tokens]
timestamp: 2026-07-18T00:00:00Z
---

# Why

Phase 3 of the design-system initiative: make nemo's look consumable by
application developers as a design file. pencil.dev's `.pen` format is code-first
and token-centric (design tokens + reusable components with slots/states/
variants), driven mainly through a Pencil MCP server + a Claude Code skill. Rather
than couple nemo to that toolchain now, we emit a **faithful, well-documented JSON
intermediate**; the actual `.pen` conversion is a later skill/MCP step.

# Why `cargo xtask`, not a `nemo` subcommand

The export is pure data generation (registry metadata + token values + theme
JSON) — it never launches the app — and is only useful during development. Putting
it in the shipped `nemo` CLI would bloat the end-user surface. So it lives in the
dev-only **`xtask`** crate (`cargo xtask design-export`, aliased in
`.cargo/config.toml`). (Screenshots differ — they must launch the real GPUI
window — so `nemo screenshot` stays a binary subcommand.)

`xtask` depends only on gpui-free crates (`nemo-tokens`, `nemo-registry`,
`nemo-config`, serde), so it compiles fast and pulls in no gpui.

# Single source of truth: `nemo-tokens`

The design tokens were extracted from the `nemo` binary into a new gpui-free
crate **`nemo-tokens`** (spacing/radius/typography scales, `SEMANTIC_COLOR_ROLES`,
`resolve_role_alias`, `RADIUS_NAMES`). The app's `theme::tokens` re-exports it and
adds the gpui-coupled render helpers (`space()`, `font_size()`, `TokenStyled`).
Both the live UI and the exporter read the same data, so they can't drift. See
[design tokens](design-tokens.md).

# What the export contains (`xtask/src/design_export.rs`)

```
{ nemoVersion, note,
  tokens: { spacing{}, radius{}, typography{name:{size,lineHeight}}, colorRoles[{role,field}] },
  themes: [ { id, name, variants:[ { name, mode, colors{field:hex} } ] } ],
  components: [ { name, category, displayName, description, variants[], sizes[], states[], slots[], properties[] } ] }
```

* **tokens** — serialized from `nemo-tokens`. `colorRoles` is the role→theme-field
  mapping (e.g. `surface`→`background`).
* **themes** — parsed verbatim from `crates/nemo/src/theme/*.json` (all 7 sets,
  light/dark variants). Dotted color keys (`muted.foreground`) are normalized to
  the snake_case field names roles reference (`muted_foreground`), keeping only
  string (hex) values. A role resolves as `theme.colors[field]`. **Limitation:**
  fields absent from a theme JSON (gpui-component defaults, e.g. `secondary`,
  `popover`) are simply not present — resolving those gpui-free would require the
  gpui-component default theme.
* **components** — from `nemo-registry` (same source as `nemo schema`): name,
  category, display name, description, slots, and full property schemas
  (type/default/enum/min/max/required). `states` is a small static per-category
  vocabulary. `variants`/`sizes` are derived from the component's `variant`/`size`
  `one_of` enum rules. These are now populated for the annotated components:
  `button` (7 variants, 5 sizes), `alert`/`tag`/`tabs`/`dropdown_button`
  (variants), `spinner`/`label` (sizes). Other components have none until their
  registry schema gains a `variant`/`size` `one_of`. The enum values live in
  `nemo-registry::builtins` (see the `enum_vals` helper) and are enforced by
  `nemo validate --strict`'s `invalid-value` lint.

Output is deterministic (BTreeMaps + sorted components/themes) so it can be
committed and diffed.

# Status & next steps

* **Done:** `nemo-tokens` extraction, `xtask` crate + `cargo xtask design-export`,
  the JSON export (tokens + 7 themes + 64 components), 4 export unit tests, clippy
  clean, no Cargo.lock drift.
* **Done:** registry `variant`/`size` `one_of` annotations, so `variants`/`sizes`
  populate in the export; enforced by a new `invalid-value` strict validate lint.
* **Next (optional):** (1) extend `one_of` annotations to more components/props as
  they gain enums (shared win with the LSP/gallery roadmap); (2) the `.pen`
  conversion skill/MCP step that turns this intermediate into a `.pen` file;
  (3) optionally fold gpui-component default colors in so absent theme fields
  resolve.

# Verification

`cargo xtask design-export --output out.json`; `cargo test -p xtask` (tokens/
themes/components present, colors normalized, deterministic valid JSON).
