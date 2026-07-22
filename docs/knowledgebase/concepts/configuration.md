---
type: Concept
title: Configuration
description: Nemo's XML configuration format and the parse → resolve → validate pipeline.
tags: [config, xml, core]
timestamp: 2026-07-11T00:00:00Z
---

Nemo configuration is **XML** (`app.xml`). There is no HCL loader in the
codebase — HCL appears only in archived docs and in comments explaining XML
equivalents. See [XML, not HCL](../decisions/xml-not-hcl-config.md).

A config is a `<nemo>` root containing `<app>`, `<variable>`, `<data>`,
`<templates>`, and a `<layout>` component tree:

```xml
<nemo>
  <app><window title="Hello" /><theme name="kanagawa" mode="dark" /></app>
  <layout type="stack">
    <button id="btn" label="Click" on-click="handler" />
  </layout>
</nemo>
```

# Pipeline

`ConfigurationLoader::load()` (`crates/nemo-config/src/loader.rs:31`) orchestrates
three stages, producing a universal `Value` tree
(`crates/nemo-config/src/value.rs`):

1. **Parse** — `XmlParser::parse()` (`xml_parser.rs:39`, quick-xml). Converts
   kebab-case attributes to snake_case, coerces string values to
   bool/int/float/array, preserves `${}` markers, processes `<include>`
   directives, and unwraps the `<nemo>` root into top-level keys. Only
   `[...]`-shaped attribute values are parsed as JSON arrays — see
   [collection properties as JSON-string attributes](../patterns/json-string-collection-properties.md).
2. **Resolve** — `ConfigResolver::resolve()` (`resolver.rs:88`) evaluates `${}`
   expressions against a `ResolveContext` built from `<variable>` blocks.
   Supports `${var.name}`, `${env.KEY}`, and functions `upper`, `lower`, `trim`,
   `length`, `coalesce`, `env`.
3. **Validate** (optional) — `ConfigValidator::validate()` (`validator.rs:22`)
   checks a `Value` against a `ConfigSchema`, returning errors and warnings.

# Single-file components (`.nemo` SFCs)

An `<imports>`/`<import src="…" [as="tag"]>` block — or `<components dir="…"/>`,
which globs every `*.nemo` in a directory — pulls in reusable **single-file
components**: one `.nemo` file bundling markup, styling, and behavior (see
[Single-file components](../patterns/single-file-components.md) for the authoring
pattern). `.nemo` files are **not** wrapped in `<nemo>`; their top-level children
are `<template>` (required, exactly one root element), optional `<style>`,
optional `<script>`, and an optional `<props>` block declaring typed props
(`<prop name type default required/>`).

Parsing/compilation touches three places:

1. **`XmlParser::parse_sfc()`** (`xml_parser.rs`) parses a `.nemo` file into an
   `SfcDefinition { name, template, style, script }`; the template body is
   flattened with the same `process_component_element` used for layout components.
2. **`process_import`** (`xml_parser.rs`) resolves `src` relative to the config's
   `base_dir`, calls `parse_sfc`, and stores the result under the top-level `sfc`
   key as `sfc[tag] = { template, style?, script?, source_path }`. The tag is
   resolved `as=` > `<template name>` > filename stem, then **kebab→snake
   normalized** so it matches how a `<labeled-button>` usage parses (type
   `labeled_button`). The **resolver skips the `sfc` subtree** — SFC bodies use
   bare `${prop}` placeholders on the *runtime* vars/interpolation path, not
   `${var.x}`/`${env.x}` load-time resolution.
3. **`parse_layout_config`** (`runtime.rs`) folds each SFC's `<style>` block onto
   its template nodes as inline attributes (`fold_sfc_styles`; type + `#id`
   selectors, constrained to the universal style attributes), then merges the
   template into the `TemplateMap` keyed by tag (rewriting template-authored bare
   `on_*` handlers to `sfc:<tag>::<fn>` and any nested SFC tags), then
   `rewrite_sfc_tags` converts every `<tag>` usage in the layout into a
   `template = "tag"` instance **before** `expand_children`. From there the
   existing template pipeline (deep-merge, named/default slot injection,
   `__anon`/id scoping) applies unchanged, so an expanded SFC is ordinary
   built-in components by the time it reaches the layout builder.

SFC `<script>` bodies are loaded under `sfc:<tag>` ids in
`load_scripts_from_config`; the single-colon prefix keeps `call_handler`'s
first-`::` split resolving `sfc:<tag>::<fn>` to (script id `sfc:<tag>`, fn).
Declared props' defaults are filled per-instance at expand time (`sfc_node_to_instance`)
for any prop the usage omits. The strict linter treats registered SFC tags as
known component types (skipping `unknown-component`), emits `missing-required`
for an omitted `required` prop, and validates slot usage
(`unknown-slot`/`missing-slot`/`slot-cardinality`) against the SFC's declared
`<slot>`s. `nemo schema --app-config app.xml` synthesizes a `ComponentDescriptor`
per SFC (props → schema, slots → `SlotSpec`) so SFC tags appear in the export.

