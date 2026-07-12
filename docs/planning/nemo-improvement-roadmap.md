# Nemo Improvement Roadmap

> **Analysis Date:** 2026-05-15  
> **Version:** 0.6.0  
> **Status:** Beta

## Executive Summary

Nemo is a configuration-driven desktop application framework built on GPUI. It delivers on its core promise — a working XML-to-native-UI pipeline with 50+ components, 6 data source types, live bindings, RHAI scripting, and a native plugin system. The codebase is clean, idiomatic Rust, and the architecture is well-considered.

Four areas have clear, high-leverage improvement opportunities:

1. **Component Storybook** — The existing component gallery is a manually-maintained XML file. Making it generated, interactive, and launchable as a first-class developer tool would collapse the feedback loop for component exploration.
2. **Build and Installation** — Packaging exists only for macOS and only via a bash script. A `nemo new` scaffold command, cross-platform packaging, and hot-reload dev mode are missing.
3. **Configuration Authoring** — An XSD schema exists but there is no editor integration (LSP, VS Code extension, language server) and no live-preview development server.
4. **LLM Configuration Generation** — The data source infrastructure is rich enough to power an AI-assisted authoring flow; adding it as a first-class capability would eliminate the cold-start problem for new application authors.

---

## 1. Component Storybook

### Current State

`examples/components/app.xml` is a sidebar-navigated component gallery that runs as a Nemo application. It covers all 50+ components with live previews and shows HCL syntax snippets via embedded `<code-editor>` components. It is the right idea, but it has three structural weaknesses:

- **Manually maintained.** Every new component, property, or variant requires a manual XML edit. Schema additions drift from the gallery silently.
- **Read-only previews.** The "Preview / HCL" tabs show a static screenshot and static code. There is no way to tweak a property and watch the component respond.
- **No deep-link or search.** With 50+ components, discovery depends on the user scrolling a sidebar list. There is no search, no category filter, and no URL-addressable route to a specific component.

The Styling page also still shows HCL syntax in its code editors even though the configuration format is now XML — a consistency gap that will trip up new authors.

### Proposed Improvements

#### 1.1 Generated Gallery (Schema-Driven)

Derive the gallery XML from the component registry schemas rather than maintaining it by hand. The `nemo-registry` crate already holds `ComponentSchema` objects with name, property types, defaults, and descriptions. A build-time code generator (`xtask` or a `cargo run --bin generate-gallery`) should emit `examples/components/app.xml` from those schemas. Each component section would include:

- All property rows with their types and defaults pulled from `PropertySchema`
- Auto-generated examples covering mandatory properties
- The current `<binding>` wiring for live updates where applicable

**Benefit:** Gallery stays correct by construction. New components appear automatically.

#### 1.2 Interactive Property Playground

Add a `playground` panel to each component page. A property inspector sidebar (driven by the same schema) would render a form — inputs for strings, sliders for numbers, checkboxes for booleans, dropdowns for enums — and apply changes to the live preview component via `set_component_property()`. This is the core loop of Storybook.js and Stately Studio and dramatically reduces time-to-understand for new component authors.

Implementation path:
1. Add a `<property-inspector>` component type to the layout engine, backed by the component's registered `ComponentSchema`.
2. Wire changes through `set_component_property()` in the RHAI context, which already exists.
3. Embed one `<property-inspector>` instance per gallery page.

#### 1.3 `nemo storybook` Subcommand

Add a `storybook` subcommand to the CLI that launches the component gallery directly, without needing to know the path to the config file:

```
nemo storybook
nemo storybook --component button
nemo storybook --search chart
```

This makes the gallery a first-class developer tool rather than an example that must be found in the repository.

#### 1.4 Search and Deep Links

The sidebar list is already functional for 50 components but will not scale further. Add:

- **In-panel search** using a `<input>` bound to a filter that hides non-matching pages (RHAI on-change handler filtering `visible` property).
- **Deep links** via `gpui-router` (already a workspace dependency) so `nemo storybook --component table` opens directly to the Table page.

#### 1.5 Fix Styling Page Code Snippets

