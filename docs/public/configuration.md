# Configuration Reference

Nemo applications are defined in XML files. This page is a complete reference for every element, attribute, and component type.

## File Structure

Nemo applications are defined in `app.nemo` single-file components (SFCs). A
file contains any of the top-level blocks below — a minimal app is just an
`<app>` block and a `<template name="app">` UI tree:

```xml
<app title="My Application">
  <window title="Window Title" width="1024" height="768">
    <header-bar github-url="https://github.com/user/repo" theme-toggle="true" />
  </window>
  <theme name="kanagawa" mode="dark" />
</app>

<variable name="api_base" type="string" default="https://api.example.com" />
<script src="./scripts" />

<data>...</data>                    <!-- Data source and sink definitions -->

<template name="app">
  <!-- UI component tree -->
</template>
```

All blocks are optional. An `<include src="..." />` directive can appear
anywhere a top-level block does; its target file is parsed and merged in place,
letting you split a large configuration across multiple files.

---

## `variable` Element

Define reusable variables with type and default value. Variables are referenced elsewhere via `${var.name}`.

```xml
<variable name="button_height" type="int" default="48" />
<variable name="api_base" type="string" default="https://api.example.com" />
```

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | No | Variable type: `"string"`, `"int"`, `"float"`, `"bool"` |
| `default` | any | No | Default value |

Use variables in any attribute value:

```xml
min-height="${var.button_height}"
url="${var.api_base}/users"
```

---

## `app` Element

Application metadata, window settings, and theme configuration.

```xml
<app title="My Application">
  <window title="Window Title" width="1024" height="768" min-width="320" min-height="240">
    <header-bar github-url="https://github.com/user/repo" theme-toggle="true" />
  </window>

  <theme name="kanagawa" mode="dark">
    <extend name="custom" primary="#FF6600" background="#1A1A2E" />
  </theme>
</app>
```

### `app.window`

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `title` | string | `"Nemo Application"` | Window title text |
| `width` | int | (maximized) | Window width in pixels. Omit for maximized. |
| `height` | int | (maximized) | Window height in pixels. Omit for maximized. |
| `min_width` | int | | Minimum window width |
| `min_height` | int | | Minimum window height |

### `app.window.header_bar`

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `github_url` | string | | URL for the GitHub link in the header |
| `theme_toggle` | bool | `false` | Show light/dark mode toggle button |