# Schema and validation

Schemas are defined **programmatically**, not as files (there is no `schema/`
directory of config schemas). `ConfigSchema`/`PropertySchema`/`ValueType`/
`ValidationRule` live in `crates/nemo-config/src/schema.rs`; a thread-safe
`SchemaRegistry` (`registry.rs`) stores them by name. Each component's
`ComponentDescriptor` embeds a `ConfigSchema`, registered in
`crates/nemo-registry/src/builtins.rs`.

`nemo validate --strict` lints components against their schemas
(`crates/nemo/src/commands/validate.rs`). `missing-required` is gated on
`additional_properties == false` (permissive builtin schemas don't declare
requireds reliably). `unknown-attribute` runs on all schemas; it skips
structural keys (`type`/`component`/`binding`/`slot`/`vars`/`template`),
handler prefixes (`on_*`), binding prefixes (`bind_*`), and **universal styling
attributes** (`width`/`height`/`margin*`/`padding*`/`border*`/`shadow`/`rounded`/
`background`/etc.) applied by `apply_layout_styles` to every component wrapper.
Custom state attributes set by Rhai handlers via `set_component_property` are
not enumerated in schemas and will produce a warning (by design — they are
indistinguishable from typos at parse time). `invalid-value` warns when a
present, literal property value violates the property's `one_of` enum rule (e.g.
`variant="bogus"`); it skips unresolved `${...}` expressions and non-string
values. These enum rules (`variant`/`size` on `button`/`alert`/`tag`/`tabs`/
`dropdown_button`/`spinner`/`label`) are the same ones the design-system export
reads.

The universal-attribute allowlist and the structural top-level elements are
single-sourced in `nemo_registry::schema_surface` (`universal_style_attributes`,
`attribute_families`, `structural_elements`), consumed by both the linter and the
`nemo schema` exporter so they cannot drift. (This replaced a hand-duplicated
list in `validate.rs` that had gone stale.)

`nemo schema` exports the whole config surface — every component/data-source/
transform/action schema plus the universal/structural surface — as nemo-native
JSON generated from the compiled registries, so it's always current with the
binary. See [config schema export](../references/config-schema-export.md).

# Config → components → layout

1. **Registry lookup** — the `ComponentRegistry` maps type names to
   `ComponentDescriptor`s (built-ins via `register_all_builtins()`).
2. **Build** — `LayoutBuilder::build()` (`crates/nemo-layout/src/builder.rs:38`)
   walks the config, verifies each type exists, resolves and validates
   properties, and produces a `BuildResult` tree.
3. **Apply** — `LayoutManager::apply_layout()` (`crates/nemo-layout/src/manager.rs:68`)
   converts `BuildResult` into a `BuiltComponent` tree keyed by component ID, and
   sets up bindings.

**Component IDs must be unique document-wide.** Because `apply_layout` stores
components in a *flat* `HashMap` keyed by ID (and parents reference children by
ID), two components sharing an ID collapse into one — the last one wins, and
every reference resolves to it. Id-less ("anonymous") elements are therefore
given generated ids: `children_to_component_map` (`xml_parser.rs`) assigns
`__anon_N` from a **document-global** counter (not per-parent), and
`LayoutNode::effective_id` (`node.rs`) falls back to `type_<counter>`. A past bug
reset the anon counter per parent, so the first id-less child of every parent
became `__anon_1` and they all collapsed (symptom: every id-less label in the
dev-dashboard rendered the last one's text). See
`test_anonymous_components_get_document_unique_ids` and
`test_anonymous_labels_survive_full_build_pipeline`.

# Data binding in config

`<binding source="data.path" target="property" transform="expr" />` maps a
`DataRepository` path to a component property. Bindings are modeled by
`BindingSpec` (`node.rs`) and managed by the `BindingManager`
(`crates/nemo-layout/src/binding.rs`); modes are `OneWay` (default), `TwoWay`,
`OneTime`. See [Data flow](data-flow.md).

# Two config layers (settings persistence)

There are **two** independent configuration stores, both surfaced in the
settings view (`ctrl+p`, `crates/nemo/src/workspace/settings.rs`):

1. **Global** — a TOML file at `~/.config/nemo/config.toml`, deserialized into
   `NemoConfig` (`crates/nemo/src/config/`). Holds cross-project user prefs:
   `app.theme_name`, `app.theme_mode`, `app.font_family`. Writable via
   `NemoConfig::save()`. Applied at startup in `main.rs`.
2. **Project** — the per-project `app.xml` (`<app><theme name mode/></app>`),
   read via `runtime.get_config("app.theme.name" | ".mode")`. The **project
   layer wins**: `apply_theme_from_runtime` (`workspace/utils.rs`) re-applies the
   XML theme after the global one, so a project's `<theme>` overrides the global
   default. If neither layer sets a theme, gpui-component's built-in default is
   used.

The settings view has a **Global** page (persists to `config.toml` via
`NemoConfig::save()`) and a **Project** page (persists to the loaded `app.xml`).
The runtime config is read-only in memory (`set_config` is a no-op), so project
edits are written straight to disk by `xml_edit::set_app_theme`
(`crates/nemo/src/workspace/xml_edit.rs`) — a **surgical text edit** that updates
only the `<theme>` element's `name`/`mode` attributes (or inserts a `<theme>`
under `<app>` if absent), preserving the rest of the hand-authored file. A future
option is an `overrides.xml` overlay to keep `app.xml` fully immutable; not yet
implemented. Theme values are matched case-insensitively against the theme *set*
names from `crates/nemo/src/theme/*.json` (`theme::get_theme_set_names`).

