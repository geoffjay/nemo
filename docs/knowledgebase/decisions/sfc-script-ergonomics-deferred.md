---
type: Decision
title: SFC authoring is XML-native; Rhai-style script ergonomics are deliberately deferred
description: Defining and composing `.nemo` single-file components already covers the Vue-style SFC concept (imports, typed props, slots, scoped style, per-SFC scripts). The Rhai-flavored ergonomics — `<script setup>` imports, a `setup(props)` lifecycle hook, `{{ }}` interpolation — are consciously deferred; the goal that "nemo files should be more than an XML-only concept" is recorded for when it is revisited.
tags: [config, sfc, rhai, ergonomics, decision, future]
timestamp: 2026-10-28T00:00:00Z
---

# Decision

Single-file-component authoring stays **XML-native today**: a `.nemo` file is a
`<props>`/`<template>`/`<style>`/`<script>` document imported with the declarative
`<imports>/<import>` block (or `<components dir>` auto-discovery), and components
compose with inline custom tags. The Rhai-flavored ergonomics popularized by Vue
— `import x from "./x.nemo"` inside a `<script setup>` block, a `setup(props)`
lifecycle hook, and `{{ expr }}` moustache interpolation — are **deliberately
deferred**, not rejected.

The longer-term direction is recorded: **nemo files should be more than an
XML-only concept**. A future revision may let Rhai play a bigger role in the
authoring surface (starting with script-side module imports), and we want the
option to return here with that framing. See [issue #94](https://github.com/geoffjay/nemo/issues/94)
for the tracked ergonomics items.

# Current state (verified)

The full Vue-style SFC concept already works, with syntax differences. A sketch
of the desired form:

```xml
<!-- foo-button.nemo (concept sketch) -->
<script setup>
fn setup(props) { bind(props['title']) }
</script>
<template>
  <button id="foo" size="lg">{{ label }}</button>
</template>
```

used as `<script setup>import foo-button from "./foo-button.nemo"</script>`
in the app, then `<foo-button label="foo" />`.

The equivalent authoring form today:

```xml
<!-- foo-button.nemo -->
<props>
  <prop name="label" type="string" default="label" />
</props>
<template name="foo-button">
  <button id="foo" size="lg" label="${label}" />
</template>
```

```xml
<!-- app.nemo -->
<imports>
  <import src="./foo-button.nemo" />
</imports>

<template name="app">
  <stack direction="vertical" spacing="12" align="start">
    <foo-button label="foo" />
    <foo-button label="bar" />
    <foo-button />               <!-- prop defaults fill omitted attributes -->
  </stack>
</template>
```

This exact demo was rendered offscreen (three buttons: `foo`, `bar`, and the
`label` default) — instances need no `id` of their own; template-internal ids
(`foo`) are namespaced per instance automatically.

### Concept → nemo mapping

| Sketch | Nemo today |
|---|---|
| `<script setup>import foo-button from "./foo-button.nemo"` | `<imports><import src="./foo-button.nemo" /></imports>`, or `<components dir="./components" />` auto-discovery (tags from `<template name>` or filename stem; `as=` aliases; `github.com/…` remote packages via `nemo get`) |
| `fn setup(props) { bind(props['title']) }` | `<props><prop name="title" type="string" default="…" required="…"/></props>` — typed, with defaults; instance attributes override; `${title}` interpolation in the template |
| `{{ label }}` | `${label}` (instance-attribute interpolation, a distinct mechanism from load-time `${var.x}`/`${env.x}`) |
| Per-component script | `<script>` in the SFC (parsed as HTML-style raw text, so Rhai `&&`/`>` need no CDATA); template-authored `on-click` routes there automatically as `sfc:<tag>::fn`; one script serves all instances, which pass their scoped `component_id` |
| — | plus: default + named slots (`required`/`multiple`), scoped `<style>` folded at compile time (v1: type and `#id` selectors), `n:for`/`n:if`/`n:key`, `nemo build` dist artifacts and `load = "dist"` |

Living examples: `examples/sfc/` (card with named slots + scoped style,
`labeled-button` with its own script), and the `examples/components` gallery.

# Gaps deliberately deferred

1. **`<script setup>` Rhai-style imports.** Imports are declarative XML; nothing
   parses Rhai import statements. Supporting `import tag from "path"` inside a
   script block means a host-side pre-pass sniffing import lines out of the
   Rhai body and expanding them like `<import>`. Feasible (moderate: parser
   integration, `as=` aliasing, error reporting fidelity), but it forks the
   import mechanism into two spellings. Today there is little perceptible gain —
   the XML block does the same job — though the option is wanted for when nemo
   leans less XML-only.
2. **`setup(props)` lifecycle hook.** Equivalent today: declared props +
   handlers. A per-instance mount hook (`sfc:<tag>::setup`) could be added,
   invoked with the resolved instance props; cost is moderate (lifecycle
   plumbing, a Rhai call per instance on first render). Defer until a concrete
   use case outgrows the declarative model.
3. **`{{ expr }}` moustache alias.** A trivial parse-time alias for `${...}`,
   but documenting two equivalent interpolation syntaxes costs more than the
   alias is worth; `${}` is the established syntax across variables, env refs,
   and SFC props.

# Consequences

* One import spelling (`<imports>`/`<components dir>`) and one interpolation
  syntax (`${...}`) until a revision revisits Rhai-native ergonomics.
* The SFC capability story — imports, typed props, slots, scoped style,
  per-SFC scripts, instance id scoping — is treated as complete for the
  current authoring model; bug fixes and docs remain fair game.
* When the "more than XML" revision happens, start from item 1 (script-side
  imports) and keep the XML block working; the goal at that point is
  ergonomics/familiarity, not capability.
