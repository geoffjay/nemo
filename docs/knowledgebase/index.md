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

## Patterns

* [Four-file component creation workflow](patterns/four-file-component-workflow.md) - the four files touched when adding a built-in component.
* [Stateful widget Entity persistence](patterns/stateful-widget-entity-persistence.md) - persist widget state in ComponentStates keyed by ID, with data-change detection.
* [Definite height for uniform_list widgets](patterns/definite-height-for-lists.md) - Table/Tree collapse to 0px without a definite parent height.
* [Collection properties as JSON-string attributes](patterns/json-string-collection-properties.md) - which components take arrays/objects as a JSON-string attribute, and how coerce_value handles them.
* [Parent-rendered child components](patterns/parent-rendered-child-components.md) - how a parent reads and renders its typed child components, vs. generic render_children.
* [Layout sizing and centering](patterns/layout-sizing-and-centering.md) - the minimal style surface: `flex` ignores its value, no percent/align, and how to center a fixed-width panel.

## Plans

* [Roadmap](plans/roadmap.md) - current capabilities, phase-2 status, remaining roadmap items, and pointers to full planning docs.
* [Declarative children over JSON-string properties](plans/declarative-children-migration.md) - migrate collection components from JSON-string attributes to nested child elements, piloted on accordion.
* [Headless renderer and screenshots](plans/headless-screenshots.md) - spike findings for headless GPUI rendering under Xvfb+lavapipe; capture path needs one more iteration.

## References

* [OKF spec](references/okf-spec.md) - pointer to the OKF v0.1 specification.
