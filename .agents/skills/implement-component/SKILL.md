---
name: implement-component
description: Implement one missing gpui-kit component in Nemo. Finds a "Not implemented" placeholder page in examples/components/templates/pages/components (or takes a component name as the argument), researches the gpui-kit component via its documented source link and existing Nemo component patterns, produces an implementation plan, implements the 4-file wrapper, verifies it with a fixture screenshot, and rewrites the placeholder gallery page with real examples. Use when asked to implement a component, wrap a gpui-kit component, or fill in a component placeholder.
---

# /implement-component — wrap one gpui-kit component into Nemo

Implement **one** component per run: pick it, research it, plan it, implement it,
verify it with a screenshot, and update its gallery page.

## Inputs

- `$ARGUMENTS` — optional component name (kebab-case, e.g. `rating`, `data-table`).
  Case-insensitive; `Data Table` → `data-table`.
- With a name: go straight to *Step 2* (the placeholder file must still exist at
  `examples/components/templates/pages/components/<name>.nemo`; if it doesn't,
  the component is already implemented — say so and stop).
- Without a name: run *Step 1*.

## Step 1 — Pick a component

1. List placeholders:
   ```sh
   grep -l "Not implemented" examples/components/templates/pages/components/*.nemo
   ```
2. Each file's header comment names the gpui-kit component; its
   `<label id="<name>_source" ...>` line carries the gpui-kit doc URL.
3. Choose the **simplest unimplemented component** (stateless display/input
   components before stateful list widgets). Order the candidates roughly as:
   - Stateless, no children (label-like: `kbd`, `marker`, `skeleton`, `stepper`) — first.
   - Stateless with children / slots (`badge`-style wrappers, `bubble`, `message`).
   - Interactive but stateless-per-render (`pagination`, `menu`, `command`, `carousel`).
   - Stateful widgets (Entity-backed: `calendar`, `rating`, `data-table`,
     `virtual-list`, `message-scroller`, `dock`, `settings`, `questionnaire`,
     `diff`, `attachment`) — last; each needs a `ComponentState` variant (see
     *Stateful components* below).
4. Announce the pick and the full remaining list, so the next invocation can
   continue where this one stopped.

## Step 2 — Research (before writing any code)

Gather, in this order:

1. **The gpui-kit page** linked in the placeholder (`https://gpui-kit.com/component/<name>`)
   via web fetch/search. Extract: purpose, every prop with name/type/default,
   variants, events/callbacks, and the markup examples shown there.
2. **gpui-kit's source** — the component's Rust implementation lives in the
   `gpui-kit` dep (crates.io `gpui-kit 0.8.1`). Find its checkout:
   ```sh
   find ~/.cargo/registry/src -maxdepth 1 -name "*gpui-kit-0.8*" -type d
   ```
   then read `src/component/<name>.rs` (or locate by name) for the real API:
   struct fields, builder methods, variants, defaults. The web page may lag
   the crate; the crate is ground truth for what the wrapper can pass through.
3. **Nearest existing Nemo wrapper(s)** in `crates/nemo/src/components/` —
   read 1–2 components of the same shape (children-bearing: `badge.rs` or
   `collapsible.rs`; simple stateless: `label.rs` or `tag.rs`; themed icons:
   `icon.rs`). Copy their structure; do not invent a second convention.
4. **Registry conventions** — skim 1–2 registrations in
   `crates/nemo-registry/src/builtins.rs` to match `ConfigSchema` style.

Note any gpui-kit capability that cannot be expressed (e.g. hooks/context not
bridged by the runtime) explicitly in the plan as out of scope — don't silently
drop props.

## Step 3 — Plan (write it down before implementing)

Produce a short plan with:

1. **4-file touch list** (see `nemo-component-patterns` skill for full patterns):
   1. `crates/nemo/src/components/<name>.rs` — struct + `RenderOnce`.
      `#[derive(IntoElement, NemoComponent)]`, `#[property]` fields (macros
      support `String`, `i64`, `f64`, `bool` + `Option<T>` of those),
      `#[children]` if it wraps content, `#[source]` **last**.
      Components with event handlers also need `runtime: Option<Arc<NemoRuntime>>`
      and `entity_id: Option<EntityId>` fields + builder methods.
   2. `crates/nemo/src/components/mod.rs` — `mod <name>; pub use <name>::<Name>;`
   3. `crates/nemo-registry/src/builtins.rs` — `reg(...)` with `ConfigSchema`
      (snake_case name; XML uses kebab-case; every prop the page will use).
   4. `crates/nemo/src/app.rs` — match arm in `render_component()`:
      children-bearing components must call `self.render_children(...)` and
      pass `.children(children)`; stateful widgets call
      `self.get_or_create_<name>_state(...)` and pass the entity.
2. **Stateful?** If the gpui-kit component holds selection/scroll/value state
   across re-renders, add a `ComponentState` variant in
   `crates/nemo/src/components/state.rs`, a `get_or_create_*_state()` in
   `app.rs`, and an `Entity<T>` field on the struct — follow
   `slider.rs`/`table.rs` plus the stateful-widget-persistence pattern.
3. **Gallery page design** — which examples the page will show (aim for 2–4:
   default + 1–2 prop variants), reusing
   `examples/components/templates/component-example.nemo` (Preview/Code tabs)
   and the properties table from an existing page (e.g. `badge.nemo`).
4. **Fixture plan** (next step) — the route/name to use.
5. **Out of scope** list, if any (from Step 2).

Do not implement yet if the plan has open questions about the gpui-kit API;
resolve them from the crate source first.

## Step 4 — Implement

Follow the plan; the 4-file pattern is mandatory. Additional constraints:

