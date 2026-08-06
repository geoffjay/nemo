---
name: nemo-xml-reference
description: Complete XML configuration reference for Nemo applications including all component types, properties, data sources, expressions, bindings, and templates. Use when writing or debugging Nemo XML config files.
---

# Nemo XML Configuration Reference

Use this skill when writing, modifying, or debugging Nemo XML configuration files.

## Document Structure

```xml
<nemo>
  <themes>                                   <!-- optional: register project theme files -->
    <theme-set src="themes/aurora.json" />
  </themes>

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

  <imports>                                  <!-- optional: single-file components (.nemo) -->
    <import src="./components/card.nemo" />
  </imports>
  <components dir="./components" />           <!-- or auto-discover every *.nemo in a dir -->

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

Top-level blocks are order-tolerant. `<themes>`, `<imports>`/`<components>`, and
routing primitives are documented in their own sections below.

## Header bar (`<header-bar>`)

The title-bar chrome at the top of the window. All contents are opt-in:

```xml
<header-bar github-url="https://github.com/you/app" theme-toggle="true">
  <menu-item label="Preferences" icon="settings" on-click="open_prefs" />
  <menu-item label="Documentation" icon="book" on-click="open_docs" />
  <menu-item separator="true" />
  <menu-item label="About" icon="info" on-click="show_about" />
