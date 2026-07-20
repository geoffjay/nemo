---
type: Pattern
title: Single-file components
description: Authoring reusable .nemo components — template + scoped script + interpolation props + default slot — imported and used as a custom tag.
tags: [components, config, templates, sfc, rhai]
timestamp: 2026-07-20T00:00:00Z
---

A **single-file component (SFC)** packages a reusable piece of UI into one
`.nemo` file — markup, optional styling, optional behavior — imported and used as
a custom tag. SFCs compile onto the existing `<template>` machinery (they are a
namespaced, file-scoped superset of it), so they add near-zero new surface: an
expanded SFC is ordinary built-in components. For the parse/compile pipeline see
[Configuration](../concepts/configuration.md#single-file-components-nemo-sfcs).

# Authoring a `.nemo` file

A `.nemo` file is **not** wrapped in `<nemo>`. Its top-level children are:

* `<template>` — **required, exactly one root element**. Its body is flattened
  the same way layout components are. Use `${prop}` placeholders for interpolation
  props and a `<slot />` for consumer content.
* `<style>` — optional. **Captured but not yet applied** (scoped-style folding is
  a later phase). XML has no raw-text elements, so a style/script body containing
  `<` or `&` must be wrapped in `<![CDATA[ … ]]>`.
* `<script>` — optional Rhai. Loaded under the id `sfc:<tag>`. Bodies must be one
  contiguous block (the parser keeps only the first text/CDATA run).

```xml
<!-- components/labeled-button.nemo -->
<template name="labeled-button">
  <button label="${label}" variant="primary" on-click="handleClick" />
</template>
<script><![CDATA[
fn handleClick(component_id, event_data) {
    set_component_property(component_id, "label", "Clicked!");
}
]]></script>
```

# Using it

```xml
<imports>
  <import src="./components/labeled-button.nemo" />   <!-- tag from <template name> -->
  <import src="./components/card.nemo" as="my-card" />  <!-- as= overrides the tag -->
</imports>
<layout type="stack">
  <labeled-button label="Save" />        <!-- ${label} → "Save" -->
  <my-card>
    <label text="Slotted content" />     <!-- injected into the card's <slot/> -->
  </my-card>
</layout>
```

# Rules that bite

* **Tag naming.** The tag is resolved `as=` > `<template name>` > filename stem,
  then kebab→snake normalized (like every element). Write `<labeled-button>` in
  markup; it matches the SFC internally as `labeled_button`.
* **Two `${}` systems.** SFC props use bare `${label}`, filled from instance
  attributes via the **runtime** vars/interpolation path (string-only). This is
  distinct from load-time `${var.x}`/`${env.x}` resolution — the resolver skips
  the SFC subtree, so bare placeholders survive to expand time. Data still flows
  through `bind_*`/`<binding>`.
* **Slots inject components, not text.** Consumer children are injected into the
  template's `<slot/>`; raw text between tags is dropped (as everywhere in nemo).
  Provide text via a child component (`<label text="…"/>`) or an interpolation
  prop. Named/multiple slots are a later phase — v1 is a single default slot.
* **Handler scoping.** A **template-authored** bare `on-click="fn"` is rewritten
  to `sfc:<tag>::fn` and routes to the SFC's own `<script>`. An **instance**
  handler (`<labeled-button on-click="globalFn"/>`) stays bare and routes to the
  global `handlers` script, overriding the template's via deep-merge. One SFC
  script serves every instance; the handler gets the per-instance scoped
  `component_id`, so it mutates just the instance that fired.
* **Id scoping is automatic.** Template-owned child ids are prefixed with the
  instance id (`body` → `<instance>_body`), so multiple instances never collide.
  Instance-injected (slot) children keep their own ids.

# Status

Phase 0 (import, tag rewrite, default slot, interpolation props) and Phase 1
(scoped `<script>`) are implemented. Scoped `<style>` folding, named/multiple
slots, typed props/auto-discovery, and a build/cache format are planned — see
[the SFC plan](../plans/sfc-components.md). Worked example: `examples/sfc/`.
