---
type: Plan
title: Raw-text `.nemo` parser (drop the CDATA requirement)
description: A parser-layer pre-splitter in front of quick-xml that treats `<script>`/`<style>` as HTML-style raw-text elements, so `.nemo` SFC bodies no longer need `<![CDATA[…]]>`. Independent of every other SFC/build phase.
tags: [config, sfc, parsing, planning]
status: implemented
---


> **Status: implemented.** `split_sfc_blocks` (`xml_parser.rs`) pre-splits
> `<script>`/`<style>` as raw-text before `quick-xml`; `parse_sfc` consumes the
> split. The `examples/sfc/*.nemo` files are the definitive CDATA-free reference.
> CDATA wrappers are still tolerated (stripped) for backward compatibility.

Today a `.nemo` single-file component is *pure XML*: it is parsed by `quick-xml`
(`crates/nemo-config/src/xml_parser.rs`), a general XML reader. XML has no
*raw-text elements*, so any `<` or `&` inside a `<style>`/`<script>` body — Rhai
`&&`, generics, a CSS `>` combinator — is treated as markup unless wrapped in
`<![CDATA[ … ]]>`. This plan removes that requirement so `.nemo` files read like
Vue single-file components rather than XML-with-extra-steps.

This was originally [SFC Phase 6](sfc-components.md#phase-6--raw-text-nemo-parser-drop-the-cdata-requirement);
it is promoted to its own plan because it is a **parser-layer** change with no
ordering dependency on any other SFC or build phase — the SFC feature phases
(0–4) and the [build system](build-system.md) are all implemented, and this can
land independently before or after any of them.

# Why (and why `nemo build` does not fix it)

The CDATA rule is a property of the **parser**, not the load-vs-build split.
[`nemo build`](build-system.md) reuses `XmlParser::parse_sfc` verbatim and runs
the same `Value`-tree transforms ahead-of-time, so the CDATA requirement is
upstream of the whole build pipeline — building a project does not change how a
`.nemo` file is authored. Dropping CDATA is therefore its own concern.

**Root cause.** `parse_sfc` (`xml_parser.rs:789`) hands the entire file to
`parse_element`, which drives `quick-xml`. A `<script>`/`<style>` body is only
captured into `__cdata__` (`:1520`) as a single text/CDATA run, and any `<`/`&`
in it must already be escaped or CDATA-wrapped or the XML reader errors. HTML, by
contrast, designates `<script>` and `<style>` as **raw-text elements** whose
contents are read verbatim to the matching close tag — which is exactly why Vue's
`@vue/compiler-sfc` (an HTML-flavored parser) never needs CDATA.

A secondary limitation falls out of the same root cause: because only the first
non-empty text run is kept (`:1494`), a `<script>` body must currently be one
contiguous block (documented in [single-file components](../patterns/single-file-components.md)
"Rules that bite"). Raw-text capture dissolves this too.

# Design — a raw-text pre-splitter in front of `quick-xml`

Add a small string pre-pass that runs *before* the XML reader and splits the
`.nemo` source into three parts, treating `<script>`/`<style>` as raw-text:

```
fn split_sfc_blocks(content: &str) -> SfcBlocks   // { template_xml, script, style }
```

1. Scan the source for **top-level** `<script …>…</script>` and
   `<style …>…</style>` blocks: match the open tag (allowing attributes up to
   `>`), then capture everything up to the literal matching `</script>` /
   `</style>` **verbatim**, without XML-parsing the interior.
2. Remove those two blocks from the source, leaving the `<template>` element (and
   only that) for `quick-xml`.
3. `parse_sfc` then:
   * runs the existing `parse_element` on `template_xml` only (unchanged — the
     `<template>` single-root requirement, `process_component_element` flatten,
     `collect_slot_specs`, and `<props>` handling at `:817-899` all stay put);
   * takes `script`/`style` from the split result **instead of** reading
     `__cdata__` at `:901-914`.

`SfcDefinition` (`:23`) is unchanged in shape, so `sfc_definition_to_value`
(`:1573`), `parse_layout_config`, the runtime SFC transforms, `nemo build`, and
render are all untouched. The change is contained entirely to `parse_sfc` plus
the new helper.

# Scope / non-goals

* **Backward compatible.** A captured block that still contains a
  `<![CDATA[ … ]]>` wrapper has the leading `<![CDATA[` / trailing `]]>` stripped,
  so existing `examples/sfc/*.nemo` and any authored file load unchanged. CDATA
  becomes *optional*, not forbidden — no forced migration.
* **`.nemo` only.** `app.xml` and `<include>`d documents keep the plain XML path
  (`process_root`); the splitter lives only in `parse_sfc`. `app.xml` `<script>`
  handling elsewhere is not touched.
* **Attributes on the open tag** (e.g. a future `<script lang="rhai">`) are
  tolerated by scanning to the first `>` of the open tag, but no attribute is
  interpreted in v1.
* **Not an HTML parser.** We adopt HTML's raw-text *rule* for two known element
  names, not HTML's attribute quirks, implicit tag closing, or entity table. The
  `<template>` half stays XML (Vue templates are HTML too); `${prop}`
  interpolation continues to live there unchanged.
* **One `<script>` and one `<style>`** per file, matching today's model. A second
  occurrence is last-wins or a warning (pick one; document it).

# Edge cases to test

* `</script>` (or `</style>`) appearing **inside a Rhai/CSS string literal** —
  the v1 splitter closes on the first literal `</script>`, the same known
  limitation HTML has. Acceptable for v1; **document it**, and add a test pinning
  the behavior so a future literal-aware scan is a deliberate change.
* **CRLF** line endings in the captured body (must survive verbatim).
* **Empty or missing** `<script>`/`<style>` block (must yield `None`, matching the
  current `.filter(|s| !s.trim().is_empty())` behavior at `:906`/`:913`).
* **Template text that looks like a block** — a `<template>` whose interior
  contains the literal text `<script>` must **not** be captured; only top-level
  blocks are split. (SFC templates are single-root elements, so top-level here
  means a sibling of `<template>`, not a descendant.)
* **CDATA-wrapped body still parses** (round-trip equality with the un-wrapped
  form).

# Critical files

| File | Role |
|---|---|
| `crates/nemo-config/src/xml_parser.rs` | new `split_sfc_blocks` helper + `SfcBlocks`; `parse_sfc` (`:789`) consumes it — parse only `template_xml`, take `script`/`style` from the split (replacing the `__cdata__` reads at `:901-914`) |

No other files change: `SfcDefinition` (`:23`), `sfc_definition_to_value`
(`:1573`), the runtime SFC pipeline, `nemo build`, and render are all unaffected.

# Verification

* **Unit (nemo-config):** a `.nemo` with an un-escaped `&&`/`<`-bearing Rhai
  `<script>` and a `>`-combinator CSS `<style>`, **no CDATA**, parses to the same
  `SfcDefinition` as the CDATA-wrapped equivalent (round-trip equality); the
  multi-line/multi-run script body is captured whole (the old "first contiguous
  run" limit is gone); the `</script>`-in-string edge case has a test pinning the
  documented v1 behavior; empty/missing blocks yield `None`; template text
  resembling `<script>` is not captured.
* **Regression:** existing `examples/sfc/*.nemo` (CDATA-wrapped) still parse and
  the example still validates `--strict` clean.
* **Optional migration:** drop the `<![CDATA[ … ]]>` wrappers in
  `examples/sfc/*.nemo` to exercise the new path end-to-end (`nemo-run` skill;
  local builds need `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`).

# Relationship to other plans

* **Was** [SFC Phase 6](sfc-components.md) — that section now points here.
* **Independent of** the [build system](build-system.md): build reuses
  `parse_sfc`, so it inherits CDATA-free authoring for free once this lands, with
  no build-side change.
* Orthogonal to the [page router](page-router.md) and every other SFC phase.

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — note that `.nemo` SFC
  `<script>`/`<style>` are parsed as raw-text (CDATA optional), distinct from the
  plain-XML `app.xml` path.
* [Single-file components](../patterns/single-file-components.md) — drop/soften
  the "must be wrapped in `<![CDATA[ … ]]>`" and "one contiguous block" notes in
  "Rules that bite"; update the authoring example to omit CDATA.
* [Roadmap](roadmap.md) and this plan — mark implemented.
