---
description: Scaffold a new Nemo example application with XML config, Rhai handlers, and README
---

# Add Nemo Example

Create a new example application for the Nemo framework with all necessary files.

## Arguments

The user should provide:
- **Example name** (e.g., `dashboard`, `iot-monitor`) — kebab-case
- **Description** — what the example demonstrates
- **Components used** — which Nemo components to showcase
- **Data sources** — optional data source configuration (timer, http, etc.)

If not provided, ask for at least the name and description.

## Steps

### 1. Read existing examples for patterns

Read one or more existing examples to match conventions:
- `examples/basic/app.xml` — minimal setup
- `examples/basic/scripts/handlers.rhai` — basic Rhai handlers
- `examples/basic/README.md` — README format
- `examples/data-binding/app.xml` — data sources and bindings example

### 2. Create directory structure

```
examples/<name>/
  app.xml
  scripts/
    handlers.rhai
  README.md
```

### 3. Create app.xml

```xml
<nemo>
  <app title="<Display Name>">
    <window title="<Display Name>">
      <header-bar github-url="https://github.com/geoffjay/nemo/tree/main/examples/<name>"
                  theme-toggle="true" />
    </window>
    <theme name="kanagawa" mode="dark" />
  </app>

  <!-- Scripts configuration -->
  <script src="./scripts" />

  <!-- Variables (if needed) -->
  <!-- <variable name="key" type="string" default="value" /> -->

  <!-- Data sources (if needed) -->
  <!-- <data>
    <source name="ticker" type="timer" interval="1000" />
  </data> -->

  <!-- Layout -->
  <layout type="stack">
    <!-- Components here -->
  </layout>
</nemo>
```

Fill in the layout section with the components the user specified, using appropriate properties and bindings.

### 4. Create scripts/handlers.rhai

```rhai
// <Example Name> Event Handlers

fn on_button_click(component_id, event_data) {
    log_info("Button clicked: " + component_id);
}
```

Add handler functions for any `on-click` or `on-change` attributes used in the XML.

### 5. Create README.md

```markdown
# <Example Name>

<Description of what this example demonstrates.>

## Running

```bash
cargo run -- --config examples/<name>/app.xml
```

## Features

- <Feature 1>
- <Feature 2>

## Screenshot

<!-- Add screenshot after running: ![Screenshot](screenshot.png) -->
```

### 6. Verify

Run `cargo run -- --config examples/<name>/app.xml` to verify the example works (if the user wants to test).

## Available Components Reference

**Layout:** stack, panel, tabs, dock
**Display:** label, text, icon, image, progress, avatar, badge, tag, spinner, alert, accordion, collapsible
**Input:** button, input, textarea, code_editor, text_editor, checkbox, select, radio, slider, switch, toggle
**Data:** table, tree, list
**Charts:** line-chart, bar-chart, area-chart, pie-chart, column-chart, realtime-chart, scatter-chart, bubble-chart, heatmap-chart, radar-chart, candlestick-chart, pyramid-chart, funnel-chart
**Navigation:** sidenav-bar, sidenav-bar-item

## Notes

- XML element names use kebab-case (e.g., `<line-chart>`)
- Property names use kebab-case in XML, converted to snake_case internally
- Data bindings use `<binding source="path" target="property" />`
- Event handlers reference Rhai function names via `on-click="function_name"`
- Themes available: kanagawa, tokyo-night, nord (with mode: dark/light)
