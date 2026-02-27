---
name: nemo-xml-reference
description: Complete XML configuration reference for Nemo applications including all component types, properties, data sources, expressions, bindings, and templates. Use when writing or debugging Nemo XML config files.
---

# Nemo XML Configuration Reference

Use this skill when writing, modifying, or debugging Nemo XML configuration files.

## Document Structure

```xml
<nemo>
  <app title="App Title">
    <window title="Window Title" width="1200" height="800"
            min-width="400" min-height="300">
      <header-bar github-url="https://..." theme-toggle="true" />
    </window>
    <theme name="kanagawa" mode="dark" />
  </app>

  <variable name="key" type="string" default="value" />
  <script src="./scripts" />
  <template name="card"> ... </template>

  <data>
    <source name="..." type="..." ... />
    <sink name="..." type="..." ... />
  </data>

  <plugin name="..." path="./plugins/..." />

  <layout type="stack">
    <!-- Component tree -->
  </layout>
</nemo>
```

## Themes

| Name | Description |
|------|-------------|
| `kanagawa` | Warm, muted palette inspired by Japanese art |
| `tokyo-night` | Cool, modern dark theme |
| `nord` | Arctic-inspired pastel scheme |

Modes: `dark`, `light`

## Expression Syntax

Expressions use `${...}` in attribute values:

| Pattern | Example | Description |
|---------|---------|-------------|
| `${var.name}` | `${var.api_url}` | Variable reference |
| `${env.KEY}` | `${env.API_TOKEN}` | Environment variable |
| `${upper(expr)}` | `${upper(var.name)}` | Uppercase |
| `${lower(expr)}` | `${lower(var.name)}` | Lowercase |
| `${trim(expr)}` | `${trim(var.input)}` | Trim whitespace |
| `${length(expr)}` | `${length(var.list)}` | Length |
| `${coalesce(a, b)}` | `${coalesce(var.custom, var.default)}` | First non-null |
| `${env(KEY)}` | `${env("HOME")}` | Env var function |
| String interpolation | `Hello, ${var.user}!` | Inline in strings |

## Data Sources

```xml
<data>
  <source name="ticker" type="timer" interval="1000" />
  <source name="api" type="http" url="https://api.example.com" interval="30000" method="GET" />
  <source name="live" type="websocket" url="ws://localhost:8080" />
  <source name="events" type="mqtt" url="mqtt://localhost:1883" topic="sensors/#" />
  <source name="cache" type="redis" url="redis://localhost:6379" channel="updates" />
  <source name="msgs" type="nats" url="nats://localhost:4222" subject="data.>" />
  <source name="conf" type="file" path="./data.json" watch="true" />
</data>
```

## Data Bindings

Connect data source paths to component properties:

```xml
<table id="my_table">
  <binding source="data.api" target="rows" />
  <binding source="data.api" target="data" transform="select:name,value" />
</table>

<label id="temp">
  <binding source="mock.temperature" target="text" />
</label>
```

## Event Handlers

Reference Rhai function names:

```xml
<button id="save" label="Save" on-click="handle_save" />
<input id="search" on-change="handle_search_change" />
```

## Components — Layout

### stack
```xml
<stack direction="vertical" spacing="8" padding="16" scroll="false">
  <!-- children -->
</stack>
```
| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `direction` | string | `"vertical"` | `"vertical"` or `"horizontal"` |
| `spacing` | integer | `0` | Gap between children (px) |
| `padding` | integer | — | Inner padding (px) |
| `scroll` | boolean | `false` | Enable scrolling |
| `width/height` | integer | — | Fixed dimensions |
| `flex` | float | — | Flex grow factor |

### panel
```xml
<panel title="Section" padding="16" border="1" shadow="md" rounded="lg">
  <!-- children -->
</panel>
```

### tabs
```xml
<tabs active-tab="0">
  <panel title="Tab 1"> ... </panel>
  <panel title="Tab 2"> ... </panel>
</tabs>
```

## Components — Display

### label
```xml
<label id="title" text="Hello World" size="lg" />
```
Required: `text`. Sizes: `sm`, `md`, `lg`

### text
```xml
<text id="body" content="Paragraph text here" />
```
Required: `content`

### icon
```xml
<icon id="info" name="info" size="24" />
```
Required: `name`. Uses Lucide icon names.

### image
```xml
<image id="logo" src="https://example.com/image.png" alt="Logo" />
```

### progress
```xml
<progress id="loading" value="75" max="100" />
```

### avatar
```xml
<avatar id="user" name="John Doe" />
```

### badge
```xml
<badge id="count" count="5" />
<badge id="status" dot="true" />
```

### tag
```xml
<tag id="status" label="Active" variant="success" outline="true" />
```
Variants: `primary`, `secondary`, `danger`, `warning`, `success`, `info`

### spinner
```xml
<spinner id="loading" size="lg" />
```
Sizes: `sm`, `md`, `lg`

### alert
```xml
<alert id="warning" message="Disk space low" title="Warning" variant="warning" />
```
Required: `message`. Variants: `info`, `warning`, `danger`, `success`

### accordion
```xml
<accordion id="faq" items='[{"title":"Q1","content":"A1"}]' multiple="false" bordered="true" />
```

### collapsible
```xml
<collapsible id="details" title="More Info" open="false">
  <!-- children shown when expanded -->
</collapsible>
```

## Components — Input

### button
```xml
<button id="submit" label="Save" variant="primary" size="md" on-click="handle_save" disabled="false" />
```
Required: `label`. Variants: `primary`, `secondary`, `danger`, `ghost`, `warning`, `success`, `info`, `outline`, `link`

