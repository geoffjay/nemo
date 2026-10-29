---
type: Plan
title: Component-page layout migration
description: Rebuild every component-gallery page on shared <component-page>/<component-example> SFC templates — Title/Description, Examples, Properties, Callbacks sections.
tags: [components, config, sfc, planning, examples]
timestamp: 2026-10-28T00:00:00Z
---

Every component page in `examples/components` should document the component,
not just demonstrate it. Agreed page layout:

```
Component Name                        (heading — from a prop)
Component description                 (paragraph — from a prop)
Examples                              (heading implied)
  Default                             (the default example)
  Alternative/Custom                  (optional example with customization)
  ...                                 (any further grouped examples)
Properties                            (table of attributes — name/type/default/description)
Callbacks                             (every supported callback + a usage example)
```

# Status: template built, Button migrated, review pending

* **`examples/components/templates/component.nemo`** — `component-page` SFC:
  `name` (required) and `description` props; three slots — default (Examples),
  `properties`, `callbacks`.
* **`examples/components/templates/component-example.nemo`** —
  `component-example` SFC: renders a section heading (`label` prop, default
  `"Default"`) plus the gallery's Preview/Code `tabs` pair. Its default slot is
  the live preview; its named `code` slot holds `code`-targeted children:
  `<text slot="code" content='<button … />' />` (single-quoted XML attributes
  avoid entity-escaping the markup; newlines inside attributes render
  verbatim). One example block = one `<component-example>`; untargeted
  children of `<component-page>` are examples in document order, so the
  `Default` example is simply the first block.
* Wired via `<imports>` in `examples/components/app.nemo`; the Properties
  format is settled (declared-mode `<table>`, see the contract below) and the
  Button page renders it live.
* `route /button` is migrated and reviewed; **every other route still uses the
  old `content_page` panel + inline tabs** — migrate them only after the layout
  is agreed on.

# Page-authoring contract

```xml
<route path="/button">
  <component-page name="Button" description="Buttons trigger actions. …">
    <component-example label="Default">           <!-- default: preview slot -->
      <button label="Button" />
      <text slot="code" content='<button label="Button" />' />
    </component-example>
    <component-example label="Callbacks (on-click)" slot="callbacks"> … </component-example>
  </component-page>
</route>
```

Callbacks region: each callback is its own `<component-example slot="callbacks">`
block (label = the callback attribute, e.g. `on-click`; the Code tab documents
the `(component_id, event_data)` handler signature). Properties region
(`slot="properties"`) is a declared-mode `<table>` — Attribute / Type /
Default / Description columns via `<table-header>` `<table-column>` children
(with explicit `width`); one `<table-row>` per attribute; **attribute props
only** — callbacks live in their own section, not repeated in the table.
Attribute name cells use `<table-cell><label text="…" size="sm" /></table-cell>`
to set them off from plain text cells.

# Parser/runtime facts this template leans on (all current behavior)

* **Slot marker is parent-scoped, last-wins.** `process_component_element`
  (`crates/nemo-config/src/xml_parser.rs`) records one `slot` marker on the
  slot element's *parent*. Sibling `<slot>`s under one container overwrite each
  other — every slot needs its own dedicated wrapper element.
* **Slot containers are leaves.** `inject_named_slots`
  (`crates/nemo/src/runtime.rs`) does not recurse into a slot container, so a
  slot must not be nested inside another slot container.
* **Nested SFC usage works** — `rewrite_sfc_tags` rewrites SFC tags inside SFC
  template bodies, and children targeting the *inner* template's named slots
  (`slot="code"` on a child of `component-example`) route correctly. Regression
  test: `test_sfc_nested_template_slot_composition` (`crates/nemo/src/runtime.rs`).
* **Two SFC id semantics** (found during the pages refactor):
  1. `expand_template` collects template-owned ids from the **expanded**
     template, not the raw body — expanding a nested SFC instance mints ids
     (`__anon_2_inner`) that only exist post-expansion; collecting pre-expansion
     left them unscoped, so two pages each embedding a nested `<content-page>`
     rendered identical bodies (the second overwrote the first in the flat
     id-keyed component map). Fixed in `expand_template`
     (`runtime.rs`); regression test
     `test_sfc_nested_sfc_instances_scope_generated_ids`.
  2. An SFC template root's `id` attribute is DISCARDED at parse time
     (`process_component_element` treats `id` as map-key-only) — a `<page
     id="overview">` root does NOT make the page addressable. Instance ids
     come from the usage site: author `<overview id="overview" />` in
     app.nemo (the map key becomes the page node id, matching sidenav
     `target`/`default-page`). Per-file anon counters reset at 1, so sibling
     anonymous instances share keys by design — explicit ids are REQUIRED for
     anything addressed by id (nav targets, `get/set_component_property`).
* **Single-quoted XML attributes** carry `<`, `>`, newlines verbatim — used for
  the Code-slot markup strings (the plain `.nemo` XML path still needs
  `&lt;` escaping only for double-quoted attributes).

# Remaining migration (per component page, mechanical)

1. Replace the route's `<panel template="content_page">` + title/description /
   `<tabs>` chrome with one `<component-page>` plus `<component-example>` blocks.
2. Split existing preview sections into `Default` first, rest as labeled blocks.
3. Add the `Callbacks` region (per-callback examples with a live demo widget
   where one exists — see `on_button_click_demo` in
   `examples/components/scripts/handlers.rhai` for the pattern).
4. Fill the `Properties` region with a declared-mode `<table>` (Attribute /
   Type / Default / Description; attribute props only — no callbacks).
5. After all pages: retire the `content_page` template from `app.nemo`, update
   `examples/components/README.md`, and mark this plan done.

Descriptions may be adapted from `nemo schema --app-config <entry>` output
(per-component `description` metadata in `crates/nemo-registry/src/builtins.rs`)
and gpui-kit pages (`https://gpui-kit.com/component/<name>`).