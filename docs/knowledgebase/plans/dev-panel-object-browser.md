---
type: Plan
title: Dev panel object browser — live component node tree + details pane
description: A new page (tab) in the dev panel displaying the running configuration's component node tree (left) with per-node details (right), sourced from the in-process LayoutManager — not the accessibility tree, which gpui-pre does not expose.
tags: [dev, dev-panel, inspector, planning]
timestamp: 2026-10-30T00:00:00Z
---

# Dev panel object browser

A new tab in the dev panel (`nemo dev` → `ctrl-shift-e`) that shows the **live
component node tree** of the running app: a tree on the left, details of the
selected node on the right.

# Research: why not the accessibility tree (xa11y)

The original idea was to reuse the `agent-desktop` approach (built on the
[`xa11y`](https://crates.io/crates/xa11y) crates) and read the application's
accessibility node tree from the dev panel. This was evaluated and **rejected**:

* `agent-desktop observe --pid <nemo>` on a running gallery (verified
  experimentally) returns exactly three elements: `application → window →
  group`, and depth 2 is a hard ceiling — `--max-depth 20` and
  `--include-hidden` change nothing.
* The reason: `gpui-pre-apple 0.3.10` (the pinned gpui snapshot) registers no
  accessibility tree. Its macOS backend has no `NSAccessibility`/`AXUIElement`
  conformance; the observed `group` is a macOS-created window stub.
* Reaching a real AX tree would mean implementing accessibility in the external
  `gpui-pre` snapshot crates — a large, off-repo effort that only makes sense
  as an accessibility feature, not an introspection channel. (Mainline gpui
  ships AX support; our snapshots predate it.)

# Decision: in-process introspection

The authoritative node tree already lives in the same process as the dev
panel: [`NemoRuntime.layout_manager`]
(`crates/nemo/src/runtime.rs`) — `Arc<RwLock<LayoutManager>>` holding a flat
`HashMap<String, BuiltComponent>`:

```
BuiltComponent {
    id: String,                          // unique node id (or synthetic)
    component_type: String,              // "stack", "button", …
    properties: HashMap<String, Value>,  // resolved (post-binding)
    handlers: HashMap<String, String>,   // event -> handler name
    children: Vec<String>,               // child ids, in order
    parent: Option<String>,
}
```

plus `root_id()`, `component_ids()`, `get_component()`. This is the same map
`render_layout` snapshots every frame (`crates/nemo/src/app.rs:787`), so the
browser shows what is actually mounted — including runtime-created `__dyn_N`
and `__anon_N` nodes, list-binding instances, and post-expansion SFC bodies,
all of which are invisible in the source file.

Advantages over AX: zero new dependencies, no inter-process plumbing, live
data, works cross-platform, and property values are the *resolved* ones
(bindings applied), which is what a debugger wants.

# UI

The dev panel window gains a tab switch in its toolbar: **Files** (existing
tree + editor) and **Objects** (the browser). Object-browser page:

* **Left** — component tree from a `LayoutManager` snapshot: `TreeItem`s
  (reuse the same gpui-kit `Tree` widget as the file tree: `TreeItem { id,
  label, children }`). Label format: `type #id` (id omitted when synthetic).
  Root = `root_id()`.
* **Right** — details of the selected node:
  * Header: type + id (+ parent id).
  * Properties table (key → resolved value).
  * Handlers table (event → handler name).
  * Bindings list (property → binding source), when present.
  * Children summary (count + ids).

## Snapshot & refresh

* Snapshot `LayoutManager` under its lock at refresh time (mirroring
  `render_layout`'s single-lock snapshot; never hold the lock across render).
* Refresh triggers: a **Refresh** toolbar button, plus an automatic
  re-snapshot when the app re-loads configuration (hot-reload /
  compile actions already bump the runtime) — initially manual refresh only;
  auto-refresh on data-dirty is a later refinement.

# Phasing

## Phase 1 — Objects tab + tree pane

**Status: implemented.**

* `workspace/dev_panel.rs` — `Tab` (`Files` | `Objects`), toolbar tab switch,
  `ObjectBrowser` state (`tree_state` + `snapshot` + `bindings` + selection),
  `refresh_object_browser` snapshot, `build_object_tree`/`node_tree_item`
  walk, left tree + right details shell.

## Phase 2 — Details pane

**Status: implemented.** Header (type, id with `(generated)` marker, parent),
Properties (sorted key/value rows, values resolved), Handlers, Bindings
(from `LayoutManager::bindings().bindings_for_component`), Children summary.

## Phase 3 — Live coordination

**Status: implemented** (manual refresh). The Refresh button re-snapshots the
`LayoutManager`; the snapshot happens on tab entry too. Auto-refresh on
hot-reload remains a later refinement.

# Critical files

| File | Role |
|---|---|
| `crates/nemo/src/workspace/dev_panel.rs` | Tab shell, node tree build, details pane |
| `crates/nemo-layout/src/manager.rs` | Read-only accessors (already public; no changes expected) |
| `docs/knowledgebase/plans/config-dev-env.md` | Cross-reference the new tab |

# Verification

* `nemo dev --app-config examples/components/app.nemo` → open dev panel →
  Objects tab shows the full rendered tree (root stack, every section, page
  SFC bodies expanded with real ids).
* Selecting a node shows its resolved properties (e.g. a button's `variant`);
  selecting a router node shows its route children.
* Edit a file in Files (add a button), Compile/Save → Objects → Refresh
  shows the new node; details pane matches.
* Non-dev run: `ctrl-shift-e` remains a no-op (existing gate).