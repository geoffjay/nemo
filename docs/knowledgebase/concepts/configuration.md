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
indistinguishable from typos at parse time).

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