The Styling page code editors show HCL block syntax (e.g., `component "styled_card" { type = "stack" }`) rather than the XML format actually used. Update these to reflect XML attribute syntax so the gallery is a reliable reference.

---

## 2. Build and Installation

### Current State

The workspace builds a single binary (`nemo`). Packaging is limited to `scripts/bundle-macos.sh`, which wraps the binary in a `.app` bundle for macOS. There is no:

- Cross-platform packaging for Linux (`.deb`, `.rpm`, AppImage) or Windows (`.msi`, portable `.exe`)
- Developer scaffold (`nemo new my-app`)
- Hot-reload dev server
- Published binary releases or package manager entries (Homebrew tap, cargo-binstall manifest)
- `cargo install nemo` documentation

The current build also has a blocking dependency conflict: `gpui 0.2.2` requires `futures ^0.3.32`, while `nemo-data` locks to `futures 0.3.31` via its `Cargo.lock`. This prevents a clean `cargo build` on a fresh clone.

### Proposed Improvements

#### 2.1 Fix the Dependency Conflict (Immediate)

The `futures` version pin in `nemo-data/Cargo.toml` should be relaxed from a locked `0.3.31` to a workspace-inherited `^0.3` so it resolves to `0.3.32+` alongside gpui. Update `Cargo.toml`:

```toml
# workspace Cargo.toml — already correct
futures = "0.3"

# nemo-data/Cargo.toml — inherit from workspace
futures.workspace = true
```

Then run `cargo update -p futures` to resolve.

#### 2.2 `nemo new` Scaffold Command

A `new` subcommand that generates a ready-to-run project directory is the single highest-leverage improvement for adoption:

```
nemo new my-app
nemo new my-dashboard --template data-binding
nemo new my-tool --template calculator
```

Scaffolded output:

```
my-app/
  app.xml          # minimal working config
  scripts/
    handlers.rhai  # empty handler file with examples commented out
  plugins/         # empty, gitignored
  README.md
```

Templates (`basic`, `data-binding`, `calculator`, `complete`) would mirror the existing examples. The subcommand lives in the `nemo` crate alongside the existing `args.rs`/`main.rs`.

#### 2.3 Hot-Reload Dev Mode

The `notify` crate is already a workspace dependency but unused. A `--watch` flag (or a `nemo dev` subcommand) should:

1. Start the app normally.
2. Spawn a `notify::Watcher` on the config directory and any `<include>` paths.
3. On change, re-parse and re-validate the config, then post a GPUI model update to trigger a layout rebuild without restarting the process.

The extension manager already has a `reload_script` method stub; the layout engine needs a corresponding `rebuild_from_config` path. This is the single highest-value improvement for the inner development loop.

#### 2.4 Cross-Platform Packaging

Add packaging for all three platforms. Recommended approach: use `cargo-bundle` (via `cargo install cargo-bundle`) driven from a CI matrix, rather than hand-rolled shell scripts. For Linux, also produce an AppImage via `appimagetool`. CI pipeline additions:

| Platform | Format | Tool |
|----------|--------|------|
| macOS | `.app` + `.dmg` | `cargo-bundle` or existing script |
| Linux | `.deb`, AppImage | `cargo-deb`, `appimagetool` |
| Windows | `.msi` | `cargo-wix` |

#### 2.5 Distribution

- **Homebrew tap:** `geoffjay/homebrew-nemo` with a formula pointing to the GitHub release binary.
- **cargo-binstall manifest:** Add `[package.metadata.binstall]` to the main `Cargo.toml` so `cargo binstall nemo` works.
- **GitHub Releases:** The CI release workflow should upload artifacts for all three platforms and auto-generate release notes from `CHANGELOG.md`.

#### 2.6 `validate` as a Standalone Command

The `--validate-only` flag already exists. Promote it to a subcommand for discoverability:

```
nemo validate app.xml
nemo validate app.xml --strict
```

Strict mode would warn on deprecated properties, missing `id` attributes, and unused templates.

---

## 3. Configuration Authoring

### Current State