### `app.theme`

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `name` | string | | Theme name (see [Available Themes](#available-themes)) |
| `mode` | string | `"dark"` | `"dark"` or `"light"` |

#### Available Themes

| Theme Name | Mode | Description |
|------------|------|-------------|
| `ayu` | light | Ayu Light |
| `ayu-mirage` | dark | Ayu Mirage |
| `kanagawa` | dark | Kanagawa Wave |
| `kanagawa-dragon` | dark | Kanagawa Dragon |
| `catppuccin` | light | Catppuccin Latte |
| `catppuccin-macchiato` | dark | Catppuccin Macchiato |
| `tokyo-night` | dark | Tokyo Night |
| `gruvbox` | dark | Gruvbox |
| `nord` | dark | Nord |

Theme resolution is case-insensitive. If a theme set has multiple variants, the
variant matching the requested mode is selected.

#### Color overrides (`extend`)

The optional `<extend>` sub-block overrides individual theme colors on top of
the selected theme. Each `<color>` child carries a dotted theme key and a
hex value:

```xml
<theme name="aurora" mode="dark">
  <extend>
    <color key="primary.background" value="#ff7a45" />
    <color key="primary.hover.background" value="#ff9466" />
  </extend>
</theme>
```

Overrides win over the base theme's colors.

#### Project themes (`themes`)

Apps can register whole theme sets from JSON files that use the same schema as
the shipped themes (the easiest way to author one is to copy
`crates/nemo/src/theme/*.json` and edit its colors):

```xml
<themes>
  <theme-set src="themes/aurora.json" />
</themes>
```

Project theme sets are consulted **before** the built-in themes, so a project
can shadow a built-in name. See `examples/custom-theme` for a complete
example.

---

## `script` Element

Configure where RHAI scripts are loaded from and which optional features they
may use.

```xml
<script src="./scripts" features="file-io" on-load="on_load" />
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `src` | string | Directory containing `.rhai` script files |
| `features` | string | Comma-separated list of opt-in script features (default: none) |
| `on_load` | string | RHAI function to run once after the layout is built — where an app hydrates its UI from persisted state so the first paint already reflects it |

### Script features

By default scripts are sandboxed — no filesystem, environment, or process
access. The `features` attribute opts in to host-access packages:

| Feature | Package | What it enables |
|---------|---------|-----------------|
| `file-io` | [`rhai-fs`](https://crates.io/crates/rhai-fs) | File read/write (`open_file`, `read_string`, `write`, `exists`, `create_dir`, `cwd`, `path`, …) |
| `system` | [`rhai-env`](https://crates.io/crates/rhai-env), [`rhai-process`](https://crates.io/crates/rhai-process) | Environment variables (`env`, `envs`, `set_env`) and subprocess execution (`cmd([...]).pipe(...).build().run()`) |
| `science` | [`rhai-sci`](https://crates.io/crates/rhai-sci) | Scientific computing (`mean`, `std`, `median`, `linspace`, matrix ops, regression, SVD, …) |
| `network` | _(reserved)_ | HTTP is already available via built-in `http_get`/`http_post`/`http_put`/`http_delete`, each with an optional trailing `headers` map (e.g. `http_get(url, #{ "Authorization": "Bearer " + token })`) |

The [`rhai-chrono`](https://crates.io/crates/rhai-chrono) package (date/time
arithmetic: `datetime_utc`, `datetime_parse`, `format`, `timedelta_days`, …)
and the `json_parse` / `json_stringify` helpers are **always** available —
they are pure and touch no host state.

> **Security:** `file-io` grants full filesystem access with the permissions
> of the host process (no sandbox root confinement). `system` grants
> environment-variable access and subprocess spawning — the most dangerous
> capability. Only enable these for apps whose scripts you trust.

Alternatively, a script can be defined inline with a CDATA block instead of `src`:

```xml
<script><![CDATA[
  fn on_click() { print("clicked"); }
]]></script>
```

---

## `templates` Element

Define reusable component templates that can be referenced by components in the layout. Templates reduce duplication when many components share the same base configuration.

### Defining Templates

```xml
<templates>
  <template name="nav_item">
    <button variant="ghost" size="sm" on-click="on_nav" />
  </template>

  <template name="content_page">
    <panel visible="false">
      <stack id="inner" direction="vertical" spacing="12" padding="32" slot="true" />
    </panel>
  </template>
</templates>
```

Each `template` block defines a named set of component properties. Templates can include nested `component` children and all standard component properties.

### Using Templates

Reference a template from any component in the layout with the `template` property:

```xml
<layout type="stack">
  <panel id="nav_home" template="nav_item" label="Home" />
  <panel id="nav_settings" template="nav_item" label="Settings" />
</layout>
```

The component inherits all properties from the template. Properties set on the instance override the template's defaults (e.g., `label` above overrides anything the template set).

### Slots

Templates can designate a child component as a **slot** by setting `slot = true`. When an instance provides children, they are injected into the slot rather than appended at the top level:

```xml
<templates>
  <template name="card">
    <panel padding="16">
      <stack id="body" direction="vertical" slot="true" />
    </panel>
  </template>
</templates>

<layout type="stack">
  <panel id="user_card" template="card">
    <label id="title" text="User Profile" />
  </panel>
</layout>
```

Here, the `title` label is injected inside the `body` stack, not directly under the `panel`.

### Template Features

- **Property override:** Instance properties take precedence over template defaults
- **Child scoping:** Template-originated child IDs are automatically prefixed with the instance ID to prevent collisions
- **Recursive templates:** A template can reference another template
- **Circular reference detection:** Nemo detects and reports circular template chains
- **Handler preservation:** Event handlers (e.g., `on_click`) from templates are preserved through expansion

---

## `data` Element

Define data sources (inputs) and sinks (outputs).

```xml
<data>
  <source name="name" type="...">
    <!-- source-specific attributes -->
  </source>

  <sink name="name" type="...">
    <!-- sink-specific attributes -->
  </sink>
</data>
```

### Data Source Types

#### `timer`

Emits an incrementing tick count at a fixed interval.

```xml
<source name="ticker" type="timer" interval="1" />
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `interval` | int | | Seconds between ticks |

**Emits:** `{ tick: <int>, timestamp: <string> }`

#### `http`

Polls an HTTP endpoint at a regular interval.

```xml
<source name="api" type="http" url="https://httpbin.org/get" interval="30" method="GET" timeout="30" />
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | string | (required) | The URL to fetch |
| `interval` | int | | Polling interval in seconds. Omit for one-shot. |
| `method` | string | `"GET"` | HTTP method: `GET`, `POST`, `PUT`, `PATCH`, `DELETE` |
| `timeout` | int | 30 | Request timeout in seconds |
| `body` | string | | Request body (for POST/PUT/PATCH) |
| `headers` | object | | Custom request headers |

**Emits:** Parsed JSON response body.

#### `websocket`

Maintains a persistent WebSocket connection with automatic reconnection.

```xml
<source name="stream" type="websocket" url="wss://api.example.com/stream" />
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | WebSocket URL (`ws://` or `wss://`) |

**Emits:** Each received message as a parsed JSON value.

#### `mqtt`

Subscribes to MQTT topics.

```xml
<source name="sensors" type="mqtt" host="localhost" port="1883" topics='["sensors/#", "alerts/high"]' client-id="nemo-app" />
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `host` | string | (required) | MQTT broker host |
| `port` | int | `1883` | MQTT broker port |
| `topics` | array | (required) | List of topic patterns to subscribe |
| `client_id` | string | | MQTT client identifier |
| `qos` | int | `0` | Quality of service level (0, 1, or 2) |

**Emits:** `{ topic: <string>, payload: <string|object> }`

#### `redis`

Subscribes to Redis pub/sub channels.

```xml
<source name="events" type="redis" url="redis://127.0.0.1:6379" channels='["app-events", "notifications"]' />
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | Redis connection URL |
| `channels` | array | List of channels to subscribe |

**Emits:** `{ channel: <string>, payload: <string|object> }`

#### `nats`

Subscribes to NATS subjects.

```xml
<source name="messages" type="nats" url="nats://127.0.0.1:4222" subjects='["updates.>", "commands.*"]' />
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | NATS server URL |
| `subjects` | array | List of subjects (supports NATS wildcards) |

**Emits:** `{ subject: <string>, payload: <string|object> }`

#### `file`

Reads from the filesystem, optionally watching for changes.

```xml
<source name="config_file" type="file" path="/path/to/data.json" watch="true" format="json" />
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | (required) | File path |
| `watch` | bool | `false` | Watch for changes |
| `format` | string | `"raw"` | Parse format: `"raw"`, `"json"`, `"lines"` |

#### `http-server`

Runs a lightweight HTTP/1.1 server inside the data-source machinery. Each
inbound request becomes a data update at `data.<name>`, and the app's script
responds via the `on-request` handler:

```xml
<source name="rest" type="http-server" port="8420" on-request="on_request" />
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `host` | string | `"127.0.0.1"` | Bind address |
| `port` | int | `8080` | Bind port |
| `on_request` | string | (required) | Rhai handler invoked per request |
| `timeout` | int | | Request timeout; unanswered requests get a 408 |

The handler receives the request object (method, path, headers, body, request
`id`) and answers with the built-in `send_response` mechanism — see
`examples/task-list` for a complete REST API on app data. Limitations by
design: HTTP/1.1 only (no TLS, no HTTP/2), Content-Length bodies only, and
handlers run on the UI thread so keep them light.

### Data Sink Types

Sinks are destinations for publishing data from scripts.

#### MQTT Sink

```xml
<sink name="commands" type="mqtt" host="localhost" port="1883" topic="commands" />
```

#### Redis Sink

```xml
<sink name="outbound" type="redis" url="redis://127.0.0.1:6379" channel="app-commands" />
```

#### NATS Sink

```xml
<sink name="nats_out" type="nats" url="nats://127.0.0.1:4222" subject="commands.>" />
```

---

## Layout (`template`)

In an `app.nemo` SFC the UI tree lives in the `<template name="app">` block
(the `<layout>` wrapper still works inside `<include>` fragments):

```xml
<template name="app">
  <component_type id="id">
    <!-- properties as attributes -->
    <label id="child_id" text="Nested child" />
  </component_type>
</template>
```

Named templates beyond `app` define reusable components (see
[`templates` Element](#templates-element)); a template's children are its body.

---

## Components

### Common Properties

These properties are available on all components:

| Property | Type | Description |
|----------|------|-------------|
| `id` | string | Component identifier for script access. Defaults to the component block label. |
| `visible` | bool | Show or hide the component. Default: `true`. |
| `flex` | int/bool | Grow to fill the main axis when truthy (`"1"`/`"true"`) |
| `scroll` | bool | Scroll along the main axis (stacks); also grows |
| `width` | int | Fixed width in pixels |
| `height` | int | Fixed height in pixels |
| `min_width` | int | Minimum width in pixels |
| `min_height` | int | Minimum height in pixels |
| `max_width` | int | Maximum width in pixels |
| `max_height` | int | Maximum height in pixels |

#### Margin

| Property | Type | Description |
|----------|------|-------------|
| `margin` | int | Outer spacing on all sides in pixels |
| `margin_x` | int | Outer spacing on left and right |
| `margin_y` | int | Outer spacing on top and bottom |
| `margin_left` | int | Outer spacing on left side |
| `margin_right` | int | Outer spacing on right side |
| `margin_top` | int | Outer spacing on top |
| `margin_bottom` | int | Outer spacing on bottom |

#### Padding

| Property | Type | Description |
|----------|------|-------------|
| `padding` | int | Inner spacing on all sides in pixels |
| `padding_x` | int | Inner spacing on left and right |
| `padding_y` | int | Inner spacing on top and bottom |
| `padding_left` | int | Inner spacing on left side |
| `padding_right` | int | Inner spacing on right side |
| `padding_top` | int | Inner spacing on top |
| `padding_bottom` | int | Inner spacing on bottom |

#### Border

| Property | Type | Description |
|----------|------|-------------|
| `border` | int | Border width on all sides in pixels |
| `border_x` | int | Border width on left and right |
| `border_y` | int | Border width on top and bottom |
| `border_left` | int | Border width on left side |
| `border_right` | int | Border width on right side |
| `border_top` | int | Border width on top |
| `border_bottom` | int | Border width on bottom |
| `border_color` | string | Border color (theme reference or hex). Default: `theme.border` |

#### Decoration

| Property | Type | Description |
|----------|------|-------------|
| `shadow` | string | Shadow preset: `"sm"`, `"md"`, `"lg"`, `"xl"`, `"2xl"` |
| `rounded` | string | Corner rounding: `"sm"`, `"md"`, `"lg"`, `"xl"`, `"full"` |

Directional properties override their generic counterpart. For example, `margin_left = 8` takes effect alongside `margin = 16` for the left side only, with the other three sides using `16`.

### `dock`

A dockable layout container that hosts panels along an edge or in the center.

```xml
<dock id="main_dock" position="center" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `position` | string | `"center"` | Dock position: `"center"`, `"left"`, `"right"`, `"top"`, `"bottom"` |

### `stack`

Arranges children in a vertical or horizontal flex layout.

```xml
<stack id="toolbar" direction="horizontal" spacing="8" padding="16" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `direction` | string | `"vertical"` | `"vertical"` or `"horizontal"` |
| `spacing` | int | `0` | Gap between children in pixels |
| `padding` | int | | Inner padding in pixels |
| `scroll` | bool | `false` | Make the stack scrollable when content overflows |
| `border` | int | | Border width in pixels |
| `border_color` | string | | Border color (theme ref or hex) |
| `shadow` | string | | Shadow size: `"sm"`, `"md"`, `"lg"` |

### `panel`

A styled container with background, optional border, and shadow.

```xml
<panel id="card" padding="16" border="2" border-color="theme.border" shadow="md" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `padding` | int | | Inner padding in pixels |
| `border` | int | | Border width in pixels |
| `border_color` | string | | Border color |
| `shadow` | string | | Shadow: `"sm"`, `"md"`, `"lg"` |
| `visible` | bool | `true` | Visibility toggle |

### `label`

Displays text with configurable size.

```xml
<label id="title" text="Hello World" size="xl" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `text` | string | `""` | Display text |
| `size` | string | `"md"` | Size: `"xs"`, `"sm"`, `"md"`, `"lg"`, `"xl"` |

### `text`

A block of text content.

```xml
<text id="paragraph" content="A longer block of text content." />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | string | `""` | Text content |

### `button`

A clickable button with style variants.

```xml
<button id="submit" label="Submit" variant="primary" on-click="on_submit" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | (required) | Button text |
| `variant` | string | `"primary"` | Visual style (see below) |
| `disabled` | bool | `false` | Disable interaction |
| `on_click` | string | | Handler function name |

**Button Variants:** `primary`, `secondary`, `danger`, `ghost`, `warning`, `success`, `info`

### `input`

A text input field.

```xml
<input id="name_input" placeholder="Enter your name" disabled="false" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `placeholder` | string | | Placeholder text |
| `value` | string | | Initial value |
| `disabled` | bool | `false` | Disable input |

### `textarea`

A multi-line text input area.

```xml
<textarea id="notes" placeholder="Enter notes..." rows="4" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `placeholder` | string | | Placeholder text |
| `default_value` | string | | Initial content |
| `rows` | int | | Number of visible rows |
| `auto_grow_min` | int | | Minimum rows when auto-growing |
| `auto_grow_max` | int | | Maximum rows when auto-growing |
| `disabled` | bool | `false` | Disable input |

### `code_editor`

A code editor with syntax highlighting and line numbers.

```xml
<code_editor id="editor" language="rust" line-number="true" default-value="fn main() {}" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `language` | string | `"plain"` | Syntax highlighting language |
| `line_number` | bool | `true` | Show line numbers |
| `searchable` | bool | `true` | Enable in-editor search |
| `default_value` | string | | Initial content |
| `multi_line` | bool | `true` | Allow multiple lines |
| `tab_size` | int | `4` | Spaces per tab |
| `hard_tabs` | bool | `false` | Insert tab characters instead of spaces |
| `rows` | int | | Number of visible rows |
| `disabled` | bool | `false` | Disable editing |

### `text_editor`

A rich text editor with a formatting toolbar.

```xml
<text_editor id="body" placeholder="Compose..." rows="8" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `placeholder` | string | | Placeholder text |
| `default_value` | string | | Initial content |
| `rows` | int | | Number of visible rows |
| `disabled` | bool | `false` | Disable editing |

### `checkbox`

A toggleable checkbox with optional label.

```xml
<checkbox id="agree" label="I agree to the terms" checked="false" on-click="on_agree_changed" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Checkbox label text |
| `checked` | bool | `false` | Initial checked state |

The change handler receives `"true"` or `"false"` as event data.

### `select`

A dropdown selection component. Options are declared as `option` child elements.

```xml
<select id="color_picker" value="Cherry">
  <option value="Apple" />
  <option value="Banana" />
  <option value="Cherry" />
</select>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | string | | Currently selected value |

### `option`

A selectable entry for use inside a `select` or `radio`.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | string | (required) | The option's value |
| `label` | string | | Display text (defaults to `value`) |

### `icon`

Displays a named icon.

```xml
<icon id="settings_icon" name="settings" size="md" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | string | (required) | Icon name |
| `size` | int | `16` | Icon size in pixels |

### `image`

Displays an image.

```xml
<image id="logo" src="/path/to/image.png" alt="Company logo" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `src` | string | | Image source path |
| `alt` | string | | Alt text |

### `progress`

A progress bar.

```xml
<progress id="upload_progress" value="75" max="100" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | float | | Current progress value |
| `max` | float | `100` | Value corresponding to a full bar |

### `list`

A vertical list. Each row is a `list-item` child whose own children are the row content.

```xml
<list id="task_list">
  <list-item><text id="t1" content="Task 1" /></list-item>
  <list-item><text id="t2" content="Task 2" /></list-item>
  <list-item><text id="t3" content="Task 3" /></list-item>
</list>
```

The `list` and `list-item` elements take no properties of their own beyond the [common properties](#common-properties).

### `notification`

A status notification message.

```xml
<notification id="alert" message="Operation completed successfully" kind="success" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `message` | string | | Notification text |
| `kind` | string | `"info"` | Type: `"info"`, `"success"`, `"warning"`, `"error"` |

### `modal`

An overlay dialog, conditionally rendered.

```xml
<modal id="confirm_dialog" title="Confirm Action" open="false">
  <label id="body" text="Are you sure?" />
</modal>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | `""` | Modal title text |
| `open` | bool | `false` | Whether the modal is visible |

### `tooltip`

Wraps child content with a tooltip.

```xml
<tooltip id="help" content="This is additional context">
  <icon id="icon" name="help" />
</tooltip>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | string | | Tooltip text |

### `tabs`

A tabbed container. Each tab is a `tab-item` child; the tab label comes from the item's `label` property and its children form the tab body.

```xml
<tabs id="view_tabs" active-tab="0">
  <tab-item id="tab1" label="Overview">
    <text id="tab1_body" content="Tab 1 content" />
  </tab-item>
  <tab-item id="tab2" label="Details">
    <text id="tab2_body" content="Tab 2 content" />
  </tab-item>
</tabs>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `variant` | string | | Visual style of the tab strip |
| `active_tab` | int | `0` | Index of the initially selected tab |

Each `tab-item` takes a `label` property for its tab strip text.

### `accordion`

Collapsible content sections. Each section is an `accordion-item` child with a title; the item's own children are its expandable body.

```xml
<accordion id="faq" multiple="true" bordered="false">
  <accordion-item id="q1" title="Question 1" open="true">
    <text id="a1" content="Answer 1" />
  </accordion-item>
  <accordion-item id="q2" title="Question 2">
    <text id="a2" content="Answer 2" />
  </accordion-item>
</accordion>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `multiple` | bool | `false` | Allow multiple items open simultaneously |
| `bordered` | bool | `true` | Show borders between items |

#### `accordion-item`

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | | Section header text |
| `open` | bool | `false` | Whether the section starts expanded |

### `alert`

Displays an important status message with a variant-colored indicator.

```xml
<alert id="error_alert" title="Error" message="Something went wrong." variant="error" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | | Alert heading |
| `message` | string | `""` | Alert body text |
| `variant` | string | `"info"` | Visual style: `"info"`, `"success"`, `"warning"`, `"error"` |

### `avatar`

Displays user initials derived from a name.

```xml
<avatar id="user_avatar" name="Alice Johnson" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | string | | Full name (initials are generated automatically) |

### `badge`

Overlays a count or dot indicator on a child element.

```xml
<badge id="notification_badge" count="5">
  <button id="btn" label="Inbox" />
</badge>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `count` | int | | Numeric count to display |
| `dot` | bool | `false` | Show a dot indicator instead of a count |

Badge wraps its child component and renders the indicator in the top-right corner.

### `bubble`

A chat bubble surface with variants and alignment. Consecutive bubbles from one
sender go in a `bubble_group`; an optional `bubble_reactions` child attaches a
reaction pill.

```xml
<bubble id="msg_bubble" text="Hello there!" variant="secondary" />
<bubble_group id="thread">
  <bubble id="b1" text="First" />
  <bubble id="b2" text="Second" alignment="end">
    <bubble_reactions id="r1" text="👍" />
  </bubble>
</bubble_group>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `text` | string | | Bubble body text |
| `alignment` | string | | `"start"` or `"end"` within its row |
| `variant` | string | `"filled"` | `"filled"`, `"secondary"`, `"muted"`, `"tinted"`, `"outline"`, `"ghost"`, `"destructive"` |

`bubble_reactions` takes `side` (`"top"`/`"bottom"`, default `"bottom"`) and
`alignment` (default `"end"`); its text/children are the reactions shown.

### `kbd`

A tag for displaying keyboard keybindings.

```xml
<kbd id="shortcut" keys="cmd-k" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `keys` | string | `"cmd-k"` | Key combination (e.g. `"cmd-k"`, `"ctrl-shift-p"`) |
| `outline` | bool | `false` | Outline style instead of filled |

### `marker`

A compact row for status text, timeline boundaries, unread labels, and system
notices.

```xml
<marker id="unread" text="Unread messages" icon="mail" variant="plain" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `text` | string | | Marker text |
| `icon` | string | | Optional leading icon |
| `variant` | string | `"plain"` | Visual style |
| `alignment` | string | | Row alignment |
| `loading` | bool | `false` | Show a loading indicator instead of the icon |
| `loading_style` | string | `"spinner"` | `"spinner"` or other indicator style |
| `text_color` | string | | Text color override (theme ref or hex) |

### `message`

A chat message row with sender identity, metadata, a bubble body, and a footer.

```xml
<message id="chat_msg" sender="Alice" timestamp="10:42" text="Hey!"
         alignment="start" footer="Delivered" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `sender` | string | | Sender name |
| `timestamp` | string | | Timestamp/metadata text |
| `avatar` | string | | Avatar name (initials are derived) |
| `text` | string | | Message body |
| `alignment` | string | `"start"` | `"start"` (incoming) or `"end"` (outgoing) |
| `variant` | string | | Bubble variant (defaults by alignment) |
| `footer` | string | | Footer text under the bubble |

### `collapsible`

A single expandable/collapsible section with a clickable title bar.

```xml
<collapsible id="details" title="Show Details" open="false">
  <text id="content" content="Hidden content revealed when expanded." />
</collapsible>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | `"Details"` | Clickable header text |
| `open` | bool | `false` | Initial expanded state |

### `dropdown_button`

A button that opens a dropdown menu. Menu entries are declared as `menu-item` children.

```xml
<dropdown_button id="actions" label="Actions" variant="primary">
  <menu-item label="Copy" />
  <menu-item label="Paste" />
  <menu-item label="Cut" />
</dropdown_button>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Action"` | Button text |
| `variant` | string | | Button variant: `"primary"`, `"secondary"`, `"danger"`, etc. |

### `menu`

A button trigger that opens a popup menu. Entries are built from the same child
elements as `dropdown_button` — `menu-item`, `menu-label`, `menu-separator`,
and `menu-submenu` — with more layout control:

```xml
<menu id="actions" label="Actions" anchor="bottom-left" scrollable="true">
  <menu-label text="Edit" />
  <menu-item label="Copy" icon="copy" />
  <menu-item label="Paste" icon="clipboard" />
  <menu-separator />
  <menu-submenu label="Share" icon="share">
    <menu-item label="Copy link" />
    <menu-item label="Email" />
  </menu-submenu>
</menu>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Menu"` | Trigger button text |
| `variant` | string | | Button variant |
| `size` | string | `"medium"` | Trigger size |
| `anchor` | string | `"top-left"` | Where the popup attaches to the trigger |
| `scrollable` | bool | `false` | Scroll long menus |
| `max_height` | int | | Max popup height in pixels (implies scrolling) |
| `disabled` | bool | `false` | Disable the trigger |

### Menu children

Shared by `menu` and `dropdown_button`:

| Element | Properties | Description |
|---------|-----------|-------------|
| `menu-item` | `label`, `icon`, `disabled`, `checked`, `href` | A menu entry; `href` makes it an external link |
| `menu-label` | `text` | A non-interactive section label |
| `menu-separator` | — | A separator line |
| `menu-submenu` | `label`, `icon` | A nested submenu; its children are menu entries |

### `radio`

A group of mutually exclusive options.

Options are declared as `option` child elements (the same element used by `select`).

```xml
<radio id="size_picker" value="Medium" direction="horizontal">
  <option value="Small" />
  <option value="Medium" />
  <option value="Large" />
</radio>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | string | | Initially selected option |
| `direction` | string | `"vertical"` | Layout: `"vertical"` or `"horizontal"` |

### `slider`

A draggable range input for numeric values.

```xml
<slider id="volume" min="0" max="100" step="1" value="50" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `min` | float | `0.0` | Minimum value |
| `max` | float | `100.0` | Maximum value |
| `step` | float | `1.0` | Step increment |
| `value` | float | `0.0` | Initial value |

### `spinner`

An animated loading indicator.

```xml
<spinner id="loading" size="lg" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `size` | string | `"md"` | Size: `"xs"`, `"sm"`, `"md"`, `"lg"` |

### `skeleton`

A loading placeholder with a pulse animation. Size it with the common
`width`/`height`/`rounded` properties to match the content it stands in for.

```xml
<skeleton id="loading_row" width="240" height="16" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `secondary` | bool | `false` | Use the muted (secondary) surface color |

### `svg`

Renders SVG vector graphics from a file/URL (`src`) or embedded inline markup
(`content`). `currentColor` inside the SVG resolves against the theme.

```xml
<svg id="logo" src="assets/logo.svg" width="48" height="48" />
<svg id="icon" content="<svg xmlns='…'><path …/></svg>" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `src` | string | | Path or URL of an `.svg` file |
| `content` | string | | Inline SVG markup (overrides `src`) |

### `pagination`

Page navigation with prev/next buttons, page numbers, and an ellipsis menu of
hidden pages. The new page number is handed to the `on-click` handler; the
app updates `current-page` in response.

```xml
<pagination id="pager" current-page="1" total-pages="42" on-click="on_page" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `current_page` | int | `1` | Active page |
| `total_pages` | int | `1` | Number of pages |
| `visible_pages` | int | `5` | Page numbers shown around the current page |
| `compact` | bool | `false` | Compact `‹ 3/42 ›` mode |
| `disabled` | bool | `false` | Disable all controls |
| `size` | string | `"medium"` | Control size |

### `rating`

A star rating row. Clicking a star updates the value and fires `on-click` with
the new rating.

```xml
<rating id="score" value="3" max="5" on-click="on_rate" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | int | `0` | Current rating |
| `max` | int | `5` | Number of stars |
| `size` | string | `"medium"` | Star size |
| `color` | string | | Star color (theme ref or hex) |
| `disabled` | bool | `false` | Read-only |

### `stepper`

A step-by-step progress indicator. `stepper-item` children are the steps;
`label` is the step text and children override it.

```xml
<stepper id="setup" selected-index="1">
  <stepper-item id="s1" label="Account" />
  <stepper-item id="s2" label="Profile" icon="user" />
  <stepper-item id="s3" label="Done" disabled="true" />
</stepper>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `selected_index` | int | `0` | Index of the current step |
| `layout` | string | `"horizontal"` | `"horizontal"` or `"vertical"` |
| `size` | string | `"medium"` | Step size |
| `text_center` | bool | `false` | Center step labels |
| `disabled` | bool | `false` | Disable all steps |

Each `stepper-item` takes `label`, `icon`, and `disabled`.

### `switch`

A toggle control with a sliding visual style.

```xml
<switch id="dark_mode" label="Dark Mode" checked="true" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Switch label text |
| `checked` | bool | `false` | Initial state |
| `disabled` | bool | `false` | Disable interaction |

### `tag`

A small colored label for categorization.

```xml
<tag id="status_tag" label="Active" variant="success" outline="true" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Tag"` | Tag text |
| `variant` | string | `"secondary"` | Color: `"primary"`, `"secondary"`, `"danger"`, `"success"`, `"warning"`, `"info"` |
| `outline` | bool | `false` | Use outline style instead of filled |

### `toggle`

A button that toggles between on/off states, optionally with an icon.

```xml
<toggle id="favorite" label="Favorite" icon="star" checked="false" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Toggle button text |
| `icon` | string | | Optional icon name |
| `checked` | bool | `false` | Initial state |
| `disabled` | bool | `false` | Disable interaction |

### `sidenav_bar`

A vertical navigation sidebar with collapsible icon+label items. When collapsed, only icons are shown. When expanded, icons and labels are shown side by side. Has a 1px border on left and right by default.

```xml
<sidenav_bar id="sidebar" collapsed="false" width="200">
  <sidenav_bar_item id="nav_home" icon="globe" label="Home" />
  <sidenav_bar_item id="nav_inbox" icon="inbox" label="Inbox" />
  <sidenav_bar_item id="nav_settings" icon="settings" label="Settings" />
</sidenav_bar>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `collapsed` | bool | `false` | When `true`, show icons only (narrow bar). When `false`, show icons and labels. |
| `width` | int | `200` | Width of the sidebar in pixels when expanded. Collapsed width is fixed at 48px. |

The `collapsed` property can be toggled from RHAI scripts using `set_component_property()` to dynamically expand or collapse the sidebar.

Non-`sidenav_bar_item` children (such as buttons) are rendered at the bottom of the sidebar, useful for placing a collapse/expand toggle button.

### `sidenav_bar_item`

A navigation item for use inside a `sidenav_bar`. Displays an icon and a label. When the parent `sidenav_bar` is collapsed, only the icon is shown.

```xml
<sidenav_bar_item id="nav_home" icon="globe" label="Home" />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `icon` | string | `"info"` | Icon name (see [icon names](#icon)) |
| `label` | string | `""` | Display text shown when the parent sidenav is expanded |

Items use the `theme.sidebar_foreground` text color and `theme.list_hover` background on hover.

### `app_shell` (App Shell containers)

A standard application frame: a sidenav of section menu items, switchable
content pages, and a footer. Six registered types work together:

```xml
<app_shell id="shell" sidenav-width="220" default-page="overview">
  <app_sidenav>
    <sidenav_item id="nav_overview" icon="layout-dashboard" label="Overview" target="overview" />
    <sidenav_item id="nav_settings" icon="settings" label="Settings" target="settings" />
  </app_sidenav>
  <app_content>
    <page id="overview"><label text="Overview content" /></page>
    <page id="settings"><label text="Settings content" /></page>
  </app_content>
  <app_footer>
    <stack direction="horizontal"><label text="Ready" /></stack>
  </app_footer>
</app_shell>
```

| Element | Properties | Description |
|---------|-----------|-------------|
| `app_shell` | `sidenav_width` (default `200`), `collapsed`, `default_page` | The frame; `collapsed` renders the icon-only sidenav, `default_page` names the initially-active page |
| `app_sidenav` | — | Sidenav region; holds `sidenav_item` children |
| `sidenav_item` | `icon` (default `"info"`), `label`, `target` | Menu row; clicking switches to the `<page id>` matching `target` and highlights the item — no handler needed. `on-click` still fires if present |
| `app_content` | — | Content region; holds `page` children |
| `page` | — (its `id` is the target) | A content page, shown when its id is the active target |
| `app_footer` | — | Full-width footer region |

Only the active page's body renders — pages unmount when not shown. Toggle
`collapsed` at runtime with `set_component_property("shell", "collapsed", ...)`.

### `router` / `route` / `nav_link`

A chrome-free page router: renders one of its `route` children by URL-style
path, with history, `:param` capture, `*` fallback, and nested routers.

```xml
<router id="main" default="/home" primary="true">
  <route path="/home"><label text="Home" /></route>
  <route path="/users/:id"><label text="User" /></route>
  <route path="*"><label text="Not found" /></route>
</router>

<nav_link router="main" route="/home" label="Home" />
```

| Element | Properties | Description |
|---------|-----------|-------------|
| `router` | `default` (default `"/"`), `primary` | The switch; `primary` marks the router targeted by a bare `--route` flag or `navigate(path)` |
| `route` | `path`, `on_enter`, `on_leave` | A route body; `on-enter`/`on-leave` are Rhai lifecycle hooks fired on navigation |
| `nav_link` | `route`, `router`, `label` | A clickable link; highlights when its route is active |

Route state projects into the data repository at `data.route.<router-id>.*`
(path and params), readable from bindings and scripts. Scripts navigate with
`navigate(path)` / `navigate(router, path)` and `back()` / `forward()`; the CLI
can start at a route with `--route /path` or `--route <router-id>=/path`.
Only the active route's body is mounted — inactive routes don't render.

### `table`

Tabular data display with columns, sorting, and striped rows. Two authoring
styles: **declarative children** (columns and rows as nested elements) or
**data-driven** (`columns`/`data` JSON arrays).

```xml
<!-- Declarative: columns, rows, and cells as children. Text content is the
     cell/column label. -->
<table id="users" stripe="true" height="220">
  <table-header>
    <table-column field="name" width="150">Name</table-column>
    <table-column field="email" width="220">Email</table-column>
  </table-header>
  <table-row>
    <table-cell>Alice Johnson</table-cell>
    <table-cell>alice@example.com</table-cell>
  </table-row>
</table>

<!-- Data-driven: same table from JSON attributes. -->
<table id="users2" stripe="true"
  columns='[{"key": "name", "label": "Name", "width": 150}, {"key": "email", "label": "Email", "width": 220}]'
  data='[{"name": "Alice", "email": "alice@example.com"}, {"name": "Bob", "email": "bob@example.com"}]'
/>
```

Declarative children:

| Element | Properties | Description |
|---------|-----------|-------------|
| `table_header` | — | The header row; holds `table-column` children |
| `table_column` | `field`, `width`, `align` | A column definition; text content is the label |
| `table_row` | `col_span` | A row; holds `table-cell` children |
| `table_cell` | `col_span`, `row_span`, `align` | A cell; children are the content, cells are positional |

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `columns` | array | `[]` | Array of column objects with `key`, `label`, and optional `width` (int) |
| `data` | array | `[]` | Array of row objects keyed by column `key` values |
| `stripe` | bool | `false` | Alternate row background colors |
| `bordered` | bool | `true` | Draw borders around cells |
| `height` | int | `300` | Container height in pixels (required for scrollable content) |

!!! note
    Table requires a parent with definite height. If you don't set `height`, the default is 300px. Headers will always be visible, but rows may not render without sufficient height.

### `tree`

Hierarchical tree view with expand/collapse and keyboard navigation. Like
`table`, it can be authored declaratively with nested `tree_node` children
(text content or non-tree-node children are the node label) or data-driven
from an `items` JSON array:

```xml
<!-- Declarative. -->
<tree id="file_tree" height="300">
  <tree_node id="src" label="src" expanded="true">
    <tree_node id="main.rs" label="main.rs" />
    <tree_node id="lib.rs" label="lib.rs" />
  </tree_node>
  <tree_node id="Cargo.toml" label="Cargo.toml" />
</tree>

<!-- Data-driven. -->
<tree id="file_tree2" items='[{"id": "src", "label": "src", "expanded": true, "children": [{"id": "src/main.rs", "label": "main.rs"}]}, {"id": "Cargo.toml", "label": "Cargo.toml"}]' />
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `items` | array | `[]` | Array of tree item objects (see below) |
| `height` | int | `300` | Container height in pixels |

**Tree item object:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique item identifier |
| `label` | string | No | Display text (defaults to `id`) |
| `expanded` | bool | No | Initial expanded state |
| `disabled` | bool | No | Disable the item |
| `children` | array | No | Nested tree items |

### Charts

Nemo includes seventeen chart types for data visualization. All charts read data from a `data` array property and map fields via axis properties.

Charts that plot against a category axis also accept an optional `tick_margin` (int) property controlling the gap between axis ticks and labels. Chart height is set via the common `height` property (default 300px); like `table` and `tree`, charts need a definite height to render.

#### `line_chart`

```xml
<line_chart id="revenue" x-field="month" y-field="revenue" dot="true" height="300"
  data='[{"month": "Jan", "revenue": 186}, {"month": "Feb", "revenue": 305}, {"month": "Mar", "revenue": 237}]'
/>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field name for x-axis labels |
| `y_field` | string | (required) | Field name for y-axis values |
| `dot` | bool | `false` | Show data point dots |
| `linear` | bool | `false` | Use linear interpolation (vs. smooth curves) |
| `height` | int | `300` | Chart height in pixels |
| `data` | array | `[]` | Array of data point objects |

#### `bar_chart`

```xml
<bar_chart id="visitors" x-field="month" y-field="visitors" show-label="true" height="300"
  data='[{"month": "Jan", "visitors": 275}, {"month": "Feb", "visitors": 200}]'
/>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field name for x-axis labels |
| `y_field` | string | (required) | Field name for bar values |
| `show_label` | bool | `false` | Show value labels on bars |
| `height` | int | `300` | Chart height in pixels |
| `data` | array | `[]` | Array of data point objects |

#### `area_chart`

Supports multiple series via `y_fields`.

```xml
<area_chart id="traffic" x-field="month" y-fields='["desktop", "mobile"]' height="300"
  data='[{"month": "Jan", "desktop": 186, "mobile": 80}, {"month": "Feb", "desktop": 305, "mobile": 200}]'
/>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field name for x-axis labels |
| `y_fields` | array | (required) | List of field names for each series |
| `fill_opacity` | float | | Opacity of the filled area (0.0 - 1.0) |
| `height` | int | `300` | Chart height in pixels |
| `data` | array | `[]` | Array of data point objects |

#### `pie_chart`

Set `inner_radius` to create a donut chart.

```xml
<pie_chart id="browsers" value-field="amount" height="300"
  data='[{"label": "Chrome", "amount": 275}, {"label": "Safari", "amount": 200}, {"label": "Firefox", "amount": 187}]'
/>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value_field` | string | (required) | Field name for slice values |
| `outer_radius` | float | | Outer radius in pixels |
| `inner_radius` | float | | Inner radius (set > 0 for donut chart) |
| `height` | int | `300` | Chart height in pixels |
| `data` | array | `[]` | Array of data point objects |

#### `candlestick_chart`

Financial OHLC (Open-High-Low-Close) chart.

```xml
<candlestick_chart id="stocks" x-field="date" open-field="open" high-field="high" low-field="low" close-field="close" height="300"
  data='[{"date": "Mon", "open": 100.0, "high": 110.0, "low": 95.0, "close": 108.0}, {"date": "Tue", "open": 108.0, "high": 115.0, "low": 102.0, "close": 104.0}]'
/>
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for x-axis labels |
| `open_field` | string | (required) | Field for open values |
| `high_field` | string | (required) | Field for high values |
| `low_field` | string | (required) | Field for low values |
| `close_field` | string | (required) | Field for close values |
| `height` | int | `300` | Chart height in pixels |
| `data` | array | `[]` | Array of OHLC data objects |

#### `column_chart`

A vertical column chart. Behaves as an alias of `bar_chart` with the same properties: `x_field` (required), `y_field` (required), `show_label`, and `data`.

#### `scatter_chart`

A scatter plot of data points on a numeric plane.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for x values |
| `y_field` | string | (required) | Field for y values |
| `dot_size` | float | | Point size |
| `data` | array | `[]` | Array of data point objects |

#### `bubble_chart`

A scatter chart with variable-size bubbles.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for x values |
| `y_field` | string | (required) | Field for y values |
| `size_field` | string | (required) | Field driving bubble radius |
| `min_radius` | float | | Smallest bubble radius |
| `max_radius` | float | | Largest bubble radius |
| `data` | array | `[]` | Array of data point objects |

#### `stacked_column_chart` / `clustered_column_chart`

Vertical multi-series column charts — stacked one atop another, or grouped side by side.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for x-axis labels |
| `y_fields` | array | (required) | List of field names, one per series |
| `data` | array | `[]` | Array of data point objects |

#### `stacked_bar_chart` / `clustered_bar_chart`

Horizontal multi-series bar charts — stacked or grouped side by side.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `y_field` | string | (required) | Field for y-axis labels |
| `x_fields` | array | (required) | List of field names, one per series |
| `data` | array | `[]` | Array of data point objects |

#### `heatmap_chart`

A grid of colored cells representing values across two categories.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for the column category |
| `y_field` | string | (required) | Field for the row category |
| `value_field` | string | (required) | Field for the cell value |
| `data` | array | `[]` | Array of cell objects |

#### `radar_chart`

A radar (spider/web) chart with polygonal data series.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `categories` | array | (required) | Axis category labels |
| `y_fields` | array | (required) | List of field names, one per series |
| `max_value` | float | | Upper bound of the radial axis |
| `data` | array | `[]` | Array of data objects |

#### `pyramid_chart` / `funnel_chart`

A pyramid of centered horizontal bars sorted by value, or a funnel of narrowing segments.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label_field` | string | (required) | Field for segment labels |
| `value_field` | string | (required) | Field for segment values |
| `data` | array | `[]` | Array of segment objects |

#### `realtime_chart`

A multi-line chart for live time-series data with multiple traces.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `x_field` | string | (required) | Field for x-axis labels |
| `y_fields` | array | | List of trace field names |
| `linear` | bool | `false` | Use linear interpolation (vs. smooth curves) |
| `data` | array | `[]` | Array of data point objects |

---

## Data Binding

Connect data sources to component properties so that components update automatically when data changes.

### Binding Element

```xml
<label id="display" text="Waiting...">
  <binding source="data.ticker" target="text" transform="tick" />
</label>
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `source` | string | (required) | Data path (e.g., `"data.source_name"`) |
| `target` | string | (required) | Component property to update |
| `transform` | string | | Field name to extract from the data |
| `mode` | string | `"one_way"` | Binding mode: `"one_way"`, `"two_way"`, or `"one_time"` (set once at init) |

### Shorthand Binding

Use the `bind-` prefix as a shortcut:

```xml
<text id="raw_data" content="No data yet" bind-content="data.api" />
```

This is equivalent to:

```xml
<binding source="data.api" target="content" />
```

### Transform

The `transform` attribute extracts a field from the source data. Given data `{ tick: 42, timestamp: "2026-01-01" }`:

- `transform = "tick"` produces `42`
- `transform = "timestamp"` produces `"2026-01-01"`

Nested paths are supported: `transform = "payload.temperature"`.

### Multiple Bindings

A component can have multiple bindings:

```xml
<label id="sensor" text="Loading...">
  <binding source="data.sensors" target="text" transform="payload" />
</label>
```

---

## Event Handlers

Attach RHAI functions to component events using `on_<event>` attributes.

```xml
<button id="btn" label="Click" on-click="handle_click" />
```

The handler name references a function defined in a RHAI script. By default, Nemo looks for the function in the `handlers` script (loaded from `scripts/handlers.rhai`). To call a function in a different script, use the `script_id::function_name` format:

```xml
on-click="utils::format_data"
```

---

## Control-Flow Directives

`.nemo` templates support Vue-style namespaced attributes for iteration and
conditionals:

### `n:if`

Compile-time conditional. The compiler emits a `bind-visible` binding from the
expression, so the element shows/hides as the bound data changes:

```xml
<panel id="details" n:if="data.ui.show_details == 'yes'">...</panel>
```

### `n:for`

Iteration. Over a **static** list it expands at compile time; over a **live
data source** it becomes a runtime list binding that creates and removes
component instances as the data changes. `n:key` names the identity field:

```xml
<!-- Static: one chip per literal, expanded at compile time. -->
<label id="chip" n:for="tag in ['config', 'sfc', 'runtime']" n:key="tag"
       text="#${tag}" />

<!-- Live: one row per task in data.tasks, keyed by id. -->
<stack id="task_list" n:for="task in data.tasks" n:key="task.id">
  <text>${task.name}</text>
</stack>
```

---

## SFC Imports and Components

Single-file components can be imported and used as custom tags. Import an
explicit file with `<import>`, or a whole directory with `<components dir>`:

```xml
<imports>
  <import src="./templates/card.nemo" />
</imports>

<!-- or: -->
<components dir="./components" />
```

Every template defined in an imported SFC becomes a usable element. Imported
component instances need explicit `id`s prefixed to avoid anonymous-id
collisions across instances:

```xml
<template name="app">
  <card id="my_card">
    <label id="my_card_title" slot="header" text="A card" />
  </card>
</template>
```

---

## Actions

Actions are named, schema-validated operations registered by the runtime and
available to tooling via `nemo schema`. The built-ins:

| Action | Key properties | Description |
|--------|---------------|-------------|
| `notification` | `message` (required), `type` | Display a notification |
| `navigate` | `target` (required) | Navigate to a different view |
| `refresh` | `target` | Refresh data or UI |
| `http_request` | `url` (required), `method`, `body` | Make an HTTP request |
| `set_data` | `target` (required), `value` (required) | Set a data value |

---

## Expression Language

XML attributes support expressions inside `${}` delimiters.

### Variable References

```xml
min-height="${var.button_height}"
url="${var.api_base}/endpoint"
```

### Environment Variables

```xml
path="${env.HOME}/config"
```

### String Interpolation

```xml
title="Hello ${var.user_name}!"
```

### Built-in Functions

| Function | Description |
|----------|-------------|
| `upper(s)` | Convert string to uppercase |
| `lower(s)` | Convert string to lowercase |
| `trim(s)` | Remove leading/trailing whitespace |
| `length(v)` | Length of string, array, or object |
| `coalesce(a, b, c)` | First non-null value |
| `env("VAR")` | Get environment variable |

### Conditional Expressions

```xml
text="${var.enabled ? 'Active' : 'Inactive'}"
```

---

## Color References

Properties that accept colors (like `border_color`) support two formats:

### Theme References

Reference colors from the active theme:

| Reference | Description |
|-----------|-------------|
| `theme.border` | Theme border color |
| `theme.accent` | Theme accent color |
| `theme.danger` | Theme danger/error color |
| `theme.background` | Theme background color |
| `theme.foreground` | Theme foreground/text color |

### Hex Colors

Standard CSS hex colors:

```xml
border-color="#FF6600"
border-color="#FF660080"   <!-- with alpha -->
```
