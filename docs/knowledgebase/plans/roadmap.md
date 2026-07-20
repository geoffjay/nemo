---
type: Plan
title: Roadmap
description: Current capabilities, phase-2 status, and remaining roadmap items with planning detail.
tags: [roadmap, planning]
timestamp: 2026-07-12T00:00:00Z
---

# Where Nemo is

Nemo currently supports:

* Declarative XML UI with 50+ built-in components and a component registry.
  See [Components](../concepts/components.md).
* High-level layout **containers** (`crates/nemo/src/containers/`) that package
  common app layouts — first is `app-shell` (sidenav + switchable content pages +
  footer, with built-in page switching). See [Containers](../patterns/containers.md).
* A chrome-free page **router** (`<router>`/`<route>`/`<nav-link>`) with
  URL-style path routes + params, history, `on-enter`/`on-leave` hooks, nested
  routers, and a Rhai `navigate()`/`back()`/`forward()` API. See
  [Routing](../patterns/routing.md).
* Reusable **single-file components** (`.nemo` SFCs): one file with
  `<template>`/`<style>`/`<script>`, imported via `<import>` and used as a custom
  tag, with `${prop}` interpolation, a default slot, and a scoped Rhai script.
  Compiled onto the template machinery (Phase 0+1). See
  [Single-file components](../patterns/single-file-components.md).
* Live data binding from HTTP, WebSocket, MQTT, Redis, NATS, timer, and file
  sources through a transform pipeline and central repository. See
  [Data flow](../concepts/data-flow.md).
* Three extension tiers — Rhai scripts, native `cdylib` plugins, and WASM
  Component Model plugins — over a unified `PluginContext`. See
  [Extensions](../concepts/extensions.md).
* Built-in themes, workspace shell (header/footer/main view), and a settings UI.
* CLI subcommands: `nemo new` (scaffold), `nemo dev` (hot-reload),
  `nemo validate` (config validation with `--strict` lints), `nemo schema`
  (export the config schema as nemo-native JSON, generated from the compiled
  registries so it's always current). See
  [config schema export](../references/config-schema-export.md).
* Cross-platform packaging (`.tar.gz`/`.zip`/`.app`/`.dmg`/`.deb`/`.rpm` +
  checksums) and distribution (`install.sh`, Homebrew tap auto-push).

Worked examples live under `examples/` (basic, components, data-binding,
data-streaming, calculator, pid-control, complete, task-list, dev-dashboard,
app-shell, router, sfc).
Example configs are validated in CI via `nemo validate --strict`
(`.github/workflows/ci.yml`, `validate-examples` job).

# Phase 2 — build, installation, and developer experience

All workstreams done or deferred. No separate planning doc remains.

| Item | Status | Notes |
|------|--------|-------|
| CLI subcommand architecture | ✅ Done | `new`/`dev`/`validate` subcommands; bare `nemo --app-config` preserved. |
| `nemo new` scaffold | ✅ Done | 4 templates (`basic`, `calculator`, `data-binding`, `complete`) embedded via `include_str!`; scaffolds validate. |
| Hot-reload dev mode | ✅ Done | `nemo dev` + `--watch`; `notify` watcher drives `Workspace::reload_config` (~0.7 s). Confirmed: `create_runtime` re-reads `<include>`s and reloads `.rhai` from disk on rebuild. |
| `nemo validate` subcommand | ✅ Done | `--strict` lints; `unknown-attribute` runs on all schemas with universal-style allowlist; `missing-required` gated on strict schemas. `--format human\|json`; `--validate-only` forwards. |
| Cross-platform packaging | ✅ Validated | v0.7.0-rc.1 produced 14 assets across 5 targets. AppImage/`.msi` deferred; signing declined (`xattr` workaround). |
| Distribution | ✅ Done | `install.sh` verified end-to-end; Homebrew tap auto-push working. AUR/Scoop/Winget deferred. |
| Headless renderer / screenshots | ✅ Done (macOS) | `nemo screenshot` via gpui's offscreen `Window::render_to_image` behind the opt-in `screenshot` feature. Linux capture remains open. See [headless screenshots](headless-screenshots.md) + [decision](../decisions/screenshot-via-test-support-feature.md). |

# Remaining roadmap items

Not yet started. Priorities (from the original roadmap's matrix, with
completed items pruned):

| Improvement | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| Fix Styling page XML snippets | Gallery accuracy | Low | P1 |
| Schema-driven gallery generation | Gallery correctness | Medium | P1 |
| `nemo storybook` subcommand | Discoverability | Low | P1 |
| XML LSP server (`nemo-lsp`) | Config authoring | High | P1 |
| `nemo generate` CLI | AI-assisted start | Medium | P2 |
| Interactive property playground | Component exploration | High | P2 |
| VS Code extension | Editor integration | Medium | P2 |
| Built-in `<chat>` component | AI in apps | High | P3 |
| Authoring assistant application | Full guided flow | High | P3 |

## Configuration schema (feedstock for LSP / gallery / LLM)

**Phase 1 landed** (`nemo schema`): a nemo-native JSON export generated from the
compiled registries + the canonical out-of-registry surface
(`nemo-registry::schema_surface`: universal style attributes, `on-*`/`bind-*`
families, structural elements). See
[config schema export](../references/config-schema-export.md). This is the shared
feedstock the items below assume ("auto-generated from `nemo-registry`").

**Phase 2 remains**: enrich the *content* so the schema answers "what values are
allowed". Derive per-property schema from `#[derive(NemoComponent)]` (anti-drift,
generated from the struct fields); add `#[property(one_of = [...])]` enum
annotations (`align`, `variant`, `size`, `direction`, …) and numeric ranges;
populate `events`/`bindableProperties`/`slots`; and add a component containment
table (currently implicit in the `app.rs` render match arms). Optional later: an
XSD projection for third-party XML-editor completion.

