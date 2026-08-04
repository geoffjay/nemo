---
type: Plan
title: Configuration development environment — in-app config editor, file tree, and compile
description: A dev-mode configuration editor (file tree + syntax-highlighted editor + compile actions) that toggles open over the running app and can detach to a second GPUI window. Only available under `nemo dev`.
tags: [config, dev, editor, build, workspace, planning]
timestamp: 2026-08-03T00:00:00Z
---

# Configuration development environment

The end goal for nemo is that all configuration files are nemo SFC files (`.nemo`)
that get built and loaded. A built-in configuration development environment lets a
developer load, edit, and build those files without leaving the application.

This plan adds a **configuration dev panel**: a toggle panel that overlays the running
app under `nemo dev`, with a collapsible file tree (left), a syntax-highlighted code
editor (center), and compile actions (compile current file / build whole project).
The panel can **detach into a second GPUI window** and re-dock.

# Why (today there is no in-app config editor)

`nemo dev` (`commands/dev.rs`) runs the app with hot-reload but offers no editing UI
— the developer edits files in an external editor. The existing `CodeEditor` component
(`components/code_editor.rs`) is a thin wrapper over `gpui_component::input::Input`
in `code_editor` mode; it has no file I/O and is config-driven (built from
`BuiltComponent`), not usable as a standalone workspace tool. There is no file-tree
browser, and `nemo build` is a CLI-only command.

So this plan adds:

1. a **dev panel** entity (`workspace/dev_panel.rs`) — file tree + code editor +
   toolbar — that renders over the running app when toggled;
2. a **toggle + detach** action model — a `ToggleDevPanel` action (keybinding) and a
   header-bar button, plus `DetachDevPanel` / `DockDevPanel` to move the panel into a
   second GPUI window and back;
3. **compile actions** wired to the existing `commands::build` logic — "Compile File"
   (`build_single_component`) and "Build Project" (`build_project`), with toast
   notifications for success/failure;
4. a **`nemo dev`-only gate** — the panel is only constructible when the workspace was
   launched in dev mode (a `dev_mode: bool` on `Workspace`).

# Decisions (settled with the project owner)

* **Layout: builder in a separate window.** Clicking the header-bar code button
  (or `ctrl-shift-e`) opens the dev panel in its own GPUI window, not as a
  sidebar. Clicking again when the window is already open focuses that window.
  Closing the builder window returns focus to the main window. The file watcher
  continues to hot-reload the main app from the builder window.
* **Pop-out removed.** The earlier sidebar + detach/dock model was replaced with
  the simpler window-only model above. No overlay, no detach/dock toggle.
* **Compile scope: both.** Two actions: "Compile File" (the current `.nemo` →
  `<out>/components/<tag>.json`, reusing `commands::build::build_single_component`) and
  "Build Project" (the whole project → `dist/layout.json`, reusing
  `commands::build::build_project`). Both shell out to the existing pure `Value`-tree
  transforms — no second compiler.
* **Editor: syntax-highlighted + file I/O.** The editor is `InputState` in `code_editor`
  mode with tree-sitter highlighting (`html` for `.nemo`/`.xml`, `toml` for `nemo.toml`,
  `rust`/`text` fallback). Load from disk on tree selection, save to disk on
  `ctrl-s` / Save button, with an unsaved-change indicator (dot in the tab/title).

# Architecture

## Dev panel entity

`workspace/dev_panel.rs` defines `DevPanel`, a GPUI `Render` entity:

```
DevPanel {
    project_root: PathBuf,           // config file's directory (tree root)
    selected_file: Option<PathBuf>,   // currently-open file
    dirty: bool,                      // unsaved changes
    tree_state: Entity<TreeState>,    // file-tree state (collapsible)
    editor_state: Entity<InputState>,  // code editor state
    detached_window: Option<WindowHandle<DevPanelRoot>>, // when popped out
    runtime: Arc<NemoRuntime>,        // for compile + reload coordination
}
```

* **File tree** — built by walking `project_root` (depth-limited, `.git`/`target`/
  `dist`/`.nemo` filtered), converted to `TreeItem` list (reusing
  `components::tree::values_to_tree_items` shape). `TreeState` with collapsible nodes;
  selecting a leaf loads the file into the editor.
* **Code editor** — `InputState::new(window, cx).multi_line(true).code_editor(lang)
  .line_number(true).searchable(true)`, where `lang` is derived from the file extension.
  `set_value` loads file content; `value()` reads it for save.
* **Toolbar** — Save (ctrl-s), Compile File, Build Project, Detach/Dock toggle.

## Wiring into the workspace

`Workspace` gains:

* `dev_mode: bool` — set `true` only in `build_app_window` when the launch came from
  `commands::dev::run` (thread a `dev_mode` flag through `BootstrapParams`).
* `dev_panel: Option<Entity<DevPanel>>` — `None` when closed, `Some` when open or
  detached.
* `pending_toggle_dev_panel: bool` — deferred toggle, processed in `render` (where
  `Window` is available), mirroring the existing `pending_project_path` pattern.

`Workspace::render` overlays the dev panel (when `Some` and not detached) as an
absolute-positioned panel over the app, or renders nothing when detached (the panel
lives in its own window then).

## Toggle + detach actions

New actions in `workspace/actions.rs`:

