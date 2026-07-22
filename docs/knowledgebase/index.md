---
okf_version: "0.1"
---

# nemo knowledge base

This is the working knowledge base for the nemo project, conforming to the
[Open Knowledge Format (OKF) v0.1](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md).

It consolidates working knowledge about the project: what nemo is, how it is
structured, decisions and their rationale, recurring patterns, and plans.
It is authored by people and agents and meant to be read by both.

## For agents (policy)

This section is the single source of truth for how agents (Claude Code,
opencode, and others) should use this knowledge base. Tooling injects it into
context automatically (Claude via a `SessionStart` hook, opencode via the
`instructions` config), so it does not depend on `CLAUDE.md`/`AGENTS.md` being
picked up.

**Consult before acting.** Before working on a task, scan the entries below and
read any concept/decision/pattern doc relevant to what you are about to change.
Prefer the recorded decision or pattern over re-deriving one. This index is the
map; read the specific doc on demand rather than guessing.

**Update after acting.** Update the knowledge base when a change would make an
existing entry wrong or leave a new fact unrecorded. In particular:

* Config parsing/resolution/validation → update [Configuration](concepts/configuration.md).
* A new or changed component, the NemoComponent macro, or render dispatch →
  update [Components](concepts/components.md) and the
  [four-file component workflow](patterns/four-file-component-workflow.md).
* Data sources, transforms, the repository, or bindings →
  update [Data flow](concepts/data-flow.md).
* Extensions/plugins (Rhai, native, WASM, plugin API) →
  update [Extensions](concepts/extensions.md).
* A new architectural decision or a change to startup/threading →
  add or update a [decision](decisions/index.md) and [Architecture](concepts/architecture.md).
* A new recurring convention → add a [pattern](patterns/index.md).

Concept docs require YAML frontmatter with a `type` field; `index.md` and
`log.md` are reserved. When you add a doc, add a one-line pointer to the matching
category index below and a line to [`log.md`](log.md). If you deliberately decide
*not* to record a change, that is fine — the policy is judgement, not a mandate
to touch the KB on every edit.

## Concepts

* [Architecture](concepts/architecture.md) - what nemo is, the crate layout, and the config → layout → render flow.
* [Configuration](concepts/configuration.md) - the XML config format and the parse → resolve → validate pipeline.
* [Components](concepts/components.md) - the NemoComponent macro, RenderOnce components, and stateful widgets.
* [Data flow](concepts/data-flow.md) - data sources, transforms, the repository, bindings, and the integration gateway.
* [Extensions](concepts/extensions.md) - the three-tier model: Rhai scripts, native plugins, and WASM plugins.

## Decisions