- Colors go through `resolve_color()` (`theme.*`/hex); shadows/rounding through
  `apply_shadow`/`apply_rounded` presets. Never hardcode palette hexes.
- Text/sizing via existing tokens; look at how the closest sibling maps
  gpui-kit props → gpui calls rather than re-deriving.
- Every attribute the gallery page or fixture uses **must** exist in the
  schema — `nemo validate --strict` warns on unknown attributes and the CI
  validate-examples job treats the page as a contract.

## Step 5 — Test configuration + screenshot verification

Verify with a **standalone fixture** that mounts the page directly — NOT the
full gallery. (The gallery is an `app-shell` whose `default-page="overview"`
hides component pages, and `nemo screenshot --route` doesn't reach the inner
router when `--app-config` is given; a fixture sidesteps both.)

1. Create a throwaway fixture (do **not** commit it):
   `examples/components/fixtures/<name>-test.nemo`:
   ```xml
   <app title="Fixture <Name>">
     <theme name="tokyo-night" mode="dark" />
   </app>

   <import src="../templates/component.nemo" />
   <import src="../templates/component-example.nemo" />
   <import src="../templates/pages/components/<name>.nemo" />

   <template name="app">
     <stack id="root_page" direction="vertical" padding="24">
       <component-page-<name> />
     </stack>
   </template>
   ```
   (Entry files are SFCs: `<app>` + `<import>` + a top-level
   `<template name="app">`; a bare `<template>` with no `<app>` fails to parse.
   The `../templates` relative paths resolve against the fixture's own
   directory, so it must live in `examples/components/fixtures/`.)
2. Validate and capture (**macOS only**; the screenshot feature is macOS-first):
   ```sh
   ./target/debug/nemo validate examples/components/fixtures/<name>-test.nemo --strict
   DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
     cargo run -p nemo --features screenshot -- \
     screenshot --app-config examples/components/fixtures/<name>-test.nemo \
     --out /tmp/<name>-implement.png --size 1200x900 --settle-ms 800
   ```
3. **Look at the PNG** (it's an image — view it, don't trust the exit code).
   Confirm: component renders, prop variants visibly differ, no blank/black
   regions, no overlapping text. Rebuild first if the binary predates the
   implementation (`cargo build --features screenshot -p nemo` — a stale
   `target/debug/nemo` silently renders the placeholder).
4. Iterate: implementation or fixture edits, re-validate, re-capture, until the
   image is correct.
5. Also capture light mode once (`--theme gruvbox --mode light`) when the
   component resolves theme colors, to catch hardcoded-color mistakes
   (`tokyo-night` is dark-only — use `gruvbox`/`nord`/`catppuccin` for light
   captures).
6. Delete the fixture directory when done — it must never be committed
   (`git status` must be clean of `examples/components/fixtures/`).

## Step 6 — Update the gallery page

Rewrite `examples/components/templates/pages/components/<name>.nemo`:

- Drop the placeholder comment and the whole `<stack id="<name>_status">`
  block (`Not implemented` + `Source:` labels). The `<template
  name="component-page-<name>">` wrapper and the existing
  `<component-page name="..." description="...">` stay.
- Fill 2–4 `<component-example>` sections matching Step 3's design. Follow a
  completed page (e.g. `badge.nemo`):
  - live widgets in the default slot, **every element needs an explicit `id`**
    prefixed `<name>_` (parse-time `__anon_N` ids collide across SFC
    instances — id-less children break the page chrome),
  - copy-ready markup in `slot="code"` via a `code-editor` whose
    `default-value` comes from a `<script>`-defined raw string
    (`let x = #"..."#;` interpolated as `{{ x }}`),
  - a `<table slot="properties">` listing each attribute (name/type/default/
    description rows) — match what Step 2 extracted from gpui-kit.
- If the component accepts event handlers, show one in an example and list it
  in a `<table slot="callbacks">` (mirror an existing page that has one).

## Step 7 — Wrap up

1. Full verification, mirroring CI:
   ```sh
   cargo build -p nemo && cargo test -p nemo-config -p nemo -- <name> 2>/dev/null || true
   cargo clippy --all-targets -- -D warnings 2>&1 | grep -E "^error" | head
   cargo fmt --all -- --check
   ./target/debug/nemo validate examples/components/app.nemo --strict
   ```
2. Screenshot **the real gallery page** for the record (fixture is gone by
   now, so navigate via the running app or a temporary second fixture if the
   gallery route cannot be reached headlessly — do not commit that one either).
   Prefer: re-create the fixture against the *final* page, capture, delete.
3. Move/replace the placeholder's note: the component page should no longer
   contain the string `Not implemented`.
4. Report: component name, the 4 files touched, registry schema props,
   fixture screenshot path, and the remaining `Not implemented` pages:
   ```sh
   grep -l "Not implemented" examples/components/templates/pages/components/*.nemo
   ```

## Pitfalls (learned the hard way)

- **Every SFC child needs an explicit `id`** — anonymous children get
  parse-time `__anon_N` ids that collide across file instances.
- **`#[source]` must be the last field** of the component struct.
- **Stateful widgets need definite height** — `uniform_list`-based components
  (table, tree, virtual-list) collapse to 0px without
  `div().w_full().h(px(height))`.
- **Warnings vs errors**: `--strict` fails on errors, not unknown-attribute
  warnings — but a schema missing an attribute the page uses is still a bug
  (the page will silently drop the attribute at runtime).
- **gpui-kit crate source is ground truth**, not the website docs; prop
  defaults and variant names have drifted before.
- **One component per run.** If research reveals the pick is blocked (missing
  upstream feature, needs an unimplemented primitive), document the blocker,
  pick the next candidate, and note the blocked one last in the report.