## Component storybook

`examples/components/app.xml` is a sidebar-navigated component gallery covering
all 50+ components. It has three structural weaknesses: it's manually
maintained (schema additions drift silently), previews are read-only, and
there's no search or deep-linking. The Styling page still shows HCL syntax
snippets instead of XML.

### Generated gallery (schema-driven)

Derive the gallery XML from the component registry schemas rather than
maintaining it by hand. The `nemo-registry` crate holds `ComponentSchema`
objects with name, property types, defaults, and descriptions. A build-time
code generator (`xtask` or `cargo run --bin generate-gallery`) should emit
`examples/components/app.xml` from those schemas — each section including all
property rows with types/defaults from `PropertySchema`, auto-generated
examples covering mandatory properties, and the current `<binding>` wiring for
live updates where applicable. Gallery stays correct by construction; new
components appear automatically.

### Interactive property playground

Add a `playground` panel to each component page. A `<property-inspector>`
component type, backed by the component's registered `ComponentSchema`, would
render a form (inputs for strings, sliders for numbers, checkboxes for
booleans, dropdowns for enums) and apply changes to the live preview via
`set_component_property()` (which already exists in the Rhai context).

Implementation path:
1. Add a `<property-inspector>` component type to the layout engine.
2. Wire changes through `set_component_property()` in the Rhai context.
3. Embed one `<property-inspector>` instance per gallery page.

### `nemo storybook` subcommand

```
nemo storybook
nemo storybook --component button
nemo storybook --search chart
```

Makes the gallery a first-class developer tool rather than an example that
must be found in the repository.

### Search and deep links

- **In-panel search** — a `<input>` bound to a filter that hides non-matching
  pages (Rhai on-change handler filtering `visible` property).
- **Deep links** — via `gpui-router` (already a workspace dependency) so
  `nemo storybook --component table` opens directly to the Table page.

### Fix Styling page code snippets

The Styling page code editors show HCL block syntax instead of XML attribute
syntax. Update so the gallery is a reliable reference.

## Configuration authoring

Nemo's XML config is expressive (variables, templates, slots, data bindings,
expressions, multi-file includes) but authoring is a raw-text experience with
no editor assistance beyond basic XML syntax checking. Key friction: no
autocomplete, no hover docs, no inline validation, no live preview, expression
syntax (`${}`) and binding paths opaque to editors.

### XML Language Server (`nemo-lsp`)

A `nemo-lsp` binary (new crate) implementing LSP for `.xml` files with a
`<nemo>` root element. Speaks JSON-RPC over stdio; plugs into VS Code, Neovim,
Zed, and any LSP-capable editor. Reuses `nemo-config`'s parser and
`nemo-registry`'s schema access directly (both are library crates); ~500 lines
of plumbing around `tower-lsp`.

Capabilities (in priority order):

| Capability | Implementation |
|------------|---------------|
| Completion — component names | From `ComponentRegistry` component list |
| Completion — attribute names | From `ComponentSchema.properties` for the element under cursor |
| Completion — enum values | From `PropertySchema` where type is an enum (variant, size, direction, etc.) |
| Hover documentation | `PropertySchema.description` rendered as Markdown |
| Diagnostics (validation) | Run existing `nemo validate` logic, emit LSP diagnostics with range |
| Completion — data source refs | From `<data>` block names for `bind-*` and `<binding source=...>` |
| Completion — template refs | From `<templates>` block names for `template="..."` |
| Completion — handler names | From loaded `.rhai` files for `on-click=` etc. |
| Go-to-definition | Jump to `<template>` definition, `<source>` definition |