* [XML is the configuration format (not HCL)](decisions/xml-not-hcl-config.md) - applications are configured in XML; HCL is not implemented.
* [Components implement RenderOnce, not Render](decisions/renderonce-for-components.md) - components are stateless and consumed on render.
* [The gpui git dependency is load-bearing in Cargo.lock](decisions/pin-gpui-git-dep.md) - a rev-less git dep means Cargo.lock pins the working revision; avoid `cargo update`.
* [Three-tier extension model with a unified PluginContext](decisions/three-tier-extensions.md) - Rhai, native cdylib, and WASM plugins share one host API.
* [cargo audit ignores transitive advisories we cannot upgrade](decisions/audit-ignore-transitive-advisories.md) - .cargo/audit.toml ignores advisories pinned via the gpui/wasmtime deps; wasmtime upgrade tracked.
* [nemo screenshot uses gpui's test-support render-to-image path](decisions/screenshot-via-test-support-feature.md) - opt-in `screenshot` feature enables offscreen capture; macOS-first, additive to Cargo.lock.
* [Screenshots target macOS; Windows out of scope](decisions/screenshots-windows-out-of-scope.md) - macOS-first; Linux best-effort/deferred, Windows out of scope.

## Patterns

* [Four-file component creation workflow](patterns/four-file-component-workflow.md) - the four files touched when adding a built-in component.
* [Stateful widget Entity persistence](patterns/stateful-widget-entity-persistence.md) - persist widget state in ComponentStates keyed by ID, with data-change detection.
* [Definite height for uniform_list widgets](patterns/definite-height-for-lists.md) - Table/Tree collapse to 0px without a definite parent height.
* [Collection properties as JSON-string attributes](patterns/json-string-collection-properties.md) - which components take arrays/objects as a JSON-string attribute, and how coerce_value handles them.
* [Parent-rendered child components](patterns/parent-rendered-child-components.md) - how a parent reads and renders its typed child components, vs. generic render_children.
* [Layout sizing and centering](patterns/layout-sizing-and-centering.md) - the minimal style surface: `flex` ignores its value, no percent/align, and how to center a fixed-width panel.
* [Containers](patterns/containers.md) - high-level layout containers (`app-shell`) with slot regions and built-in page switching, in `crates/nemo/src/containers/`.
* [Routing](patterns/routing.md) - the chrome-free `<router>`/`<route>`/`<nav-link>` primitive: host-side router state, deferred navigation, `data.route.*` param projection, and lifecycle hooks.
* [Single-file components](patterns/single-file-components.md) - authoring reusable `.nemo` components (template + scoped script + `${prop}` interpolation + default slot), imported and used as a custom tag.

## Plans

* [Roadmap](plans/roadmap.md) - current capabilities, phase-2 status, remaining roadmap items, and pointers to full planning docs.
* [Declarative children over JSON-string properties](plans/declarative-children-migration.md) - migrate collection components from JSON-string attributes to nested child elements, piloted on accordion.
* [Headless renderer and screenshots](plans/headless-screenshots.md) - `nemo screenshot` implemented on macOS via gpui's offscreen `Window::render_to_image`; Linux capture remains open.
* [Devtools inspector](plans/devtools-inspector.md) - what a nemo-devtools crate would take; the introspection surfaces already exist, in-process panel recommended over an external client.
* [Design tokens and active redesign](plans/design-tokens.md) - centralized spacing/radius/typography/semantic-color tokens (gpui-free `nemo-tokens` crate); full chrome migration with screenshot verification.
* [Design-system export](plans/design-system-export.md) - `cargo xtask design-export` emits tokens + themes + component structure as a pencil.dev-friendly JSON intermediate.
* [Runtime component creation](plans/runtime-component-creation.md) - let handlers/scripts create and remove built-in component instances at runtime, via Rhai and PluginContext.
* [Page router](plans/page-router.md) - a general chrome-free `<router>`/`<route>` primitive (URL-path routes + params, history, lifecycle hooks, nested routers, `navigate()` Rhai API) replacing the fragile visibility-toggle page-switching pattern. **Implemented** — see [routing](patterns/routing.md).
* [Single-file components (`.nemo` SFCs)](plans/sfc-components.md) - a Vue-like `<template>`/`<style>`/`<script>` file, imported and used as a custom tag, implemented by expanding onto the existing template machinery. **Phase 0–4 implemented** — see [single-file components](patterns/single-file-components.md).
* [Build system](plans/build-system.md) - a `nemo build` command with a `nemo.toml` manifest, compiled `.nemo` component artifacts, opt-in `dist/` project builds, and Go-style remote component libraries in `.nemo/packages`; expands and supersedes SFC "Phase 5". **Phase 0 implemented** (`nemo.toml` manifest, project-root discovery, `nemo build` dry-run skeleton, manifest-aware launch); Phases 1–3 pending.

## References

* [OKF spec](references/okf-spec.md) - pointer to the OKF v0.1 specification.
* [Configuration schema export (`nemo schema`)](references/config-schema-export.md) - the nemo-native JSON schema generated from the compiled registries; its shape, determinism, and Phase-1 caveats.
