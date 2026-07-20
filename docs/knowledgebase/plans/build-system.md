---
type: Plan
title: Build system — manifest, compiled artifacts, and remote component libraries
description: A `nemo build` command with a `nemo.toml` project manifest, compiled `.nemo` component artifacts, opt-in `dist/` project builds, and Go-style remote component libraries pulled into a `.nemo/packages` cache. Expands and supersedes SFC "Phase 5".
tags: [build, config, sfc, packages, cli, planning]
timestamp: 2026-07-20T00:00:00Z
---

# Build system — manifest, compiled artifacts, and remote component libraries

The end goal driving this plan is **reusable components and component libraries**: let an
application developer build a component once and share it across many apps, ideally
pulling third-party libraries in from a single source (a git repo). This turns the
optional "Phase 5 — compiler / binary format" sketch in
[single-file components](sfc-components.md) into a designed build system and **supersedes**
that phase.

# Why (today there is no build layer)

There is **no build/dist/artifact/cache concept anywhere** in the codebase. Config is
parsed fresh from `app.xml` on every launch and every hot-reload; there is no caching of
the parsed tree. Every `src`/`href` in `<include>`/`<import>` is a filesystem path resolved
against the config file's parent directory (`xml_parser.rs` `resolve_path`), and the app is
always launched by pointing `--app-config` at a single XML file (`main.rs` `run_app`). See
[Configuration](../concepts/configuration.md).

So reusing a component across apps means copying `.nemo` files by hand, and there is no unit
of distribution and no way to depend on a remote library. This plan adds:

1. a **project manifest** (`nemo.toml`) naming the app entry, build output, and dependencies;
2. **`nemo build`** — compile an individual `.nemo` component **and** a whole project
   (project → `<projectdir>/dist/`);
3. **explicit opt-in** loading of a built `dist/` (source stays the default);
4. **remote component libraries** pulled Go-style via `<import src="github.com/…">` into a
   `.nemo/packages` dev cache, pinned by a lockfile (the ideal case, **not** critical path).

The drawback of no longer pointing at one XML file is contained: the manifest is optional
and additive. `nemo --app-config app.xml` keeps working unchanged; the manifest is what
unlocks builds and dependencies.

# Decisions (settled with the project owner)

* **Manifest = a separate `nemo.toml`** at project root — not embedded in `app.xml`, not a
  bespoke `go.mod`. Keeps `app.xml` purely UI and matches the existing TOML convention.
  **Name distinction:** the global user-prefs TOML is `config.toml`
  (`crates/nemo/src/config/`, the `--config` flag); the per-project **manifest** is
  `nemo.toml`.
* **Import syntax = XML-native** — keep `<imports><import src="github.com/…" [as="…"]/></imports>`;
  the resolver distinguishes a module path from a local path. No new mini-parser, consistent
  with today's SFC imports. (A Go-style textual `import (...)` block was considered and
  declined.)
* **`dist/` loading = explicit opt-in** — source (`app.xml`) is the default; `dist/` is used
  only when `--dist` is passed or the manifest sets `[build] load = "dist"`. No stale-cache
  surprises; `nemo dev` always stays interpret-on-change.

```toml
# nemo.toml
name  = "foo"
entry = "app.xml"          # default "app.xml"

[build]
out  = "dist"              # default "dist"
load = "source"            # "source" (default) | "dist"

[dependencies]
"github.com/geoffjay/nemo-components" = "v1.2.0"
"github.com/geoffjay/nemo-form"       = "v0.3.1"
```

# Key decision: compile ahead-of-time reusing the runtime's own transforms

The build command does **not** invent a second compiler. Every step it performs already
exists in the load path; `nemo build` just runs those pure `Value`-tree transforms
ahead-of-time instead of at startup:

* parse + `${}` resolve — `ConfigurationLoader::load`;
* SFC parse — `XmlParser::parse_sfc` → `SfcDefinition`;
* style-fold, handler-ref rewrite, nested-tag rewrite, tag→instance rewrite — the runtime's
  `collect_sfc_tags` / `rewrite_sfc_handlers` / `rewrite_sfc_tags`;
* template/slot/id-scope expansion — `expand_children` / `expand_template`.

These operate on the homogeneous `Value` tree with no GPUI dependency, so they can be
factored into functions callable from `commands/build.rs`. The artifact is **JSON** (SFC
Phase 5's choice: inspectable, proven by `nemo schema` and `cargo xtask design-export`;
`Value` is already serde-serializable). By dist-load time the tree is the same resolved
`Value` the source path produces, so `parse_layout_config`, `LayoutBuilder`, and
`LayoutManager` are all unchanged.

