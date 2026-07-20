---
type: Plan
title: Single-file components (`.nemo` SFCs)
description: A Vue-like single-file component system — one `.nemo` file with `<template>`/`<style>`/`<script>`, imported and used as a custom tag, implemented by expanding onto the existing template machinery.
tags: [components, config, templates, planning, rhai, styling]
timestamp: 2026-07-19T00:00:00Z
---

# Single-file components (`.nemo` SFCs)

Let authors write one reusable component in a `.nemo` file and use it as a custom
tag. Example — `components/button-group.nemo`:

```
<template name="foo">
  <button id="btn" variant="primary"><slot></slot></button>
</template>
<style>
  button { height: 32px; }
</style>
<script>
  fn handleClick(component_id, event_data) { /* rhai */ }
</script>
```

used in `app.xml`:

```xml
<nemo>
  <imports>
    <import src="./components/button-group.nemo" />   <!-- tag = "foo" (or as="…") -->
  </imports>
  <layout type="stack">
    <foo>Click me</foo>          <!-- default slot -->
  </layout>
</nemo>
```

# Why (the drawback with today's reuse)

Reuse today is served by three loosely-coupled mechanisms that don't add up to a
component unit (see [Configuration](../concepts/configuration.md)):

* **`<templates><template name>` + `template="name"`** — a fragment referenced by
  a magic string, with `<vars>`/`${}` string interpolation and a **single**
  `<slot>`. Expanded at runtime on the `Value` tree (`crates/nemo/src/runtime.rs:929-1553`).
* **`<include src>`** — a *top-level-only* merge (`xml_parser.rs:611`,
  `merge_into`) that requires each included file to be a full `<nemo>` document and
  resolves collisions silently last-wins. No scoping, no aliasing, no per-component
  encapsulation.
* **Styling and behavior are global** — styling is inline attributes consumed by
  `apply_layout_styles` (`crates/nemo/src/app.rs:514`); Rhai handlers live in one
  flat namespace (`call_handler`, `runtime.rs:494`).

So markup, data, children, styling, and behavior are separate globally-scoped
concerns, and loading a reusable piece means whole-document merges with no
encapsulation. SFCs bundle all five into one authorable, importable, scoped file.

# Key decision: expansion, not new render dispatch

An SFC is a **namespaced, file-scoped superset of the existing `<template>`
mechanism**. Each `.nemo` file compiles to:

1. one `TemplateMap` entry (its `<template>` markup, with `<style>` folded in),
2. one Rhai script loaded under a per-SFC id (`sfc::<name>`),
3. a tag→template rewrite so `<foo …>children</foo>` becomes a template instance
   *before* expansion.

Then the existing `expand_children` → `expand_template` pipeline
(`runtime.rs:1471`/`:1282`) handles props (deep-merge + interpolation), slot
injection, and per-instance ID scoping. By the time an SFC reaches
`LayoutBuilder` and `render_component`, it is ordinary built-in components — every
downstream stage is unchanged.

**Why expansion over a first-class render callback** (the Layer-2 approach in
[runtime component creation](runtime-component-creation.md)):

| Concern | Expansion (chosen) | Render callback in `app.rs` `_` arm |
|---|---|---|
| Downstream changes | none — output is built-ins | `render_component`, flat id-map, Value→built-in recursion all change |
| Props | reuse `deep_merge_values` (`:1015`) + `interpolate_variables` (`:1168`) | new prop plumbing |
| Slots | reuse `find_and_inject_slot` (`:1237`) | reimplement against `BuiltComponent.children` (ids) |
| ID uniqueness | reuse `scope_template_children` (`:1388`) | invent scoping in the flat map |
| Styling | fold into node props at expand time | needs a render-time cascade |
| Risk | contained to the runtime Value stage | touches the render hot path |

Layer 2 stays the right tool for *plugin-authored native* component types; SFCs
are static markup compositions that expansion covers with near-zero new surface.

Facts that make this cheap:

* `parse_element` already captures `<style>`/`<script>` raw bodies into
  `__cdata__` (`xml_parser.rs:958`); only `process_component_element` strips it
  when flattening the layout tree (`:800`), and `process_script` already reads it
  (`:415`).