### input
```xml
<input id="name" placeholder="Enter name" value="" on-change="handle_change" />
```

### textarea
```xml
<textarea id="notes" placeholder="Enter notes" rows="4" auto-grow-min="2" auto-grow-max="10" />
```

### code-editor
```xml
<code-editor id="code" language="rust" line-number="true" searchable="true" tab-size="4" rows="10" />
```

### text-editor
```xml
<text-editor id="content" placeholder="Write here..." rows="6" />
```

### checkbox
```xml
<checkbox id="agree" label="I agree" checked="false" on-change="handle_toggle" />
```

### select
```xml
<select id="color" options='["Red","Green","Blue"]' value="Red" on-change="handle_select" />
```

### radio
```xml
<radio id="size" options='["Small","Medium","Large"]' value="Medium" direction="horizontal" />
```

### slider
```xml
<slider id="volume" min="0" max="100" step="1" value="50" />
```

### switch / toggle
```xml
<switch id="dark_mode" label="Dark Mode" checked="false" />
<toggle id="bold" label="Bold" icon="bold" />
```

## Components — Data

### table
```xml
<table id="users" height="400" stripe="true" bordered="true" on-click="handle_row_click">
  <binding source="data.api" target="data" />
</table>
```
IMPORTANT: Must have a parent with definite height (the `height` property sets this).

### tree
```xml
<tree id="files" height="300">
  <binding source="data.filesystem" target="items" />
</tree>
```

### list
```xml
<list id="items">
  <binding source="data.items" target="items" />
</list>
```

## Components — Charts

All charts accept `height` (default 300), `width`, and `data` (via binding or inline).

### line-chart
```xml
<line-chart id="trend" x-field="date" y-field="value" dot="true" linear="false" height="300">
  <binding source="data.timeseries" target="data" />
</line-chart>
```
Required: `x-field`, `y-field`

### realtime-chart
```xml
<realtime-chart id="live" x-field="timestamp" y-fields='["temp","humidity"]' height="400">
  <binding source="data.stream" target="data" />
</realtime-chart>
```
Required: `x-field`

### bar-chart / column-chart
```xml
<bar-chart id="sales" x-field="month" y-field="revenue" show-label="true" />
```
Required: `x-field`, `y-field`

### area-chart
```xml
<area-chart id="stacked" x-field="date" y-fields='["a","b","c"]' fill-opacity="0.3" />
```
Required: `x-field`, `y-fields`

### pie-chart
```xml
<pie-chart id="dist" value-field="count" inner-radius="40" outer-radius="100" />
```
Required: `value-field`

### scatter-chart / bubble-chart
```xml
<scatter-chart id="plot" x-field="x" y-field="y" dot-size="4" />
<bubble-chart id="bubbles" x-field="x" y-field="y" size-field="pop" min-radius="3" max-radius="30" />
```

### heatmap-chart
```xml
<heatmap-chart id="heat" x-field="hour" y-field="day" value-field="count" />
```
Required: `x-field`, `y-field`, `value-field`

### radar-chart
```xml
<radar-chart id="skills" categories='["A","B","C","D","E"]' y-fields='["score1","score2"]' max-value="100" />
```
Required: `categories`, `y-fields`

### candlestick-chart
```xml
<candlestick-chart id="stock" x-field="date" open-field="open" high-field="high" low-field="low" close-field="close" />
```
Required: `x-field`, `open-field`, `high-field`, `low-field`, `close-field`

### stacked/clustered variants
```xml
<stacked-column-chart x-field="month" y-fields='["a","b"]' />
<clustered-column-chart x-field="month" y-fields='["a","b"]' />
<stacked-bar-chart y-field="category" x-fields='["q1","q2"]' />
<clustered-bar-chart y-field="category" x-fields='["q1","q2"]' />
```

### pyramid-chart / funnel-chart
```xml
<pyramid-chart label-field="stage" value-field="count" />
<funnel-chart label-field="step" value-field="users" />
```
Required: `label-field`, `value-field`

## Components — Navigation

### sidenav-bar
```xml
<sidenav-bar id="nav" collapsed="false" width="200">
  <sidenav-bar-item icon="home" label="Home" on-click="nav_home" />
  <sidenav-bar-item icon="settings" label="Settings" on-click="nav_settings" />
</sidenav-bar>
```

## Components — Feedback

### modal
```xml
<modal id="confirm" title="Confirm Action" open="false">
  <label text="Are you sure?" />
  <button label="Yes" on-click="confirm_action" />
</modal>
```

### notification
```xml
<notification id="toast" message="Saved successfully" kind="success" />
```
Required: `message`. Kinds: `info`, `warning`, `danger`, `success`

### tooltip
```xml
<tooltip id="help" content="Click to save">
  <button label="Save" />
</tooltip>
```
Required: `content`

## Common Layout Properties

These work on most components via `apply_layout_styles()`:

| Property | Type | Description |
|----------|------|-------------|
| `width` / `height` | integer | Fixed size in px |
| `min-width` / `min-height` | integer | Minimum size |
| `flex` | float | Flex grow factor |
| `padding` | integer | All-side padding |
| `padding-x` / `padding-y` | integer | Horizontal/vertical padding |
| `margin` | integer | All-side margin |
| `margin-x` / `margin-y` | integer | Horizontal/vertical margin |
| `border` | integer | Border width |
| `border-color` | string | Border color (theme ref or hex) |
| `background` / `background-color` | string | Background color |
| `shadow` | string | Shadow preset: sm, md, lg, xl, 2xl |
| `rounded` | string | Corner radius: sm, md, lg, xl, full |
| `visible` | boolean | Show/hide the component |