# Phasing

Phases 0–2 are the critical path; Phase 3 (remote packages) is the ideal-case follow-on.
Each phase is independently shippable.

## Phase 0 — Manifest, project-root discovery, `nemo build` skeleton

* New `crates/nemo-config/src/manifest.rs`: `ProjectManifest { name, entry, build, dependencies }`
  deserialized with `toml`; re-export from `nemo-config/src/lib.rs`.
* `find_project_root(start) -> Option<PathBuf>`: walk up from cwd/target to find `nemo.toml`.
  This is the project's only marker file — there is no project-root concept today
  (`ActiveProject` is GUI state, not a path).
* CLI, mirroring the four existing subcommands: add `Build(BuildArgs)` to `Command`
  (`crates/nemo/src/args.rs`), declare `pub mod build;` in `commands/mod.rs`, add
  `commands/build.rs` (`pub fn run(BuildArgs) -> anyhow::Result<()>`), and a dispatch arm in
  `main.rs`. Phase-0 `run` resolves the manifest/root and prints the build plan (dry run).
* Manifest-aware launch in `run_app`: when `--app-config` is a directory or omitted, use
  `find_project_root` + manifest `entry`; when it is a file (today's behavior), skip the
  manifest. Additive — existing invocations untouched.

## Phase 1 — Compile an individual component file

`nemo build path/to/button-group.nemo` → a compiled component artifact (the unit a library
ships).

* Reuse `parse_sfc` → `SfcDefinition`; run style-fold (SFC Phase 3) and handler-ref rewrite
  (SFC Phase 1) ahead-of-time via the factored transforms.
* Artifact JSON: `{ tag, template (style-folded Value), script?, meta: { name, source } }`,
  written to `<out>/components/<tag>.json` (a `.nemoc` extension is cosmetic; defer).
* A **component library** is a package directory of `.nemo` sources plus a `nemo.toml` whose
  `[package] exports = [...]` (or, by convention, every top-level `.nemo`) lists exported
  tags. `nemo build` on a package emits all component artifacts.
* **Verify:** a compiled artifact re-loaded yields a `TemplateMap` entry identical to the one
  `parse_layout_config` builds from source (round-trip).

## Phase 2 — Build a project to `dist/`, load it with `--dist`

`nemo build examples/foo` → `examples/foo/dist/`; `nemo run foo --dist` (or manifest
`load = "dist"`) loads the built tree, skipping parse/expand.

* Build runs the pipeline **once**: `ConfigurationLoader::load` → SFC registration +
  style-fold + `rewrite_sfc_tags` → `expand_children` → serialize the **post-`expand_children`
  `Value`** (SFC Phase 5's recommended boundary). Emit the resolved scripts too (copy `.rhai`
  + SFC `<script>` bodies) so a `dist/` load needs no source tree. Layout:
  `dist/{layout.json, scripts/…, components/…, lock?}`.
* Serialize boundary: post-expand `Value` (JSON). The `LayoutConfig`/`LayoutNode` AST
  (`crates/nemo-layout/src/node.rs`, already serde-derived) is an alternative that also
  skips `LayoutBuilder::build`; deriving `Serialize` on `BuiltComponent` would go further but
  couples the cache to render types — defer (per SFC Phase 5).
* Load: `ConfigurationLoader::load_from_dist(dir)` returns the same resolved `Value` the
  source path produces; gate on `--dist` / manifest `load` in `run_app`. **Default stays
  source; `nemo dev` never uses dist.**
* **Verify:** built `dist/` render tree byte-identical to the source render tree; editing a
  `.nemo` and re-running without rebuild still loads source (opt-in semantics hold).

## Phase 3 — Remote component libraries (ideal case; NOT critical path)

`<import src="github.com/geoffjay/nemo-components"/>` pulls a versioned library into a
`.nemo/packages` dev cache and exposes its exported tags — the Go module model, XML-native.
Grouped/multiple imports are just multiple `[dependencies]` keys / `<import>` elements.

* **Fetch** — isolate git/network in a new small crate `crates/nemo-pkg` (keeps VCS deps out
  of `nemo-config`). `nemo get` (new subcommand, or folded into `nemo build`): for each
  dependency, `git clone`/`fetch` `https://<modulepath>` at the tagged version into
  `.nemo/packages/<modulepath>@<version>/`. `.nemo/packages` and `dist/` are gitignored.
* **Lockfile** `nemo.lock` pins resolved commit hashes (Go's `go.sum` / `Cargo.lock` role);
  `nemo build`/`nemo get` writes it, loads honor it — reproducible, offline once fetched.
* **Resolution** — extend `resolve_path` and `process_import` (`xml_parser.rs`) so a `src`
  that parses as a **module path** (has a host segment; no `./`/`../`/absolute prefix; not an
  existing file) resolves against `.nemo/packages/<path>@<lockedversion>` instead of the
  local filesystem. A module import brings in **all** exported tags of the package (Go
  imports a package, not a file); `as="nf"` becomes a tag prefix/namespace.
* **Fix a known gap:** `process_import` currently builds its SFC sub-parser **without**
  `with_base_dir`, so relative paths inside a fetched `.nemo` would resolve CWD-relative.
  Propagate `base_dir = import_path.parent()` (mirroring `process_include`) so
  package-internal imports resolve within the package.
* **Verify:** e2e — a project depending on a small fixture repo; `nemo get` populates
  `.nemo/packages`, `nemo build`/run uses an imported tag; lockfile determinism (same inputs
  → same lock).

# Critical files

| File | Role |
|---|---|
| `crates/nemo-config/src/manifest.rs` (new) | `ProjectManifest` (nemo.toml), `find_project_root` (P0) |
| `crates/nemo-config/src/lib.rs` | re-export manifest types + `load_from_dist` |
| `crates/nemo-config/src/loader.rs` | `load` reused; add `load_from_dist` (P2) |
| `crates/nemo-config/src/xml_parser.rs` | `resolve_path` + `process_import` module-path resolution & base_dir fix (P3) |
| `crates/nemo/src/args.rs` | `Build` (+ `Get`) `Command` variants + arg structs |
| `crates/nemo/src/commands/{mod.rs,build.rs,get.rs}` | new build/get handlers, mirroring `dev.rs`/`schema.rs` |
| `crates/nemo/src/main.rs` | dispatch arm + manifest-aware `run_app` path resolution |
| `crates/nemo/src/runtime.rs` | factor the pure SFC compile transforms (`collect_sfc_tags`, `rewrite_sfc_tags`/`rewrite_sfc_handlers`, `expand_children`) for reuse from `build.rs` |
| `crates/nemo-layout/src/node.rs` | serde boundary alternative for dist (P2) |
| `crates/nemo-pkg` (new crate) | git fetch + lockfile for remote packages (P3) |
| `.gitignore` | ignore `dist/` and `.nemo/packages/` |

# Reuse (avoid new code)

* `ConfigurationLoader::load` / `load_xml_string` — parse + resolve.
* `XmlParser::parse_sfc`, `resolve_path`, and `process_include`'s child-parser base_dir
  pattern.
* Runtime SFC + expansion machinery (`runtime.rs`) already does style-fold, handler rewrite,
  slot injection, id-scoping — build runs the same transforms ahead-of-time.
* `toml` crate + serde `Value` (JSON) already in the tree; no new serialization stack.
* CLI subcommand shape from `commands/{dev,schema,validate,new}.rs`; `xtask design-export`
  and `nemo schema --output` as the file-emitting precedents.

# Verification

* **Unit (nemo-config):** manifest parse & defaults; `find_project_root` walk-up; a
  module-path-vs-local classifier on `resolve_path`; compiled-component round-trip; dist-vs-
  source render-tree equality.
* **End-to-end (macOS; local builds need `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`):**
  a new `examples/foo/` with `nemo.toml` + `app.xml` + a local `.nemo` component; `nemo build`
  produces `dist/`; run source (default) and `--dist` and confirm identical render (`nemo
  screenshot` behind the opt-in `screenshot` feature; `nemo-run` skill). P3: add a dependency
  to a fixture repo, `nemo get`, confirm `.nemo/packages` + lockfile + imported tag renders.
* Add `examples/foo` to the CI `validate-examples` job once it passes `nemo validate --strict`.

# Relationship to other plans

* **Supersedes** SFC [Phase 5](sfc-components.md) (compiler / binary format) — that section
  becomes this plan.
* **Independent of** the [page router](page-router.md): expanded SFCs are ordinary built-ins
  by build time, so a built `dist/` contains resolved router arms with no special handling.
  The SFC caveat about scoping a `<router>` nested inside an SFC is orthogonal to the build
  layer.

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — document the manifest, project-root
  discovery, the `dist/` load path, and module-path import resolution.
* A new [pattern](../patterns/index.md) for authoring & consuming component libraries.
* [Roadmap](roadmap.md) — add the build system as phases land; mark SFC Phase 5 superseded.