### VS Code extension

A thin `nemo-vscode` extension (~200 lines TypeScript):
1. Registers `nemo-lsp` as the language server for files with a `<nemo>` root.
2. Syntax highlighting via a TextMate grammar (XML base + Nemo-specific token
   scopes for `${...}` expressions and data binding paths).
3. A "Preview in Nemo" command that saves the current file and runs
   `nemo --app-config <file>` in a terminal pane.

### Live preview

`nemo dev` already hot-reloads on save. A side-by-side preview in the IDE
(WebView panel with screenshot refresh, or a floating auto-reloading window)
would close the loop. Depends on [headless screenshots](headless-screenshots.md)
for the automated path.

### Expression language improvements

The resolver supports `${var.name}`, `${env.VAR}`, conditional ternary, and
six built-in functions (`upper`, `lower`, `trim`, `length`, `coalesce`, `env`).
Add:
- `${config.section.key}` — read from an external TOML/JSON config file
  referenced in `<app>`.
- `${data.source_name.field}` — inline data binding without a separate
  `<binding>` child element (shorthand for simple cases).
- Edit-distance "Did you mean?" suggestions for undefined variables.

## LLM configuration generation

The cold-start problem is significant: a new author must understand the XML
structure, component names, property syntax, data source types, Rhai handler
conventions, and template mechanics before producing a working app. Nemo
already has the infrastructure to host an LLM-assisted authoring flow
(`<http>` data source, `<input>`/`<textarea>`, Rhai scripting, `<code-editor>`).

### `nemo generate` CLI

```
nemo generate "A dashboard with a sidebar, a table of users from a REST API, and a refresh button"
nemo generate --from screenshot.png  # describe UI from an image
```

Implementation:
1. Build a system prompt including the full component schema (auto-generated
   from `nemo-registry`) and property documentation.
2. Include 3-5 few-shot examples (from `examples/`).
3. Call an OpenAI-compatible endpoint (ship with `--api-url`/`--api-key` flags
   or `NEMO_LLM_URL`/`NEMO_LLM_KEY` env vars).
4. Stream the XML response; detect `<nemo>...</nemo>` boundaries, write to
   `app.xml`.
5. Run `nemo validate app.xml`; if it fails, re-send validation errors to the
   LLM for a correction pass.

The system prompt preamble should be auto-generated at build time from the
schema so it stays current as components evolve.

### Built-in `<chat>` component

```xml
<chat id="assistant"
      api-url="${env.ANTHROPIC_API_URL}"
      api-key="${env.ANTHROPIC_API_KEY}"
      model="claude-sonnet-4-6"
      system-prompt="You are a Nemo configuration assistant..."
      placeholder="Describe the UI you want to build..." />
```

A persistent chat interface backed by an HTTP data source. Maintains
conversation history in component state and streams responses. Makes the LLM
accessible as a first-class layout component — an AI panel embeddable in any
Nemo application.

### Authoring assistant application

A purpose-built `examples/assistant/app.xml` combining:
- A chat panel backed by `<chat>` with a Nemo-specific system prompt.
- A `<code-editor>` pane displaying the current generated config.
- A "Launch Preview" button that writes the code-editor content to a temp file
  and runs `nemo --app-config <tmp>`.
- A template browser (simplified storybook) as a reference panel.

System prompt strategy: (1) the Nemo XML reference auto-generated from schemas
(~4,000 tokens), (2) the current file content (for targeted edits, not full
rewrites), (3) the last validation error if any, (4) a constraint to always
emit valid Nemo XML inside a `<nemo>` root element.

### Configuration diff and apply

Rather than replacing the entire config on each generation, implement a
diff/apply flow: the LLM proposes changes as a diff or targeted additions, a
Rhai handler merges them into the live config tree, and hot-reload picks up
the written file change. Makes the authoring loop feel like pair programming
rather than generate-and-replace.

# Risk notes

- **nemo-lsp** reuses `nemo-config` and `nemo-registry` as libraries — no
  architectural changes needed. Main risk is LSP protocol compliance,
  mitigated by `tower-lsp` handling the JSON-RPC layer.
- **`nemo generate`** requires an API key at runtime. Ship with flags/env vars
  so it works with any OpenAI-compatible endpoint, not just Anthropic.
- The **plugin factory gap** (plugins can declare schemas but not provide
  runtime instances) should be addressed before the plugin system is promoted
  as stable. Does not block any roadmap item above.