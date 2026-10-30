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

# Collapsing the sidenav

`app-shell` honours a `collapsed` bool prop on every render (icon-only sidenav at
48px). Config fixes it at launch; to toggle at runtime, flip the property from a
script — the App re-renders because `set_component_property` wakes the UI poll
loop:

```xml
<app-shell id="shell" sidenav-width="220">
  ...
</app-shell>
```

```rhai
fn on_toggle_nav(component_id, event_data) {
  let current = get_component_property("shell", "collapsed");
  set_component_property("shell", "collapsed", current != true);
}
```

A common trigger is a `<header-bar>` `<header-button>` (icon in the app chrome,
left of the menu) — see `examples/components/` (`on_toggle_nav`).

# Slot-region routing

Regions are found by type, not position: the container filters
`component.children` for `app_sidenav`/`app_content`/`app_footer`. The sidenav
renders its `<sidenav-item>`s with the shell's own icon/label/active styling and
renders **any other children generically** (so `<nav-link>`s and `<label>`s work
there too). The content region uses built-in page switching **only when it has
`<page>` children** — the active page's body is rendered via `render_children`;
otherwise it renders its children as-is. The footer's children are always
rendered generically. This is the typed
[parent-rendered child components](parent-rendered-child-components.md) pattern.

# Composing with a router

Because the sidenav renders arbitrary children and the content region renders
its children as-is when there are no `<page>`s, `app-shell` composes with the
[router](routing.md): put a `<router>` in `<app-content>` and drive it with
`<nav-link>`s in `<app-sidenav>` to get URL-style routes/params/history instead
of built-in page switching. `examples/complete/` uses exactly this shape.

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