* `BuiltComponent.properties` is a dynamic `HashMap<String, Value>` — expanded
  SFCs are just map entries (see [parent-rendered child components](../patterns/parent-rendered-child-components.md)).
* ID scoping already prevents the flat-map collision bug class the document-global
  `anon_counter` guards against (`xml_parser.rs:16`).

# Phase 0 — MVP: import, tag rewrite, default slot, interpolation props

Deliver `<foo>content</foo>` from one `.nemo` file with template + default slot +
`${prop}` interpolation. No scoped style/script yet (authors can use inline attrs
and the global `handlers` script).

**`crates/nemo-config/src/xml_parser.rs`**

* Add `XmlParser::parse_sfc(content) -> Result<SfcDefinition, _>`: run existing
  `parse_element` (`:875`), then walk top-level children — `template` → run
  `process_component_element` (`:789`) on its single root child; `style`/`script`
  → read `__cdata__` raw. Require exactly one element child in `<template>`
  (single-root, matches `find_and_inject_slot`).
* New `SfcDefinition { name, template: Value, style: Option<String>, script:
  Option<String>, source_path }` (re-export from `lib.rs`).
* Add an `"imports"`/`"import"` arm to `process_root`'s match (`:108-156`):
  resolve `src` relative to `base_dir` (as `process_include` does, `:611`), call
  `parse_sfc`, store under a new top-level key `"sfc"` = `{ tag → SfcDefinition }`.
  `as=` overrides the tag; default tag is `<template name>` or the filename stem.
* Document the v1 body limit: `parse_element` stores only the **first** non-empty
  text/CDATA run (`:930-932`), so `<script>`/`<style>` bodies must be one
  contiguous block (multi-run accumulation is later hardening).

**`crates/nemo/src/runtime.rs`**

* In `parse_layout_config` (`:1516`): read `"sfc"`, merge each SFC template into
  the `TemplateMap` at the plugin-template merge site (`:1520-1525`).
* Add `fn rewrite_sfc_tags(layout: &Value, sfc_names) -> Value` (pure), called
  just before `expand_children` (`:1531`): any node whose `type` is a registered
  SFC tag becomes `{ template:"foo", <attrs→vars+overrides>, component:{children} }`.
* Fold plain instance attributes into the `vars` map so `<foo label="Hi">` feeds
  `${label}` via `extract_vars` (`:1133`)/`interpolate_variables` (`:1168`); scalar
  attrs matching template attrs overlay via `deep_merge_values` (`:1015`).

**Reused unchanged:** `expand_template`, `find_and_inject_slot` (default slot works
today), `deep_merge_values`, `scope_template_children`,
`parse_component_from_value` (`:1597`), all of `app.rs`.

# Phase 1 — Scoped `<script>`

* `load_scripts_from_config` (`runtime.rs:291`): load each SFC's `<script>` under
  `sfc::<name>` via `ExtensionManager::load_script` (per-id AST + persistent
  `Scope`, `rhai_engine.rs:252`); `reload_script` (`:276`) covers `nemo dev`.
* During SFC compilation, rewrite bare `on_*` handler values (no `::`) to
  `sfc::<name>::<fn>`. `call_handler` (`:494`) already routes `id::fn` to the named
  script and bare names to the global `handlers` script — **no change to
  `call_handler`**; already-qualified refs are left alone.
* One SFC script serves all instances, distinguished by the instance-scoped
  `component_id` it receives; props/state via host fns `get/set_component_property`,
  `get/set_data` (`rhai_engine.rs:389`). Namespace all SFC scripts under `sfc::`
  so none shadow the reserved `handlers` id. See [Extensions](../concepts/extensions.md)
  for the pure-function scope rules SFC scripts must follow.

# Phase 2 — Named / multiple slots

* `process_component_element` (`:789`, slot detection `:828`): record a slot
  **name** (`<slot name="header"/>`), not just a bool.
* Generalize `find_and_inject_slot` (`:1237`) to name-keyed injection; consumer
  children targeting a slot via `slot="header"` group by name, unnamed children go
  to the default slot.
