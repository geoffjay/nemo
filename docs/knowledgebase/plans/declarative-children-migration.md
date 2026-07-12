---
type: Plan
title: Declarative children over JSON-string properties
description: Migrate collection components from JSON-string attributes to nested child elements, piloted on accordion.
tags: [components, config, refactor, planning]
timestamp: 2026-07-11T00:00:00Z
---

Move collection components from
[JSON-string collection properties](../patterns/json-string-collection-properties.md)
to nested child elements, so authors write:

```xml
<accordion id="faq" multiple="true" bordered="true">
  <accordion-item title="Question 1">Answer 1</accordion-item>
  <accordion-item title="Question 2">Answer 2</accordion-item>
</accordion>
```

instead of `items='[{"title":…,"content":…}]'`.

# Why

* Editor/tooling support (validation, autocomplete) works on real elements, not
  opaque JSON-in-a-string.
* Item bodies can hold **arbitrary components** (buttons, forms, charts) rather
  than plain strings.
* Removes the silent failure modes of attribute JSON (malformed JSON coerces to
  a string; documented object fields that the code reads as strings).

# Pilot: accordion

The infrastructure already exists — this mirrors the `sidenav-bar` /
`sidenav-bar-item` precedent in
[parent-rendered child components](../patterns/parent-rendered-child-components.md).
No XML-parser change is required for the attribute-based item form.

1. **Registry** — add an `accordion-item` descriptor in
   `crates/nemo-registry/src/builtins.rs` (`title`, `open`, …), beside
   `accordion`.
2. **Dispatch** (`crates/nemo/src/app.rs`) — in the `"accordion"` arm, collect
   child `BuiltComponent`s of type `accordion_item` (kebab→snake); add an
   `"accordion_item" => div().into_any_element()` standalone fallback arm.
3. **Component** (`crates/nemo/src/components/accordion.rs:74`) — iterate the
   passed-in child components instead of `props.get("items")`.
4. **State init** (`crates/nemo/src/components/state.rs:160` +
   `app.rs:763`) — `get_or_create_accordion_state` derives initially-open indices
   from the `items` array today; read each child's `open` attribute instead.
5. **Docs/examples** — update the docstring and `examples/**/*.xml`.

# Item body: decided — nested components

**Decision:** the item body is expressed as **nested child components**, rendered
via `render_children`:

```xml
<accordion-item title="Q1"><label text="Answer 1" /></accordion-item>
```

This needs no parser change and is consistent with how `tabs`, `collapsible`,
`panel`, and `badge` already treat children; it unlocks rich content (buttons,
forms, charts) inside a panel rather than a plain string.

Rejected alternatives: a `content` **attribute** (string-only) and a bare **text
body** (`<accordion-item>Answer 1</accordion-item>`, which would have required
exposing `__cdata__` in `process_component_element` at `xml_parser.rs:660`).

# Back-compat: decided — hard switch

**Decision:** no fallback. The `items='[…]'` property is removed; `accordion`
only accepts `accordion-item` children. This is a **breaking change** — any
config using `items` must be migrated (including `examples/**/*.xml` and the
`accordion` doc example). The pilot's step 3 becomes a replacement of the
`props.get("items")` loop rather than an addition, and step 4 reads the `open`
attribute from child components only.

# Generalization

The same move applies to other collection components once accordion proves it
out: `tabs` (`<tab-item label="…">…</tab-item>` unifies the labels-from-JSON /
panels-from-children split), `select`/`radio` (`<option>`), `dropdown-button`
(`<menu-item>`), and `list` (`<list-item>`).

Optional hardening: schemas do not currently restrict which child types a parent
accepts, so any component can nest under any other. A parent/child-type
constraint (accordion only accepts accordion-item) could be added to the registry
schema as part of this work.

# Status

**Accordion pilot landed** (2026-07-11). `accordion` now takes `<accordion-item>`
children (nested-component bodies, hard switch — `items` removed):

* Registry: `accordion` schema drops `items`; new `accordion_item` type
  (`title`, `open`) — `crates/nemo-registry/src/builtins.rs`.
* Dispatch: the `"accordion"` arm collects `accordion_item` children, renders
  each item's children as its body, and derives initial-open from each item's
  `open` attribute; added an `"accordion_item"` standalone-fallback arm —
  `crates/nemo/src/app.rs`.
* Component: `Accordion` carries `Vec<AccordionItemData>` instead of reading the
  `items` property — `crates/nemo/src/components/accordion.rs`.
* State: `get_or_create_accordion_state` now takes a precomputed
  `HashSet<usize>` of open indices — `crates/nemo/src/components/state.rs`.
* Example migrated: `examples/components/app.xml`.

Verified: builds, component + parser tests pass, and `nemo validate
examples/components/app.xml` passes. Not yet exercised in a live GPUI window.

The generalization to `tabs`/`select`/`radio`/`dropdown-button`/`list` remains
**not started**. When those land (or the pilot is confirmed in the app), record
a decision and update [Components](../concepts/components.md) and
[Configuration](../concepts/configuration.md).