</header-bar>
```

| Attribute / child | Description |
|-------------------|-------------|
| `github-url` | Optional external link shown as an icon on the right. |
| `theme-toggle` | `true` shows the light/dark toggle icon on the right. |
| `<menu-item>` children | **Opt-in** dropdown menu. When any `<menu-item>` is present, a hamburger icon appears on the **far left** (before the title) and opens a native dropdown built from these items. |

`<menu-item>` attributes: `label` (entry text), `icon` (optional Lucide icon
name), `on-click` (handler name), `separator="true"` (renders a divider instead
of a clickable entry — no `label`/`on-click` needed). Handlers receive the
standard `(component_id, event_data)`; `component_id` is `"header-bar"` and
`event_data` is `"click"`.

## Themes

`<theme name="…" mode="…">` under `<app>` selects a **theme set**. The seven
built-in sets (match on the set name, case-insensitive):

| `name` | Description |
|--------|-------------|
| `kanagawa` | Warm, muted palette inspired by Japanese art |
| `kanagawa-dragon` | Darker Kanagawa variant |
| `tokyo-night` | Cool, modern dark theme |
| `nord` | Arctic-inspired pastel scheme |
| `catppuccin` | Soft pastel theme |
| `catppuccin-macchiato` | Warmer Catppuccin variant |
| `gruvbox` | Retro warm palette with high contrast |

Modes: `dark`, `light` (each set supplies the variants it defines).

### Project-defined themes

Ship your own theme sets and register them with a top-level `<themes>` block.
Each `<theme-set src="…">` points at a JSON file using the same schema as the
built-ins (copy a shipped `crates/nemo/src/theme/*.json` and edit its colors).
Registered sets are then selectable by name and appear in the settings picker:

```xml
<themes>
  <theme-set src="themes/aurora.json" />
</themes>
<app>
  <theme name="aurora" mode="dark" />
</app>
```

### Color overrides (`<extend>`)

Override individual colors on top of the selected theme (overrides win over the
base). Keys are dotted color roles; values are hex or a `theme.*` reference:

```xml
<theme name="aurora" mode="dark">
  <extend>
    <color key="primary.background" value="#ff7a45" />
    <color key="primary.hover.background" value="#ff9466" />
  </extend>
</theme>
```

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
  <source name="secure" type="http" url="https://api.example.com/me"
          headers='{"Authorization":"Bearer ${env.API_TOKEN}"}' />  <!-- headers: object or JSON string; ${env.X}/${var.x} resolved at load -->
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

The `<script>` element accepts an `on-load` attribute naming a Rhai function run once at startup:

```xml
<script src="./scripts" on-load="init_handler" />
```

`on-load` is the app-level lifecycle hook; `<route>` elements additionally expose
per-route `on-enter` / `on-leave` hooks (see **Routing** below). All are invoked
with the same `(component_id, event_data)` signature.

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

### Runtime component creation (Rhai)

Handlers can create and remove built-in component instances at runtime — no
need to pre-declare every possible component and toggle `visible`. Four Rhai
functions are available:

```rhai
create_component(parent_id, type, props)            // returns generated __dyn_N id
create_component_with_id(parent_id, id, type, props)
update_component(id, props)                          // bulk property set
remove_component(id)                                  // recursive subtree + binding teardown
```

`props` is a Rhai map (`#{text: "Hi", label: "Go"}`). A `"handlers"` sub-map
(`#{handlers: #{click: "on_click"}}`) is extracted as event handlers. No
`<binding>` support — a dynamic component that needs reactive data must use
an explicit handler calling `set_component_property`, or set props at creation
time. `remove_component` refuses to remove the root. See
[runtime component creation](../../docs/knowledgebase/patterns/runtime-component-creation.md).

## Single-File Components (`.nemo` SFCs)

Package a reusable piece of UI into one `.nemo` file (template + optional scoped
script/style), then import it and use it as a custom tag — Vue-like. An SFC is
expanded onto the template machinery, so by render time it is ordinary built-in
components.

### Importing

```xml
<imports>
  <import src="./components/card.nemo" />            <!-- tag from <template name> or filename -->
  <import src="./components/button.nemo" as="my-btn" />  <!-- as= overrides the tag -->
  <import src="github.com/user/nemo-widgets" as="nw" />  <!-- remote library (see Build below) -->
</imports>

<components dir="./components" />                     <!-- OR auto-discover every *.nemo in a dir -->
```

The tag is resolved `as=` > `<template name>` > filename stem, then kebab→snake
normalized (write `<labeled-button>`; it matches internally as `labeled_button`).
A remote `src` (a `github.com/…` module path) resolves against the fetched
package cache and brings in **all** the package's exported tags; `as=` becomes a
tag-name prefix.

### Authoring a `.nemo` file

A `.nemo` file is **not** wrapped in `<nemo>`. Its top-level children:

* `<props>` — optional. Declares typed props with defaults; instance attributes
  override them.
* `<template>` — **required, exactly one root element**. Use `${prop}`
  placeholders for interpolation and `<slot />` for consumer content.
* `<style>` — optional scoped CSS subset (folded onto template nodes at compile
  time; there is no runtime cascade).
* `<script>` — optional Rhai, loaded under the id `sfc:<tag>`.

```xml
<!-- components/labeled-button.nemo -->
<props>
  <prop name="label" type="string" default="Button" />
  <prop name="variant" type="string" default="primary" />
</props>
<template name="labeled-button">
  <button label="${label}" variant="${variant}" on-click="handleClick" />
</template>
<script><![CDATA[
fn handleClick(component_id, event_data) {
    set_component_property(component_id, "label", "Clicked!");
}
]]></script>
```

Used in `app.xml`:

```xml
<labeled-button label="Save" />        <!-- ${label} → "Save"; variant → default "primary" -->
```

### Slots

The default `<slot />` receives consumer children; a named `<slot name="header"
required="true" multiple="false" />` receives children tagged `slot="header"`.
Slots inject **component children, not raw text** — provide text via a child
component (`<label text="…"/>`) or a `${prop}`.

```xml
<!-- card.nemo template -->
<template name="card">
  <panel>
    <stack id="head"><slot name="header" required="true" /></stack>
    <stack id="body"><slot /></stack>
  </panel>
</template>
```
```xml
<!-- usage -->
<card>
  <label slot="header" text="Title" />   <!-- → header slot -->
  <text content="Body" />                 <!-- → default slot -->
</card>
```

### Scoped `<style>`

A CSS subset folded onto matching template nodes at compile time, scoped to the
SFC's own subtree. v1 selectors: **type** (`panel { … }` matches nodes with that
`type`) and **id** (`#head { … }`). No class/combinator/pseudo/media.
Declarations are limited to the universal style attributes below; CSS names
normalize to nemo's (`border-radius`→`rounded`, `background-color`→`background`),
sizes drop `px` (`20px`→`20`), colors stay strings (incl. `theme.*`).

```xml
<style><![CDATA[
  panel { padding: 20px; border-radius: lg; }
  #head { padding-bottom: 8px; }
]]></style>
```

### Rules that bite

* **CDATA for `<`/`&`.** `.nemo` is parsed as XML, which has no raw-text
  elements, so a `<script>`/`<style>` body containing `<` or `&` (Rhai `&&`,
  generics, a CSS `>` combinator) **must** be wrapped in `<![CDATA[ … ]]>`. The
  body must also be one contiguous block.
* **Handler scoping.** A **template-authored** bare `on-click="fn"` routes to the
  SFC's own `<script>` (rewritten to `sfc:<tag>::fn`). An **instance** handler
  (`<labeled-button on-click="globalFn"/>`) stays bare and routes to the global
  `handlers` script. One SFC script serves every instance; the handler receives
  the per-instance scoped `component_id`, so it mutates only the instance that
  fired.
* **Id scoping is automatic.** Template-owned child ids are prefixed with the
  instance id (`body` → `<instance>_body`), so multiple instances never collide.
  Slot-injected children keep their own ids.
* **Two `${}` systems.** SFC props use bare `${label}` (runtime interpolation,
  string-only), distinct from load-time `${var.x}`/`${env.x}`. Data still flows
  through `bind-*`/`<binding>`.

## Routing (`<router>` / `<route>` / `<nav-link>`)

A chrome-free page-routing primitive. A `<router>` mounts only the matching
route's body; navigation is either declarative (`<nav-link>`) or scripted
(`navigate()` in Rhai).

```xml
<!-- Declarative nav bar: no handler needed. `router` is optional if the target
     router has primary="true". A link highlights when its route is active. -->
<stack direction="horizontal">
  <nav-link router="main" route="/home" label="Home" />
  <nav-link router="main" route="/users/42" label="User 42" />
  <nav-link router="main" route="/settings" label="Settings" />
</stack>

<router id="main" default="/home" primary="true" flex="1">
  <route path="/home">
    <label text="Home" size="xl" />
  </route>

  <!-- :param capture → data.route.<router-id>.params.<name> -->
  <route id="user_route" path="/users/:id" on-enter="on_user_enter" on-leave="on_user_leave">
    <label bind-text="data.route.main.params.id" />
  </route>

  <!-- A route may host its own nested <router>. -->
  <route path="/settings"> … </route>

  <!-- Trailing "*" is the not-found fallback (routes match in document order). -->
  <route path="*"><label text="Not found" /></route>
</router>
```

| Element | Key attributes |
|---------|----------------|
| `router` | `id`, `default` (initial path), `primary` (target of `router`-less nav-links / bare `navigate()`), plus layout props (`flex`, height, `scroll`) |
| `route` | `path` (`/x`, `/x/:param`, or `*`), optional `id`, `on-enter`, `on-leave` |
| `nav-link` | `route` (required), `router` (optional if a primary router exists), `label` |

* **Params** are projected to `data.route.<router-id>.path` and
  `data.route.<router-id>.params.<name>` — read them with `bind-text` /
  `<binding>` (e.g. `bind-text="data.route.main.params.id"`).
* **Rhai API:** `navigate("/path")` (primary router) / `navigate("router-id",
  "/path")` (explicit); `back()` / `forward()` (optionally `back("router-id")`).
* **Launch flag:** `--route <path>` (or `--route <router-id>=<path>`) starts a
  router at a specific route instead of its `default`, for this launch only.
* **Sizing:** the router renders content-sized (not `size_full`). For a tall,
  scrolling page, wrap the router in a `scroll` stack and leave the router itself
  content-sized.

> **Gotcha — router inside an SFC.** A `<router>` nested inside an SFC
> `<template>` has its `id` scoped per instance (`main` → `<instance>_main`), but
> `<nav-link router="main">` / `navigate("main", …)` targets are **not yet**
> rewritten to the scoped id — navigation to such a router silently fails. Until
> that follow-up lands, keep routers at the top level, not inside a reused SFC.

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

## Project Manifest & Build (`nemo.toml`, `nemo build`, `nemo get`)

A nemo *project* is a directory with a `nemo.toml` manifest at its root. Any
`nemo`/`nemo dev`/`nemo build` invocation walks up from the target to find that
root, so relative paths (entry, `<import src>`, `dir=`, theme `src`) resolve
against it consistently.

```toml
# nemo.toml — an application project
name = "my-app"
entry = "app.xml"          # the config the app launches

[build]
out = "dist"               # build output directory
load = "source"            # "source" (default) parses .nemo at launch;
                           # "dist" loads pre-compiled artifacts from out/
```

```toml
# nemo.toml — a reusable component library (published for others to import)
name = "nemo-widgets"

[package]
exports = ["components/*.nemo"]   # which .nemo files this package exposes
```

**`nemo build`** compiles `.nemo` components to JSON artifacts:

* `nemo build path/to/card.nemo` — compile one component.
* `nemo build` in a `[package]` project — compile the whole exported library.
* `nemo build` in an app project — build the app to a loadable `out/` (`dist/`).
  Launch it with `nemo --dist` (or set `load = "dist"` in the manifest) to load
  the pre-compiled artifacts instead of re-parsing `.nemo` at startup.

**Remote component libraries** (Go-style): import a library by module path and
fetch it with `nemo get`.

```xml
<imports>
  <import src="github.com/user/nemo-widgets" as="nw" />
</imports>
```
```sh
nemo get                    # fetch every module <import> into .nemo/packages/,
                            # pinning resolved versions in nemo.lock
```

`nemo get` clones each `github.com/…` module (git) into `.nemo/packages/`,
records exact versions in `nemo.lock`, and module `<import>`s then resolve
against that cache. Commit `nemo.lock`; the `.nemo/packages/` cache is generated
(gitignore it).