* Wire the already-declared-but-disconnected `SlotSpec { name, accepts, multiple,
  required }` on `ComponentDescriptor` (`crates/nemo-registry/src/descriptor.rs`)
  to validate slot names / `required` / `multiple` at expand time.

# Phase 3 — Scoped `<style>`

Compile-time **selector→prop folding** (there is no runtime cascade — see
[layout sizing and centering](../patterns/layout-sizing-and-centering.md)).
Compile the `<style>` block into inline props on matching template nodes at
SFC-load time, before the template enters the `TemplateMap`; afterward styles are
indistinguishable from author-written inline attributes.

* New CSS-subset parser. **v1 selectors:** type (`button {}` → `type=="button"`)
  and id (`#btn {}`). Defer `.class` until a `class` attribute exists. No
  combinators/pseudo-classes/media queries in v1.
* Constrain accepted declarations to
  `nemo_registry::schema_surface::universal_style_attributes()` so `<style>` can
  only set attributes `apply_layout_styles` (`app.rs:514`) consumes. Normalize css
  names (`border-radius`→`rounded`, `background-color`→`background`); sizes
  `32px`→int; colors via `resolve_color`/`resolve_theme_color`
  (`components/mod.rs:84`/`:120`) so tokens/semantic roles stay usable. Unknown
  props warn + drop.
* **Scoping is inherent** — folding is per-SFC-subtree, so a `button {}` rule
  can't reach other components; no scope-hash attribute needed.
* **Precedence (low→high):** `<style>` rule → template inline attr → instance
  attr. Fold `<style>` only where the attr is absent, then let `deep_merge_values`
  overlay instance attrs at expand time.

# Phase 4 — Typed props & auto-discovery

* `<props><prop name type default/></props>` inside the SFC; coerce/validate in
  `parse_sfc`; reconcile with the scalar `#[derive(NemoComponent)]` model
  (`crates/nemo-macros/src/lib.rs`) and register a `ComponentDescriptor` per SFC so
  `nemo validate`/`nemo schema` see SFC tags. Until then props are stringly-typed.
* `<components dir="./components"/>` auto-discovery (opt-in) globbing `*.nemo`.

# Phase 5 — Compiler / binary format (optional, off the critical path)

* `nemo build`: run parse → SFC registration → style-fold → `rewrite_sfc_tags` →
  `expand_children` once, serialize the resolved tree, load it at startup to skip
  re-parse/expand. `nemo dev` stays interpret-on-change.
* **Serialize boundary:** emit the post-`expand_children` `Value` or the
  `LayoutConfig`/`LayoutNode` AST (`crates/nemo-layout/src/node.rs`) — both already
  `serde`-derived (JSON-capable). Deriving `Serialize` on `BuiltComponent`
  (`manager.rs:29`, currently `Debug, Clone` only) would let the cache skip
  `LayoutBuilder::build` too, but couples the cache to render-layer types — defer.
* **Format:** JSON first (proven by `nemo schema` and `cargo xtask design-export`;
  inspectable). Add postcard/bincode only if startup profiling shows JSON parse
  dominates.
* **Risks:** cache invalidation must hash every input (app.xml, all imported
  `.nemo`, all scripts); version skew; bypasses error-surfacing paths. Keep
  strictly optional.

# Cross-cutting risks & decisions

* **Flat id-keyed model** — central correctness risk, fully mitigated by expansion
  (inherits `scope_template_children`). SFC-internal anonymous children get
  `__anon_N` ids (`xml_parser.rs:16`) — ensure they're treated as template-owned so
  `scope_owned_descendants` (`:1439`) prefixes them per instance. Ship a
  multi-instance collision test in Phase 0.
* **Two `${}` systems** — keep SFC props on the runtime `vars`/interpolation path
  (string-only) and data on the `binding`/`bind_*` path
  (`parse_component_from_value:1643`). Document which is which.
* **Backward-compat** — `<templates>`/`template=` and `<include>` are untouched and
  additive; no forced migration. (Optional future: re-express `<templates>` as
  implicit SFCs.)

# Relationship to the page-router plan

This plan and [page router](page-router.md) are **independent** — neither depends
on the other, and either can land first without redesigning the other. They touch
the codebase in disjoint places:

