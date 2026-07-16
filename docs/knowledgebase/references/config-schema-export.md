---
type: Reference
title: Configuration schema export (`nemo schema`)
description: The nemo-native JSON schema emitted from the compiled registries, and its shape.
tags: [schema, cli, tooling, configuration]
timestamp: 2026-07-16T00:00:00Z
---

`nemo schema` exports a machine-readable description of the XML configuration
surface **for the exact binary that produced it**. It is generated from the
in-memory registries (`register_all_builtins`) plus the canonical out-of-registry
surface in `nemo_registry::schema_surface`, so it never drifts and needs no
hand-maintenance — important while the config API is pre-v1.

Implementation: `crates/nemo/src/commands/schema.rs` (DTOs + mapping),
`crates/nemo-registry/src/schema_surface.rs` (universal attributes, attribute
families, structural elements). Wired via the `Schema` subcommand in
`crates/nemo/src/args.rs` and dispatched in `main.rs`.

# Usage

```sh
nemo schema                 # pretty JSON to stdout
nemo schema --compact       # single-line JSON
nemo schema -o schema.json  # write to a file
```

Output is **deterministic** (components/data-sources/transforms/actions sorted by
name; `properties` keep schema declaration order via `IndexMap`; no timestamp),
so it can be committed and diff-checked in CI.

# Shape (nemo-native JSON)

```jsonc
{
  "nemoVersion": "0.7.0",
  "universalAttributes": [ { "name": "max_width", "type": "integer", "description": "…" }, … ],
  "attributeFamilies": [ { "prefix": "on-",  "description": "…" }, { "prefix": "bind-", "description": "…" } ],
  "structural":  [ { "element": "app", "description": "…", "attributes": [ … ], "childElements": ["window","theme","plugins"] }, … ],
  "components":  [ { "name", "category", "displayName", "description",
                     "properties": [ { "name", "type", "default?", "enum?", "min?", "max?", "required?" } ],
                     "events": [], "bindableProperties": [], "slots": [], "allowedChildren": [] }, … ],
  "dataSources": [ { "name", "displayName", "description", "properties": [ … ], "capabilities": { "polling", "streaming", "manualRefresh" } }, … ],
  "transforms":  [ { "name", "displayName", "description", "properties": [ … ] }, … ],
  "actions":     [ { "name", "displayName", "description", "properties": [ … ] }, … ]
}
```

Names use the internal snake_case form (XML `max-width`/`on-load` normalize to
`max_width`/`on_load` at parse). Property `type` is the coarse `ValueType`
(`string`/`integer`/`float`/`boolean`/`array`/`object`/`any`).

# Phase 1 caveat

The exporter maps the full `PropertySchema` surface, but the builtins are sparse
today, so in the current output:

- `enum`/`min`/`max` are **absent for almost every property** — no builtin calls
  `.one_of(...)`/`.min(...)`/`.max(...)` yet (enum-like props such as `align`,
  `variant`, `size`, `direction` are bare strings).
- `events`, `bindableProperties`, `slots`, and `allowedChildren` are **empty** —
  the `EventSpec`/`BindableProperty`/`SlotSpec` metadata and containment aren't
  populated.

The JSON *shape* is stable, so consumers can code against it now. **Phase 2**
(macro-derived per-property schema from `#[derive(NemoComponent)]`, enum/range
annotations, and a containment table) fills that content. This export is the
planned feedstock for `nemo-lsp`, the schema-driven gallery, the LLM
`nemo generate` prompt, and the docs (see [roadmap](../plans/roadmap.md)).

# Single-sourced universal attributes

`schema_surface::universal_style_attributes()` is the one canonical list of the
style attributes `apply_layout_styles` applies to every component. Both the
`nemo validate` `unknown-attribute` linter and this exporter consume it, so they
can't drift. (It replaced a hand-duplicated list in `validate.rs` that had gone
stale — missing `max_width`/`max_height`/`scroll`, which caused false
`--strict` warnings.)
