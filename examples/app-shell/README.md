# App Shell Example

Demonstrates the `app-shell` **container** — a high-level layout that packages a
standard application frame (left sidenav, switchable content area, full-width
status footer) so you describe intent instead of assembling stacks and wiring
page-toggle handlers.

## Run

```sh
cargo run -- --app-config examples/app-shell/app.nemo
```

## What It Shows

- `<app-shell>` container with three region slots: `<app-sidenav>`, `<app-content>`, `<app-footer>`
- `<sidenav-item icon=".." label=".." target="..">` navigation items
- Built-in **page switching**: clicking a sidenav item shows the matching
  `<page id="...">` and highlights the active item — **no Rhai handler required**
- Only the active page's body is rendered; the footer spans the full width

## Why Containers

Compare this to `examples/data-streaming/`, which builds the same sidebar +
content layout by hand from a horizontal `<stack>`, a `<sidenav-bar>`, and an
`on_nav` Rhai handler that toggles panel visibility. The container collapses all
of that layout plumbing into one declarative element, leaving developer effort
for Rhai scripts and plugins.