* The only shared file is `runtime.rs`, and the edits are in different functions —
  the router adds `RouterRegistry`/nav-queue fields, `apply_pending_navigations`,
  and a `RuntimeContext` nav handle; SFCs edit `parse_layout_config` (`:1516`) and
  `load_scripts_from_config` (`:291`). That is a mechanical merge, not a design
  conflict.
* SFCs deliberately **do not** touch `app.rs`/`render_component`; the router only
  *adds* render arms and never changes the dispatch mechanism, `call_handler`
  routing, or the template-expansion pipeline SFCs build on. So expanded SFCs
  (built-ins by render time) are unaffected by the router, and the router's new
  arms are unaffected by SFCs.

The one genuine interaction is **composition, not dependency**: nesting a
`<router>` inside an SFC `<template>`. SFC ID-scoping (`scope_template_children`)
renames the router's `id` per instance (e.g. `main` → `foo_main`), so any
`<nav-link router="main">` / `navigate("main", …)` *inside that SFC* must be
rewritten to the scoped id — the same rewrite the SFC pass already does for handler
refs (Phase 1) and should extend to `router=`/nav targets. This is a small,
containable follow-up owned by **whichever plan lands second**, plus one test. The
reverse case — an SFC used inside a `<route>` body — works for free, since the SFC
is expanded to built-ins before the router renders the route.

**Suggested ordering** (they're independent, so this is value/risk, not a
requirement): router first (self-contained, proven `app-shell` pattern, bounded
scope) then SFCs, folding the "scope nested router ids" item into the SFC work. If
SFCs are the higher-priority capability, doing them first costs the router nothing.

# Critical files

| File | Role |
|---|---|
| `crates/nemo-config/src/xml_parser.rs` | `parse_sfc`; `imports`/`import` arm in `process_root` (`:108`); reuse `process_component_element` (`:789`), `__cdata__` capture (`:958`) |
| `crates/nemo/src/runtime.rs` | SFC→`TemplateMap` merge + `rewrite_sfc_tags` in `parse_layout_config` (`:1516`); per-SFC `load_script` in `load_scripts_from_config` (`:291`); reuse expand/slot/scope fns (`:1015-1507`) |
| `crates/nemo-registry/src/schema_surface.rs` | `universal_style_attributes()` — `<style>` allowlist (Phase 3) |
| `crates/nemo/src/components/mod.rs` | `resolve_color` (`:84`)/`resolve_theme_color` (`:120`) — style value resolution (Phase 3) |
| `crates/nemo-registry/src/descriptor.rs` | `ComponentDescriptor`/`SlotSpec` — named-slot validation (Phase 2), per-SFC descriptor (Phase 4) |
| `crates/nemo-layout/src/{node.rs,manager.rs}` | serde boundary for the compiler (Phase 5) |

# Verification

* **Unit (nemo-config):** `parse_sfc` yields a template-shaped `Value` + captured
  style/script bodies; `imports` resolves relative paths and aliases;
  malformed/missing `.nemo` errors clearly.
* **Unit (runtime):** `rewrite_sfc_tags` turns `<foo>` into a template instance;
  attr→vars folding; **multi-instance id-collision regression**; default-slot
  injection; (P2) named-slot routing; (P3) style-fold precedence; (P1) handler-ref
  rewrite routes to `sfc::<name>::fn`.
* **End-to-end:** an `examples/sfc/` app validated by `nemo validate --strict` and
  run in a live GPUI window (`nemo-run` skill; local builds need
  `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`). Confirm render,
  default-slot content, scoped styles, and a scoped handler firing. Mirror
  `test_task_list_handlers_end_to_end` (`rhai_engine.rs`) for a scoped handler
  mutating a property.
* **Compiler (P5):** round-trip through `nemo build`, assert identical render; a
  cache-invalidation test on an edited `.nemo`.

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — document SFC parsing, `<imports>`,
  and the tag-rewrite step in the pipeline.
* [Components](../concepts/components.md) — an "SFCs" section alongside the
  built-in component model.
* A new [pattern](../patterns/index.md) for authoring `.nemo` files.
* [Roadmap](roadmap.md) — move this item as phases land.
