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
  <script src="./scripts" on-load="init_handler" />
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
| `catppuccin` | Soft pastel theme (Latte light / Frappé / Macchiato / Mocha) |
| `gruvbox` | Retro warm palette with high contrast |

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
  <source name="ticker" type="timer" interval="1" />                 <!-- tick every 1 second -->
  <source name="api" type="http" url="https://api.example.com" interval="30" />  <!-- poll every 30 seconds -->
  <source name="live" type="websocket" url="ws://localhost:8080" />
  <source name="events" type="mqtt" url="mqtt://localhost:1883" topic="sensors/#" />
  <source name="cache" type="redis" url="redis://localhost:6379" channel="updates" />
  <source name="msgs" type="nats" url="nats://localhost:4222" subject="data.>" />
  <source name="conf" type="file" path="./data.json" watch="true" />
</data>
```

> **`interval` is in SECONDS, not milliseconds.** `create_source` reads it via
> `Duration::from_secs(...)` (`crates/nemo-data/src/sources/mod.rs`), so
> `interval="1000"` means poll every ~17 minutes — data will appear to never
> load. Use `interval="1"` for a 1-second timer, `interval="30"` for a 30-second
> poll. Only `interval` is honored — the `refresh` attribute seen in some older
> examples is **not** wired to any source and is silently ignored.

## Data Bindings

Connect data source paths to component properties. A source object is delivered
whole to the binding; `transform` (optional) reshapes it before it reaches the
target property.

```xml
<!-- Whole array / nested subtree -> table/chart/list: target="data", NO transform.
     A dot-path in `source` selects the subtree. -->
<table id="agents">
  <binding source="data.agents" target="data" />
</table>
<table id="nodes">
  <binding source="data.cluster.nodes" target="data" />   <!-- nested source path is fine -->
</table>

<!-- Scalar field of a source object -> label/text: name the field in `transform`. -->
<label id="mode">
  <binding source="data.node" target="text" transform="mode" />        <!-- data.node.mode -->
</label>
<label id="temp">
  <binding source="data.sensors" target="text" transform="payload.temperature" />  <!-- nested field -->
</label>
```

### What `transform` actually does

The `transform` string is applied by `apply_transform`
(`crates/nemo-layout/src/binding.rs`) and supports exactly two forms:

1. **Field extraction** — a dot-path with no spaces and no literal `value`
   (e.g. `transform="origin"`, `transform="payload.temp"`) walks into the
   incoming Object and returns that nested field. A missing field passes the
   original value through unchanged.
2. **String templating** — any transform containing the word `value`
   (e.g. `transform="Temperature: value°C"`) stringifies the incoming data and
   substitutes it for `value`, yielding a formatted string.

There is **no** `select:...`, `filter:...`, or other prefixed transform syntax
on bindings, and the `transform` attribute does **not** call Rhai functions —
only the two forms above work. (Rust-level pipeline transforms like `select`
exist in `nemo-data` but are not reachable from the XML `transform` attribute.)

### `bind-<prop>` shorthand

Instead of a `<binding>` child, any `bind-<property>` attribute creates a
one-way binding to that property (`crates/nemo/src/runtime.rs`):

```xml
<text id="raw" content="waiting…" bind-content="data.api" />
<label id="t" bind-text="data.sensors.payload.temperature" />
```
`bind-content`, `bind-text`, `bind-value`, etc. all follow the `bind-<prop>`
pattern; the attribute value is the source path (no transform).

## Event Handlers

Reference Rhai function names:

```xml
<button id="save" label="Save" on-click="handle_save" />
<input id="search" on-change="handle_search_change" />
```

The `<script>` element accepts an `on-load` attribute naming a Rhai function run once at startup (the only lifecycle hook nemo exposes):

```xml
<script src="./scripts" on-load="init_handler" />
```

### Handler signature: every handler takes `(component_id, event_data)`

Nemo invokes **every** XML-referenced handler — `on-click`, `on-change`,
`on-load`, … — with exactly two string arguments, `(component_id, event_data)`.
Rhai resolves functions by name **and arity**, so a zero-parameter handler fails
at runtime with `Function not found: <name>`. Always write:

```rhai
fn init_handler(component_id, event_data) { ... }
```

`on-load` is no exception: it is dispatched as `call_handler(handler, "app", "load")`
(`crates/nemo/src/app.rs`), so `component_id` is `"app"` and `event_data` is
`"load"`.

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
| `max-width/max-height` | integer | — | Maximum dimensions |
| `flex` | float | — | Flex grow factor |
| `align` | string | cross-axis | `start`/`center`/`end`/`stretch` (default: `center` for horizontal, `stretch` for vertical) |
| `justify` | string | main-axis | `start`/`center`/`end`/`between`/`around` |

### dock
```xml
<dock position="center">
  <!-- dockable panels -->
