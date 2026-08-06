---
type: Plan
title: "`app.nemo` SFC as the project entry"
description: Make the application entry file a `.nemo` SFC (not `app.xml`), compiled at build time. Extends the SFC structure with app-level blocks (`<app>`, `<data>`, `<imports>`, `<variable>`), changes the manifest default, wires compile-into-load, and migrates the toolchain (build, dev, validate, schema, settings, templates).
tags: [config, sfc, build, runtime, cli, planning]
timestamp: 2026-08-05T00:00:00Z
---

# `app.nemo` SFC as the project entry

The project entry file is `app.nemo` — a single-file component using the same
`<template>`/`<props>`/`<style>`/`<script>` structure as component `.nemo` files,
extended with app-level blocks. It is compiled at build time; the runtime loads
the compiled output (today's `dist/layout.json`), not the source.

See [the decision](../decisions/app-nemo-sfc-entry.md).

```nemo
<!-- app.nemo -->
<app title="My Dashboard">
  <window title="My Dashboard" width="1200" height="800">
    <header-bar github-url="https://github.com/user/repo" theme-toggle="true" />
  </window>
  <theme name="nord" mode="dark" />
</app>

<imports>
  <import src="./components/card.nemo" />
</imports>

<data>
  <source name="api" type="http" url="https://api.example.com" interval="30" />
</data>

<variable name="refresh_interval" type="string" default="30" />

<template name="app">
  <stack id="root" direction="vertical" spacing="20" padding="32">
    <label id="title" text="Dashboard" size="xl" />
    <card>
      <label slot="header" text="Users" />
      <text content="User list goes here" />
    </card>
  </stack>
</template>

<script>
fn init(component_id, event_data) {
    // app-level on-load handler
}
</script>

<style>
stack { padding: 20px; }
</style>
```

# Decisions (settled)

See [`app.nemo` SFC entry decision](../decisions/app-nemo-sfc-entry.md).

* **`app.nemo` is an SFC.** It uses `<template>`/`<props>`/`<style>`/`<script>`,
  extended with app-level blocks. Not a `<nemo>` XML document.
* **Build output is not XML.** The compiled output is the resolved `Value` tree
  (today's `dist/layout.json`), loaded via `load_from_dist`. A binary format is
  a later optimization. No `Value`→XML serializer is built.
* **App-level blocks:** `<app>`, `<data>`, `<imports>`, `<variable>` are
  optional top-level blocks alongside `<props>`/`<template>`/`<style>`/`<script>`.
  The compiler maps them to the same `Value` tree keys `process_root` produces
  today.
* **`app.xml` entries are hard-deprecated.** `ConfigurationLoader::load` rejects
  any non-`.nemo` entry with `ConfigError::DeprecatedXmlEntry`. XML remains valid
  only inside `<include>` fragments and the `overrides.xml` settings overlay —
  never as an application entry. The default and the *only supported* entry is
  `app.nemo`.

# Why

Today the entry file is `app.xml`, a `<nemo>` XML document parsed by
`process_root()` — a different code path from the SFC pipeline
(`parse_sfc()`). This split means:
* Two authoring formats: `app.xml` (XML with `<nemo>` root) and `.nemo` (SFC
  with `<template>`/`<style>`/`<script>`).
* Control-flow directives (`n:for`/`n:if`) work in `.nemo` SFCs but not in
  `app.xml` — the app layout can't use them.
* The `config-dev-env.md` plan stated the end goal: "all configuration files
  are nemo SFC files that get built and loaded."

Unifying on `.nemo` means one authoring format, directives in the app layout,
and the build system as the single entry point (compile → `dist/` → load).

# App-level blocks

The SFC structure is extended with optional top-level blocks that the compiler
maps to `Value` tree keys. Today's `process_root` arms map to these blocks:

| Block | `process_root` arm today | `Value` key |
|-------|--------------------------|-------------|
| `<app>` | `process_app` | `app` |
| `<data>` | `process_data` | `data` |
| `<imports>` | `process_import` | `sfc` |
| `<variable>` | `process_variable` | `variable` |
| `<template>` | (layout) | `layout` |
| `<script>` | `process_script` | `scripts` |
| `<props>` | (SFC props) | `sfc[tag].props` |

The compiler runs the same `process_*` logic on these blocks — it produces the
same `Value` tree `process_root` produces today. The runtime's
`parse_layout_config` and everything downstream is unchanged.

**`<app>` carries window/theme/header-bar metadata.** Its children
(`<window>`, `<theme>`, `<header-bar>`) are processed by `process_app` exactly
as today. The `<template>` is the layout tree (what today lives under
`<layout>` in `app.xml`).

# Phasing

## Phase 1 — Extend `SfcDefinition` with app-level blocks

**Status: implemented.**

`parse_sfc` (`xml_parser.rs:842`) gains handling for `<app>`, `<data>`,
`<imports>`, `<variable>` top-level blocks. `SfcDefinition` gains fields:
`app: Option<Value>`, `data: Option<Value>`, `imports: Value` (SFC map),
`variables: Value` (variable map). The compiler (a new
`sfc_definition_to_app_value` or extension of `sfc_definition_to_value`)
produces the same `Value` tree `process_root` produces.

* Extend `SfcDefinition` with `app`, `data`, `variables` fields.
* Extend `parse_sfc` to recognize and process the new blocks using the
  existing `process_app`/`process_data`/`process_variable`/`process_import`
  logic.
* New `compile_app_sfc(content) -> Result<Value>` (in `nemo-config`): parse +
  compile an `app.nemo` to the resolved `Value` tree. This is the entry-point
  equivalent of `ConfigurationLoader::load` but for SFC source.
* **Verify:** an `app.nemo` compiles to the same `Value` tree as the equivalent
  `app.xml` (round-trip equality on a fixture).

## Phase 2 — Manifest default + build compiles `app.nemo`

**Status: implemented.**

* `manifest.rs`: `default_entry()` returns `"app.nemo"` (was `"app.xml"`).
  Tests updated; existing `entry = "app.xml"` projects keep working.
* `build_project` (`commands/build.rs:117`): calls `ConfigurationLoader::load`,
  which now detects a `.nemo` entry and compiles it via `compile_app_sfc`; the
  resolved `Value` is serialized to `dist/layout.json` (same as today).
* `build_single_component` already handles `.nemo` files — unchanged.
* **Verify:** `nemo build` on a project with `app.nemo` produces
  `dist/layout.json`; `--dist` loads it identically to source.

## Phase 3 — Runtime `load_config` accepts `.nemo`

**Status: implemented.**

`NemoRuntime::load_config` (`runtime.rs:246`) branches on `.json` (dist) vs
everything else, calling `ConfigurationLoader::load`. The `.nemo` detection
lives in `ConfigurationLoader::load` (loader.rs), so `load_config` works
unchanged — the SFC is compiled to a `Value` tree via `compile_app_sfc`, then
proceeds as today. All callers (`build`, `validate`, `schema`, `load_config`)
share this single dispatch point.

* `ConfigurationLoader::load` gains a `.nemo` extension check: dispatches to a
  new `load_nemo_string` (mirrors `load_xml_string` but calls
  `compile_app_sfc`), then runs the same directive-compile + `${}` resolution.
* The `resolve_app_config_via_manifest` path (`main.rs:219`) already returns
  `<root>/<manifest.entry>` — no change needed; the extension detection happens
  in `ConfigurationLoader::load`.
* **Verify:** `nemo --app-config app.nemo` launches the app; `nemo dev
  --app-config app.nemo` hot-reloads on save.

## Phase 4 — Dev/watch recompiles `.nemo` on save

**Status: implemented.**

`nemo dev` (`commands/dev.rs`) watches `app.xml` and `.rhai` files. The
watcher now also accepts `.nemo` (app entry and `<import>`ed components); on
change, the SFC tree is recompiled to `Value`, then reloaded. The reload path
(`reload_config` → `create_runtime` → `load_config`) already compiles `.nemo`
via Phase 3, so the only change was admitting `.nemo` into the watch filter.

* `path_is_watchable` (`workspace/mod.rs:560`): extended its extension match to
  include `nemo` alongside `xml`/`rhai`/`toml`. Temp/hidden/VCS exclusions apply
  unchanged. The `dev.rs` "requires --app-config" message now mentions `app.nemo`.
* `Workspace::reload_config` → `create_runtime` → `load_config`: if the config
  path is `.nemo`, `load_config` compiles it (Phase 3). No additional change
  needed beyond Phase 3.
* Performance: compile-on-every-save must stay fast for hot-reload (~0.7s
  target). `compile_app_sfc` is the same parse + resolve `load` does today,
  plus the raw-text splitter and (later) directive expansion — all pure
  string/`Value` operations, no I/O beyond `<import>` reads. Profile if slow.
* **Verify:** edit `app.nemo` while `nemo dev` is running; the app hot-reloads
  within the debounce window.

## Phase 5 — Validate/schema accept `.nemo`

**Status: implemented.**

`nemo validate` (`commands/validate.rs`) and `nemo schema`
(`commands/schema.rs`) both already called `ConfigurationLoader::load(path)`,
which Phase 3 wired to dispatch `.nemo` to `load_nemo_string` →
`compile_app_sfc`. No new detection code was needed — the single dispatch point
in `ConfigurationLoader::load` serves every caller.

* `validate::run`: calls `loader.load(path)`, which compiles `.nemo` to the
  `Value` tree, then runs the same validation logic. The validation itself
  operates on the `Value` tree — unchanged.
* `schema::run`: calls `loader.load(path)`, then `register_sfc_descriptors`
  synthesizes a descriptor for each SFC tag in the compiled config.
* Linter (`validate.rs`): `unknown-attribute` already skips `n:`-prefixed
  attributes (directives) at `validate.rs:397` — verified.
* **Verify:** `nemo validate app.nemo --strict` passes on a fixture; `nemo schema
  --app-config app.nemo` includes SFC descriptors.

## Phase 6 — Settings persistence

**Status: implemented.**

`xml_edit.rs` does surgical text edits to `app.xml` to persist theme choices.
For `app.nemo`, the text-edit approach is fragile against SFC syntax (raw-text
blocks, directives, single-root constraint). Options:

* **`overrides.xml` overlay** — keep `app.nemo` immutable, write settings to a
  separate `overrides.xml` (or `overrides.nemo`) file that's compiled/merged at
  load. Clean separation, works with any entry format. Already mentioned as a
  "future option" in `configuration.md:216`.
* **Adapt `xml_edit` to `.nemo`** — extend the text editor to handle SFC
  structure. Fragile.
* **Move project settings to `nemo.toml`** — add a `[settings]` table. Simplest,
  but conflates build config with runtime prefs.

**Decision:** `overrides.xml` overlay — see the
[settings-overlay decision](../decisions/settings-overrides-xml.md).
`xml_edit::set_app_theme` now writes to `<entry_dir>/overrides.xml` (creating
or updating it), never the entry file. `NemoRuntime::load_config` merges the
overlay's `app` key over the entry's at load time (shallow merge). Only the
runtime applies the overlay — `nemo build`/`validate`/`schema` operate on the
source entry so `dist/` stays a faithful compile. Works for both `.nemo` and
`.xml` entries.

## Phase 7 — Templates, examples, docs migration

**Status: implemented.**

* `nemo new` templates (`crates/nemo/templates/`): all four (`basic`,
  `calculator`, `data-binding`, `complete`) are `app.nemo` SFCs; each has a
  `nemo.toml`. `new.rs` `tfile!` paths, `render_readme`, and the "Next steps"
  message reference `app.nemo`. The `complete` template keeps its multi-file
  `<include href="templates/*.nemo"/>` — the `<include>` and `<templates>` arms
  were added to `parse_sfc` (delegating to the existing `process_include`/
  `process_template`), and `AppBlocks` gained an `extra` passthrough so
  include-merged / `<templates>` keys reach the compiled `Value`. The
  `<include>`-target files are `.nemo` merge fragments (bare top-level
  `<templates>`/`<data>`; `process_include` parses them via the same
  `<nemo>`-unwrapping `parse` path, so no `<nemo>` wrapper is required).
* `project_loader.rs` `CONFIG_CANDIDATES`: `["app.nemo", ".nemo/app.nemo"]` —
  no `app.xml` candidates.
* `dev_panel.rs` auto-load: `app.nemo` only (no `app.xml` fallback).
* Examples: all 14 examples migrated to `app.nemo`; every `app.xml` entry
  deleted. `examples/complete` (`<include>`) and `examples/components`
  (`<templates>`) exercise the new SFC top-level blocks.
* `schema/nemo.xsd`: **deleted** — it described the obsolete `<nemo>` entry
  document. SFC authoring assistance will come from the planned `nemo-lsp`.
* Docs: updated `configuration.md` (entry-file + settings-overlay sections),
  `architecture.md`, the `nemo-xml-reference` skill, `README.md`.

# Critical files

| File | Role |
|---|---|
| `crates/nemo-config/src/xml_parser.rs` | `parse_sfc` app-level blocks; `compile_app_sfc` (new) |
| `crates/nemo-config/src/manifest.rs` | `default_entry` → `app.nemo` |
| `crates/nemo-config/src/loader.rs` | `load` detects `.nemo`, calls `compile_app_sfc` |
| `crates/nemo/src/commands/build.rs` | `build_project` compiles `.nemo` entry |
| `crates/nemo/src/runtime.rs` | `load_config` `.nemo` branch |
| `crates/nemo/src/commands/validate.rs` | `.nemo` detection; `n:`-attribute linter skip |
| `crates/nemo/src/commands/schema.rs` | `.nemo` detection |
| `crates/nemo/src/commands/new.rs` | templates renamed to `.nemo` |
| `crates/nemo/src/workspace/project_loader.rs` | `CONFIG_CANDIDATES` |
| `crates/nemo/src/workspace/dev_panel.rs` | auto-load path |
| `crates/nemo/src/workspace/xml_edit.rs` | settings persistence (Phase 6) |

# Reuse (avoid new code)

* `process_app`/`process_data`/`process_variable`/`process_import`
  (`xml_parser.rs`) — the compiler calls these same functions on the app-level
  blocks. No new processing logic.
* `ConfigurationLoader::load_from_dist` — the compiled `Value` tree is loaded
  the same way today's `dist/layout.json` is loaded. No new load path.
* `resolve_app_config_via_manifest` (`main.rs:219`) — generic path resolution,
  works with any entry extension.
* `nemo build`'s existing `build_project` — the serialize-to-`dist/layout.json`
  step is unchanged; only the input detection changes.
* `path_is_watchable` — already accepts `.nemo`; no watcher change needed.

# Verification

* **Phase 1:** ✅ an `app.nemo` compiles to the same `Value` tree as the
  equivalent `app.xml` (round-trip equality on a fixture covering `<app>`,
  `<data>`, `<imports>`, `<variable>`, `<template>`, `<script>`).
* **Phase 2:** `nemo build` on an `app.nemo` project produces `dist/layout.json`;
  `--dist` loads it identically to source.
* **Phase 3:** `nemo --app-config app.nemo` launches; `nemo dev --app-config
  app.nemo` hot-reloads.
* **Phase 4:** editing `app.nemo` during `nemo dev` hot-reloads within debounce.
* **Phase 5:** `nemo validate app.nemo --strict` passes; `nemo schema
  --app-config app.nemo` includes SFC descriptors.
* **Phase 6:** settings changes persist and survive reload (via overlay or
  chosen mechanism).
* **Phase 7:** `nemo new --template basic my-app` scaffolds `app.nemo` +
  `nemo.toml`; `nemo validate` passes on the scaffold.
* **Regression:** a legacy `app.xml` entry (explicit path or manifest `entry =
  "app.xml"`) is rejected with `DeprecatedXmlEntry`, not silently loaded;
  `<include>` fragments and `overrides.xml` still parse as XML.

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — rewrite the entry-file
  section: `app.nemo` is the default, `app.xml` is legacy. Document app-level
  blocks.
* [Single-file components](../patterns/single-file-components.md) — add the
  app-level blocks and `app.nemo` authoring pattern.
* [Architecture](../concepts/architecture.md) — update the load path to note
  the compile step.
* The `nemo-xml-reference` skill — add `app.nemo` authoring; soften the "XML"
  framing.
* This plan — mark phases as implemented.

# Relationship to other plans

* **Depends on** [raw-text `.nemo` parser](sfc-raw-text-parser.md) — `.nemo`
  authoring needs raw-text blocks (no CDATA).
* **Depends on** [control-flow directives](control-flow-directives.md) Phase 1
  (`n:if`) at minimum — `app.nemo`'s `<template>` should support directives.
* **Independent of** [runtime component creation](runtime-component-creation.md)
  — the entry-format change doesn't depend on runtime list expansion. But
  `app.nemo` with live-data `n:for` requires both.
* **Builds on** the [build system](build-system.md) — reuses `nemo build` →
  `dist/layout.json` → `load_from_dist`.