Nemo uses XML for all application configuration. A JSON Schema (`schema/nemo.xsd`) exists. The configuration is expressive — variables, templates, slots, data bindings, expressions, multi-file includes — but authoring it is a raw-text experience with no editor assistance beyond basic XML syntax checking.

Key friction points:
- No autocomplete for component names, attribute names, or attribute values (e.g., variant enum values for `<button>`).
- No hover documentation.
- No inline validation against the component schema (wrong attribute type, unknown component name, missing required attribute).
- No live preview while editing.
- Expression syntax (`${}`) and binding paths (`data.source_name`) are opaque to editors.
- The XSD does not fully encode the component-specific property schemas (it can't know that `variant="primary"` is valid for `<button>` but not for `<label>`).

### Proposed Improvements

#### 3.1 XML Language Server (LSP)

Build a `nemo-lsp` binary (a new crate) that implements the Language Server Protocol for `.xml` files that contain a `<nemo>` root element. The LSP speaks JSON-RPC over stdio and plugs into VS Code, Neovim, Zed, and any LSP-capable editor without per-editor plugins.

Capabilities to implement (in priority order):

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

The LSP can re-use `nemo-config`'s parser and `nemo-registry`'s schema access directly since both are library crates. The server binary is ~500 lines of plumbing around `tower-lsp`.

#### 3.2 VS Code Extension

A thin VS Code extension (`nemo-vscode`) that:

1. Registers `nemo-lsp` as the language server for files with a `<nemo>` root.
2. Provides syntax highlighting via a TextMate grammar (XML base grammar + Nemo-specific token scopes for expression syntax `${...}` and data binding paths).
3. Adds a "Preview in Nemo" command that saves the current file and runs `nemo --app-config <file>` in a terminal pane.

Publish to the VS Code Marketplace; the extension itself is ~200 lines of TypeScript.

#### 3.3 Live Preview Server (`nemo dev`)

The `nemo dev` command (from §2.3) should also open a side-by-side preview if an IDE integration is running. For VS Code, this can be a WebView panel that hosts a screenshot refreshed on save (using GPUI's headless render path, or simply a process restart + screenshot on macOS via `screencapture`). A lower-tech version is simply launching the application in a floating window that auto-reloads on save.

#### 3.4 Improve the Expression Language

The current expression resolver supports `${var.name}`, `${env.VAR}`, conditional ternary, and six built-in functions. Add:

- `${config.section.key}` — read from an external TOML/JSON config file referenced in `<app>`.
- `${data.source_name.field}` — inline data binding without a separate `<binding>` child element (shorthand for simple cases).
- Better error messages: "Undefined variable 'var.api_bas'" should suggest "Did you mean 'var.api_base'?" using edit-distance lookup.

#### 3.5 Documentation: Interactive Reference

The existing `docs/public/configuration.md` is comprehensive but static. Augment it with an embedded component explorer (the storybook from §1) deployed alongside the MkDocs site, so developers can see live previews of configuration snippets without installing Nemo.

---

## 4. LLM Chat Interface for Configuration Generation

### Current State

The cold-start problem for Nemo is significant: a new application author must understand the XML structure, component names, property syntax, data source types, RHAI handler conventions, and template mechanics before producing a working app. The existing examples help, but there is no guided authoring experience.

Nemo already has everything needed to host an LLM-assisted authoring flow:
- `<http>` data source for API calls
- `<input>` and `<textarea>` components for chat UI
- RHAI scripting for request construction and response parsing
- `<code-editor>` for displaying generated XML

### Proposed Improvements

#### 4.1 `nemo generate` CLI Command

A subcommand that takes a natural language description and produces a starter `app.xml`:

```
nemo generate "A dashboard with a sidebar, a table of users from a REST API, and a refresh button"
nemo generate "A settings panel with theme toggle and data source configuration"
nemo generate --from screenshot.png  # describe UI from an image
```

Implementation:
1. Build a system prompt that includes the full component schema (auto-generated from `nemo-registry`) and all property documentation.
2. Include 3-5 few-shot examples (the existing examples directory).
3. Call the Claude API (or any OpenAI-compatible endpoint) with the user's description.
4. Stream the XML response to stdout; detect `<nemo>...</nemo>` boundaries and write to `app.xml`.
5. Immediately run `nemo validate app.xml` and, if it fails, re-send the validation errors to the LLM for a correction pass.

The system prompt preamble should be auto-generated at build time from the schema so it stays current as components evolve.

#### 4.2 Built-in Chat Component (`<chat>`)

Add a `<chat>` component type that renders a persistent chat interface backed by an HTTP data source. Unlike a generic HTTP polling source, `<chat>` maintains conversation history in component state and streams responses:

```xml
<chat id="assistant"
      api-url="${env.ANTHROPIC_API_URL}"
      api-key="${env.ANTHROPIC_API_KEY}"
      model="claude-sonnet-4-6"
      system-prompt="You are a Nemo configuration assistant..."
      placeholder="Describe the UI you want to build..." />
```

This makes the LLM accessible as a first-class layout component — an AI panel can be embedded in any Nemo application, not just the authoring tool.

#### 4.3 Nemo Authoring Assistant Application

Ship a purpose-built authoring application (`examples/assistant/app.xml`) that combines:

- A chat panel backed by `<chat>` with a Nemo-specific system prompt
- A `<code-editor>` pane displaying the current generated config
- A "Launch Preview" button that writes the code-editor content to a temp file and runs `nemo --app-config <tmp>`
- A template browser (a simplified storybook) as a reference panel

This becomes the recommended entry point for new Nemo developers: describe what you want, see generated XML, launch it, iterate.

**System prompt strategy:** The prompt should include:
1. The Nemo XML reference (auto-generated from schemas, ~4,000 tokens)
2. The current file content (so the LLM can make targeted edits, not always full rewrites)
3. The last validation error (if any)
4. A constraint: always emit valid Nemo XML inside a `<nemo>` root element

#### 4.4 Configuration Diff and Apply

Rather than replacing the entire config on each generation, implement a diff/apply flow:

1. LLM proposes changes as a diff or targeted component additions.
2. A RHAI handler merges the proposed changes into the live config tree.
3. Hot-reload (§2.3) picks up the written file change and updates the running app.

This makes the authoring loop feel like pair programming with an AI rather than generate-and-replace.

---

## Priority Matrix

| Improvement | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| Fix futures dependency conflict | Unblocks builds | Low | **Immediate** |
| Hot-reload dev mode (`--watch`) | Daily dev loop | Medium | P0 |
| `nemo new` scaffold | Onboarding | Medium | P0 |
| Fix Styling page XML snippets | Gallery accuracy | Low | P1 |
| Schema-driven gallery generation | Gallery correctness | Medium | P1 |
| `nemo storybook` subcommand | Discoverability | Low | P1 |
| XML LSP server (`nemo-lsp`) | Config authoring | High | P1 |
| `nemo generate` CLI | AI-assisted start | Medium | P2 |
| Interactive property playground | Component exploration | High | P2 |
| VS Code extension | Editor integration | Medium | P2 |
| Cross-platform packaging | Distribution | Medium | P2 |
| Built-in `<chat>` component | AI in apps | High | P3 |
| Authoring assistant application | Full guided flow | High | P3 |
| Homebrew tap / cargo-binstall | Distribution | Low | P3 |

---

## Dependency and Risk Notes

- **futures conflict** blocks all development on a fresh clone and should be fixed before any other work.
- **Hot reload** is a prerequisite for the live preview features in §3.3 and §4.3. The `notify` crate is already present; the remaining work is the layout rebuild path.
- **nemo-lsp** reuses `nemo-config` and `nemo-registry` as libraries — no architectural changes needed. The main risk is LSP protocol compliance, mitigated by using `tower-lsp` which handles the JSON-RPC layer.
- **`nemo generate`** requires an API key at runtime. Ship it with `--api-url` and `--api-key` flags (or `NEMO_LLM_URL` / `NEMO_LLM_KEY` env vars) so it works with any OpenAI-compatible endpoint, not just Anthropic.
- The **plugin factory gap** noted in the code review (plugins can declare schemas but not provide runtime instances) does not block any of the above improvements but should be addressed before the plugin system is promoted as stable.
