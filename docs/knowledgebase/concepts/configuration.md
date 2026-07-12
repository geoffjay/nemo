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

# Config → components → layout

1. **Registry lookup** — the `ComponentRegistry` maps type names to
   `ComponentDescriptor`s (built-ins via `register_all_builtins()`).
2. **Build** — `LayoutBuilder::build()` (`crates/nemo-layout/src/builder.rs:38`)
   walks the config, verifies each type exists, resolves and validates
   properties, and produces a `BuildResult` tree.
3. **Apply** — `LayoutManager::apply_layout()` (`crates/nemo-layout/src/manager.rs:68`)
   converts `BuildResult` into a `BuiltComponent` tree keyed by component ID, and
   sets up bindings.

# Data binding in config

`<binding source="data.path" target="property" transform="expr" />` maps a
`DataRepository` path to a component property. Bindings are modeled by
`BindingSpec` (`node.rs`) and managed by the `BindingManager`
(`crates/nemo-layout/src/binding.rs`); modes are `OneWay` (default), `TwoWay`,
`OneTime`. See [Data flow](data-flow.md).