# Project manifest (`nemo.toml`)

A project may carry an **optional, additive** manifest at its root, parsed by
`nemo-config`'s `manifest.rs` into `ProjectManifest { name, entry, build,
dependencies }` (re-exported from `lib.rs`). It is distinct from the global
`config.toml` above: `config.toml` is cross-project user prefs; `nemo.toml` is
the per-project build/dependency manifest.

```toml
name  = "foo"
entry = "app.xml"          # default "app.xml"

[build]
out  = "dist"              # default "dist"
load = "source"            # "source" (default) | "dist"

[dependencies]             # remote component libraries (resolution: later phase)
"github.com/geoffjay/nemo-components" = "v1.2.0"
```

* `find_project_root(start)` walks up from a file or directory to the nearest
  `nemo.toml` (the constant `MANIFEST_FILE`), returning the directory that holds
  it — the project's only root marker.
* **Manifest-aware launch** (`main.rs` `resolve_app_config_via_manifest`): when
  `--app-config` is a **directory** or is **omitted**, the launcher resolves the
  entry via the nearest manifest (`<root>/<entry>`). An explicit **file** path is
  used unchanged, so existing `nemo --app-config app.xml` invocations are
  untouched; when omitted with no manifest in scope, the project-loader screen
  still shows. Manifest read/parse errors degrade gracefully to the loader on the
  run path.
* **`nemo build`** (`commands/build.rs`):
  * `nemo build <file.nemo>` compiles one component to a JSON artifact at
    `<out>/components/<tag>.json` — reusing `parse_sfc` then the runtime's own
    `fold_sfc_styles` + `rewrite_sfc_handlers` ahead-of-time, so the artifact's
    `template` equals the `TemplateMap` entry the runtime builds from source.
  * A `[package] exports = [...]` table (`PackageConfig`) marks a **component
    library**; `nemo build <dir>` compiles each exported component (or, with no
    `exports`, every top-level `.nemo`).
  * A plain app project (no `[package]`) prints a dry-run plan — compiling a
    project to a loadable `dist/` tree is Phase 2. `[build] load = "dist"` and
    remote `[dependencies]` are recorded but not yet acted on; `nemo dev` never
    uses `dist`. See the [build-system plan](../plans/build-system.md).

# Project-level custom themes

Beyond selecting a shipped theme, a project can **define its own themes** and
**override individual colors** — see `examples/custom-theme/`.

* **Register full custom themes** — a top-level `<themes>` block references
  external JSON files: `<themes><theme-set src="themes/aurora.json" /></themes>`.
  The files use the exact same gpui-component `ThemeSet` schema as the bundled
  themes (`crates/nemo/src/theme/*.json`), so authoring one means copying a
  shipped theme and editing colors. The parser (`xml_parser::process_themes_block`)
  only records the `src` paths into `themes` (an array); it does **not** parse the
  JSON (that needs the gpui-component `ThemeSet` type, which lives in the `nemo`
  crate). `apply_theme_from_runtime` reads `themes`, resolves each `src` relative
  to the config dir, and calls `theme::register_project_theme_sets`, which loads
  them into a **`PROJECT_THEME_SETS` overlay** (`crates/nemo/src/theme/theme.rs`).
  The overlay is consulted **before** the baked-in `THEMES`/`THEME_SETS` statics
  by `resolve_theme`/`resolve_theme_pair`/`get_theme_set_names`, so a project can
  add new themes *or* fully replace a shipped one by reusing its set name, and
  custom themes appear in the settings picker. The overlay is cleared and
  re-registered on every load (untrusted input: missing/malformed files are logged
  and skipped, not `.unwrap()`'d).

* **Override individual colors** — a `<theme>` may carry an `<extend>` block:
  `<theme name="nord"><extend><color key="primary.background" value="#ff6600" />
  </extend></theme>`. `process_theme_block` flattens these into
  `app.theme.extend = { "primary.background": "#ff6600", ... }`;
  `apply_theme_from_runtime` builds a `ThemeConfigColors` from it and passes it to
  `apply_configured_theme`, where `merge_theme_config_colors` merges it over the
  resolved base theme (overrides always win). (Both `<themes>`/`<theme-set>` and
  `<extend>`/`<color>` are registered in `nemo_registry::schema_surface` so
  `nemo validate` doesn't flag them and `nemo schema` publishes them.)
