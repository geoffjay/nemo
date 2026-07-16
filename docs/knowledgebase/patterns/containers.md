---
type: Pattern
title: Containers
description: High-level layout containers that package a common app layout with slot regions.
tags: [components, layout, containers]
timestamp: 2026-07-15T00:00:00Z
---

**Containers** are complex components that package a common application layout so
authors describe *intent* rather than *layout mechanics* (nested stacks, fixed
widths, page-toggle handlers). This keeps developer effort on Rhai scripts and
plugins. They live in `crates/nemo/src/containers/`, separate from
`components/`, but wire in through the same four points as any component (see
[four-file component workflow](four-file-component-workflow.md)).

# app-shell

The first container. A standard app frame:

```xml
<app-shell sidenav-width="200">
  <app-sidenav>
    <sidenav-item icon="layout-dashboard" label="Overview" target="overview"/>
    <sidenav-item icon="chart-pie"        label="Reports"  target="reports"/>
  </app-sidenav>
  <app-content>
    <page id="overview"><!-- ... --></page>
    <page id="reports"><!-- ... --></page>
  </app-content>
  <app-footer>
    <stack direction="horizontal"><label text="Ready"/></stack>
  </app-footer>
</app-shell>
```

* Six registered types (all `ComponentCategory::Layout`): `app_shell` and its
  region markers `app_sidenav` / `app_content` / `app_footer`, plus the leaves
  `sidenav_item` and `page`.
* **Built-in page switching:** clicking a `<sidenav-item target="X">` sets the
  active page to the `<page id="X">` and highlights the item — no handler needed.
  `on-click` still fires if present (side effects).
* Region markers and leaves render nothing on their own — the `app_shell` arm in
  `render_component` collects them by `component_type` (like `sidenav_bar`), so
  they get no-op standalone arms.

# Slot-region routing

Regions are found by type, not position: the container filters
`component.children` for `app_sidenav`/`app_content`/`app_footer`, then filters
each region's children for its leaf type (`sidenav_item`, `page`). Only the
active page's body is rendered (via `render_children`); the footer's children are
rendered generically. This is the typed
[parent-rendered child components](parent-rendered-child-components.md) pattern.

# Active-page state

The active page's `target` is stored in
`ComponentState::SelectedValue(Arc<Mutex<String>>)` via
`get_or_create_selected_value`, keyed by the shell's id, defaulting to the first
page's id. Unlike `sidenav_bar`'s collapsed flag, it is **not** overwritten from
props each render (that would reset the user's selection); an active value that
matches no page falls back to the first page. Clicks mutate the shared state and
call `cx.notify(entity_id)` — the same mechanism `SidenavBarItem` uses.

# Reuse

`AppShell` reuses `map_icon_name` (`components::icon`), the `SidenavBar` sidenav
column/item styling recipe, the `AppLayout`/`FooterBar` frame shape
(`workspace/`), and theme colors (`sidebar`, `sidebar_border`,
`sidebar_foreground`, `list_hover`, `list_active`, `border`). Example:
`examples/app-shell/`.