* `ToggleDevPanel` — toggles panel open/closed (only acts when `dev_mode`).
* `DetachDevPanel` — `cx.open_window` with `DevPanelRoot` (a thin `Render` wrapper over
  the shared `DevPanel` entity); on close, re-embeds.
* `DockDevPanel` — closes the detached window, re-embeds the panel in the main window.

Keybinding: `ctrl-shift-e` toggles the dev panel (only under `nemo dev`).

## `nemo dev`-only gate

`DevArgs` gains nothing new; `commands::dev::run` threads `dev_mode: true` into
`BootstrapParams`. The default run path (`run_app`) sets `dev_mode: false`. The header
bar only renders the dev-panel button when `dev_mode`, and `ToggleDevPanel` is a no-op
otherwise.

## Compile actions

"Compile File" and "Build Project" call the existing `commands::build` functions
(refactored to be `pub(crate)` and callable with a `PathBuf` target instead of
`BuildArgs`). On success, a success toast is shown; on failure, an error toast with the
error message. The build output goes to the same `<out>/` the CLI uses.

# Phasing

## Phase 1 — Dev panel entity + file tree + editor + file I/O

**Status: implemented.**
Dev panel with file tree (collapsible, filtered), code editor with syntax highlighting,
load-on-select, save (ctrl-s) with dirty indicator. Toggled open over the running app
under `nemo dev`. No compile, no detach yet.

* `workspace/dev_panel.rs` — `DevPanel` entity, file-tree build, editor wiring, save.
* `Workspace` — `dev_mode`, `dev_panel`, `pending_toggle_dev_panel`, render overlay.
* `workspace/actions.rs` — `ToggleDevPanel` action + `ctrl-shift-e` binding (dev-only).
* `commands/dev.rs` — thread `dev_mode: true` into `BootstrapParams`.
* `BootstrapParams` — add `dev_mode: bool`.

## Phase 2 — Compile actions

**Status: implemented.**
`commands::build::{build_single_component, build_project}` (refactored to `pub(crate)`
and taking a path). Toast notifications.

* Refactor `commands/build.rs` — `pub(crate) fn build_single_component(file: &Path)`
  and `pub(crate) fn build_project(root, manifest)` (already `fn`, make `pub(crate)`).
* `workspace/dev_panel.rs` — toolbar buttons + handlers + toasts.

## Phase 3 — Detach to second window

**Status: implemented.**
`DevPanelRoot` (wraps the shared `DevPanel` entity); docking closes that window and
re-embeds. The file watcher continues to hot-reload the main app from either window.

* `workspace/dev_panel.rs` — `DevPanelRoot` render wrapper, detach/dock handlers.
* `Workspace` — track `detached_window: Option<WindowHandle<DevPanelRoot>>`.

# Critical files

| File | Role |
|---|---|
| `crates/nemo/src/workspace/dev_panel.rs` (new) | `DevPanel` entity + file tree + editor + compile + detach |
| `crates/nemo/src/workspace/mod.rs` | `dev_mode`, `dev_panel`, toggle render overlay |
| `crates/nemo/src/workspace/actions.rs` | `ToggleDevPanel`, `DetachDevPanel`, `DockDevPanel` |
| `crates/nemo/src/main.rs` | `BootstrapParams.dev_mode`, `build_app_window` threading |
| `crates/nemo/src/commands/dev.rs` | set `dev_mode: true` |
| `crates/nemo/src/commands/build.rs` | `pub(crate)` compile functions for reuse |

# Reuse (avoid new code)

* `gpui_component::input::InputState` in `code_editor` mode — syntax highlighting,
  line numbers, search, multi-line. No custom editor.
* `gpui_component::tree::{Tree, TreeState, TreeItem}` — collapsible file tree.
  Reuse the `TreeItem` shape from `components/tree.rs`.
* `commands::build::{build_single_component, build_project}` — compile logic; no
  second compiler.
* `Workspace` deferred-action pattern (`pending_*` processed in `render`) — for the
  toggle, mirroring `pending_project_path` / `pending_reload`.
* `get_window_options` (`window.rs`) — second window options (titled "Nemo Dev").
* `gpui::App::open_window` — second native window.

# Verification

* **Smoke (dev):** `nemo dev --app-config examples/sfc/app.xml` → `ctrl-shift-e`
  opens the panel over the running app; file tree shows the project dir; selecting a
  `.nemo` loads it with highlighting; editing + ctrl-s saves; the running app
  hot-reloads (existing watcher).
* **Compile:** "Compile File" on a `.nemo` emits `<out>/components/<tag>.json`;
  "Build Project" emits `dist/layout.json`. Toast confirms.
* **Detach:** "Detach" opens a second window with the panel; the main window no longer
  shows it; "Dock" closes the second window and re-embeds. The app keeps running in the
  main window throughout.
* **Gate:** `nemo --app-config examples/sfc/app.xml` (no `dev`) → `ctrl-shift-e` is a
  no-op; no dev-panel button in the header.
* **Unit:** file-tree filtering, language inference by extension, dirty-state
  tracking (compile is already covered by build.rs tests).

# Knowledgebase updates required when implemented

* [Configuration](../concepts/configuration.md) — document the dev panel + in-app
  compile.
* This plan — mark phases as implemented.

# Relationship to other plans

* Builds on the [build system](build-system.md) — reuses its compile functions.
* Independent of the [page router](page-router.md) — the dev panel is a workspace
  overlay, not a routed view.