</dock>
```
Layout container with dockable panels. `position` defaults to `"center"`.

### panel
```xml
<panel title="Section" padding="16" border="1" shadow="md" rounded="lg">
  <!-- children -->
</panel>
```

### tabs
```xml
<tabs id="my_tabs" active-tab="0" variant="underline">
  <tab-item id="t1" label="Tab 1"> ...content... </tab-item>
  <tab-item id="t2" label="Tab 2"> ...content... </tab-item>
</tabs>
```
- **Tab pages MUST be `<tab-item id="…" label="…">`.** A `<panel>` (or any other
  element) is valid XML *anywhere*, so a config with `<panel>` tab pages **passes
  `nemo validate`** but the tab bar gets zero pages and the whole component
  renders invisibly. The render dispatch only collects children whose type is
  `tab_item` (`crates/nemo/src/app.rs`).
- `active-tab` — 0-based index of the initially selected tab.
- `variant` — tab style, one of `underline` (default), `pill`, `segmented`,
  `outline`, `tab` (`crates/nemo/src/components/tabs.rs`). Any other value falls
  back to `underline`.

> **Child-only elements.** Several elements are valid *only* inside a specific
> parent and render nothing (with no validation error) if placed elsewhere or if
> the parent's direct children are the wrong type:
> `tab-item` (in `tabs`), `menu-item` (in `dropdown-button`), `option` (in
> `select`), `list-item` (in `list`), `accordion-item` (in `accordion`),
> `sidenav-bar-item` (in `sidenav-bar`), `slot` (in `template`). When a
> container renders blank, check that its direct children are the right type.

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

### dropdown-button
```xml
<dropdown-button id="actions" label="Action" variant="primary">
  <menu-item label="Edit" on-click="handle_edit" />
  <menu-item label="Delete" on-click="handle_delete" />
</dropdown-button>
```
A button that opens a dropdown menu built from `menu-item` children. `label` defaults to `"Action"`.

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
Options may also be supplied declaratively as `option` children (each with `value` and `label`), instead of the `options` JSON attribute:
```xml
<select id="color" value="red" on-change="handle_select">
  <option value="red" label="Red" />
  <option value="green" label="Green" />
</select>
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
A list may also be built from `list-item` children, whose own children are the row content:
```xml
<list id="items">
  <list-item><label text="Row one" /></list-item>
  <list-item><label text="Row two" /></list-item>
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

## Definite-height gotcha (silent 0px collapse)

`table`, `tree`, and `list` render their bodies with a `uniform_list`, which
collapses to **0px** unless an ancestor has a *definite* height — the rows
silently disappear (a table's header may still show, which is misleading). This
also bites content inside a `tabs` region.

Give the scrollable region a real height instead of relying on flex to size it
from content:

```xml
<!-- Preferred: a scrolling stack that grows to fill its parent -->
<stack scroll="true" flex="1">
  <table id="rows"> <binding source="data.rows" target="data" /> </table>
</stack>

<!-- Or set an explicit height on the widget -->
<table id="rows" height="400"> <binding source="data.rows" target="data" /> </table>
```
