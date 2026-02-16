# Configuration Reference

Nemo applications are defined in HCL (HashiCorp Configuration Language) files. This page is a complete reference for every block, attribute, and component type.

## File Structure

A Nemo configuration file contains up to six top-level blocks:

```hcl
variable "name" { ... }   # Variable definitions (0 or more)
app { ... }                # Application and window settings
scripts { ... }            # Script loading configuration
templates { ... }          # Reusable component templates
data { ... }               # Data source and sink definitions
layout { ... }             # UI component tree
```

All blocks are optional. A minimal valid configuration is an empty file, though it will produce a blank window.

---

## `variable` Block

Define reusable variables with type and default value. Variables are referenced elsewhere via `${var.name}`.

```hcl
variable "button_height" {
  type    = "int"
  default = 48
}

variable "api_base" {
  type    = "string"
  default = "https://api.example.com"
}
```

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | No | Variable type: `"string"`, `"int"`, `"float"`, `"bool"` |
| `default` | any | No | Default value |

Use variables in any attribute value:

```hcl
min_height = "${var.button_height}"
url        = "${var.api_base}/users"
```

---

## `app` Block

Application metadata, window settings, and theme configuration.

```hcl
app {
  title = "My Application"

  window {
    title      = "Window Title"
    width      = 1024
    height     = 768
    min_width  = 320
    min_height = 240

    header_bar {
      github_url   = "https://github.com/user/repo"
      theme_toggle = true
    }
  }

  theme {
    name = "kanagawa"
    mode = "dark"

    extend "custom" {
      primary    = "#FF6600"
      background = "#1A1A2E"
    }
  }
}
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
| `kanagawa` | dark | Kanagawa Wave |
| `kanagawa-dragon` | dark | Kanagawa Dragon |
| `catppuccin` | light | Catppuccin Latte |
| `catppuccin-macchiato` | dark | Catppuccin Macchiato |
| `tokyo-night` | dark | Tokyo Night |
| `gruvbox` | dark | Gruvbox |
| `nord` | dark | Nord |

Theme resolution is case-insensitive. If a theme set has multiple variants, the variant matching the requested mode is selected.

The optional `extend` sub-block allows overriding individual theme colors.

---

## `scripts` Block

Configure where RHAI scripts are loaded from.

```hcl
scripts {
  path = "./scripts"
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `path` | string | Directory containing `.rhai` script files |

All `.rhai` files in the directory are loaded at startup. Scripts are identified by their filename without the extension (e.g., `handlers.rhai` becomes script ID `"handlers"`).

---

## `templates` Block

Define reusable component templates that can be referenced by components in the layout. Templates reduce duplication when many components share the same base configuration.

### Defining Templates

```hcl
templates {
  template "nav_item" {
    type         = "button"
    variant      = "ghost"
    size         = "sm"
    on_click     = "on_nav"
  }

  template "content_page" {
    type    = "panel"
    visible = false

    component "inner" {
      type      = "stack"
      direction = "vertical"
      spacing   = 12
      padding   = 32
      slot      = true
    }
  }
}
```

Each `template` block defines a named set of component properties. Templates can include nested `component` children and all standard component properties.

### Using Templates

Reference a template from any component in the layout with the `template` property:

```hcl
layout {
  type = "stack"

  component "nav_home" {
    template = "nav_item"
    label    = "Home"
  }

  component "nav_settings" {
    template = "nav_item"
    label    = "Settings"
  }
}
```

The component inherits all properties from the template. Properties set on the instance override the template's defaults (e.g., `label` above overrides anything the template set).

### Slots

Templates can designate a child component as a **slot** by setting `slot = true`. When an instance provides children, they are injected into the slot rather than appended at the top level:

```hcl
templates {
  template "card" {
    type    = "panel"
    padding = 16

    component "body" {
      type = "stack"
      direction = "vertical"
      slot = true
    }
  }
}

layout {
  type = "stack"

  component "user_card" {
    template = "card"

    component "title" {
      type = "label"
      text = "User Profile"
    }
  }
}
```

Here, the `title` label is injected inside the `body` stack, not directly under the `panel`.

### Template Features

- **Property override:** Instance properties take precedence over template defaults
- **Child scoping:** Template-originated child IDs are automatically prefixed with the instance ID to prevent collisions
- **Recursive templates:** A template can reference another template
- **Circular reference detection:** Nemo detects and reports circular template chains
- **Handler preservation:** Event handlers (e.g., `on_click`) from templates are preserved through expansion

---

## `data` Block

Define data sources (inputs) and sinks (outputs).

```hcl
data {
  source "name" {
    type = "..."
    # source-specific attributes
  }

  sink "name" {
    type = "..."
    # sink-specific attributes
  }
}
```

### Data Source Types

#### `timer`

Emits an incrementing tick count at a fixed interval.

```hcl
source "ticker" {
  type     = "timer"
  interval = 1
}
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `interval` | int | | Seconds between ticks |

**Emits:** `{ tick: <int>, timestamp: <string> }`

#### `http`

Polls an HTTP endpoint at a regular interval.

```hcl
source "api" {
  type     = "http"
  url      = "https://httpbin.org/get"
  interval = 30
  method   = "GET"
  timeout  = 30
}
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

```hcl
source "stream" {
  type = "websocket"
  url  = "wss://api.example.com/stream"
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | WebSocket URL (`ws://` or `wss://`) |

**Emits:** Each received message as a parsed JSON value.

#### `mqtt`

Subscribes to MQTT topics.

```hcl
source "sensors" {
  type      = "mqtt"
  host      = "localhost"
  port      = 1883
  topics    = ["sensors/#", "alerts/high"]
  client_id = "nemo-app"
}
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

```hcl
source "events" {
  type     = "redis"
  url      = "redis://127.0.0.1:6379"
  channels = ["app-events", "notifications"]
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | Redis connection URL |
| `channels` | array | List of channels to subscribe |

**Emits:** `{ channel: <string>, payload: <string|object> }`

#### `nats`

Subscribes to NATS subjects.

```hcl
source "messages" {
  type     = "nats"
  url      = "nats://127.0.0.1:4222"
  subjects = ["updates.>", "commands.*"]
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `url` | string | NATS server URL |
| `subjects` | array | List of subjects (supports NATS wildcards) |

**Emits:** `{ subject: <string>, payload: <string|object> }`

#### `file`

Reads from the filesystem, optionally watching for changes.

```hcl
source "config_file" {
  type   = "file"
  path   = "/path/to/data.json"
  watch  = true
  format = "json"
}
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | (required) | File path |
| `watch` | bool | `false` | Watch for changes |
| `format` | string | `"raw"` | Parse format: `"raw"`, `"json"`, `"lines"` |

### Data Sink Types

Sinks are destinations for publishing data from scripts.

#### MQTT Sink

```hcl
sink "commands" {
  type  = "mqtt"
  host  = "localhost"
  port  = 1883
  topic = "commands"
}
```

#### Redis Sink

```hcl
sink "outbound" {
  type    = "redis"
  url     = "redis://127.0.0.1:6379"
  channel = "app-commands"
}
```

#### NATS Sink

```hcl
sink "nats_out" {
  type    = "nats"
  url     = "nats://127.0.0.1:4222"
  subject = "commands.>"
}
```

---

## `layout` Block

Defines the UI component tree. The layout block specifies a root layout type and contains nested `component` blocks.

```hcl
layout {
  type = "stack"

  component "id" {
    type = "component_type"
    # properties...

    component "child_id" {
      type = "label"
      text = "Nested child"
    }
  }
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `type` | string | Root layout type (typically `"stack"`) |

---

## Components

### Common Properties

These properties are available on all components:

| Property | Type | Description |
|----------|------|-------------|
| `id` | string | Component identifier for script access. Defaults to the component block label. |
| `visible` | bool | Show or hide the component. Default: `true`. |
| `flex` | int | Flex grow factor |
| `width` | int | Fixed width in pixels |
| `height` | int | Fixed height in pixels |
| `min_width` | int | Minimum width in pixels |
| `min_height` | int | Minimum height in pixels |

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

### `stack`

Arranges children in a vertical or horizontal flex layout.

```hcl
component "toolbar" {
  type      = "stack"
  direction = "horizontal"
  spacing   = 8
  padding   = 16
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `direction` | string | `"vertical"` | `"vertical"` or `"horizontal"` |
| `spacing` | int | `4` | Gap between children in pixels |
| `padding` | int | | Inner padding in pixels |
| `border` | int | | Border width in pixels |
| `border_color` | string | | Border color (theme ref or hex) |
| `shadow` | string | | Shadow size: `"sm"`, `"md"`, `"lg"` |

### `panel`

A styled container with background, optional border, and shadow.

```hcl
component "card" {
  type         = "panel"
  padding      = 16
  border       = 2
  border_color = "theme.border"
  shadow       = "md"
}
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

```hcl
component "title" {
  type = "label"
  text = "Hello World"
  size = "xl"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `text` | string | `""` | Display text |
| `size` | string | `"md"` | Size: `"xs"`, `"sm"`, `"md"`, `"lg"`, `"xl"` |

### `text`

A block of text content.

```hcl
component "paragraph" {
  type    = "text"
  content = "A longer block of text content."
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | string | `""` | Text content |

### `button`

A clickable button with style variants.

```hcl
component "submit" {
  type     = "button"
  label    = "Submit"
  variant  = "primary"
  on_click = "on_submit"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Button"` | Button text |
| `variant` | string | `"secondary"` | Visual style (see below) |
| `disabled` | bool | `false` | Disable interaction |
| `on_click` | string | | Handler function name |

**Button Variants:** `primary`, `secondary`, `danger`, `ghost`, `warning`, `success`, `info`

### `input`

A text input field.

```hcl
component "name_input" {
  type        = "input"
  placeholder = "Enter your name"
  disabled    = false
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `placeholder` | string | | Placeholder text |
| `value` | string | | Initial value |
| `disabled` | bool | `false` | Disable input |

### `checkbox`

A toggleable checkbox with optional label.

```hcl
component "agree" {
  type     = "checkbox"
  label    = "I agree to the terms"
  checked  = false
  on_click = "on_agree_changed"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Checkbox label text |
| `checked` | bool | `false` | Initial checked state |
| `disabled` | bool | `false` | Disable interaction |

The change handler receives `"true"` or `"false"` as event data.

### `select`

A dropdown selection component.

```hcl
component "color_picker" {
  type    = "select"
  options = ["Red", "Green", "Blue"]
  value   = "Red"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `options` | array | | List of option strings |
| `value` | string | | Currently selected value |

### `icon`

Displays a named icon.

```hcl
component "settings_icon" {
  type = "icon"
  name = "settings"
  size = "md"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | string | | Icon name |
| `size` | string | `"md"` | Icon size |

### `image`

Displays an image.

```hcl
component "logo" {
  type = "image"
  src  = "/path/to/image.png"
  alt  = "Company logo"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `src` | string | | Image source path |
| `alt` | string | | Alt text |

### `progress`

A progress bar.

```hcl
component "upload_progress" {
  type  = "progress"
  value = 75
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value` | int | `0` | Progress value (0-100) |

### `list`

A vertical list of items.

```hcl
component "task_list" {
  type  = "list"
  items = ["Task 1", "Task 2", "Task 3"]
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `items` | array | `[]` | List of string items |

### `notification`

A status notification message.

```hcl
component "alert" {
  type    = "notification"
  message = "Operation completed successfully"
  kind    = "success"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `message` | string | | Notification text |
| `kind` | string | `"info"` | Type: `"info"`, `"success"`, `"warning"`, `"error"` |

### `modal`

An overlay dialog, conditionally rendered.

```hcl
component "confirm_dialog" {
  type  = "modal"
  title = "Confirm Action"
  open  = false

  component "body" {
    type    = "label"
    text    = "Are you sure?"
  }
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | `""` | Modal title text |
| `open` | bool | `false` | Whether the modal is visible |

### `tooltip`

Wraps child content with a tooltip.

```hcl
component "help" {
  type    = "tooltip"
  content = "This is additional context"

  component "icon" {
    type = "icon"
    name = "help"
  }
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | string | | Tooltip text |

### `tabs`

A tabbed container.

```hcl
component "view_tabs" {
  type = "tabs"

  component "tab1" {
    type = "label"
    text = "Tab 1 Content"
  }

  component "tab2" {
    type = "label"
    text = "Tab 2 Content"
  }
}
```

### `accordion`

Collapsible content sections. Each item has a title and expandable content.

```hcl
component "faq" {
  type = "accordion"
  items = [
    { title = "Question 1", content = "Answer 1", open = true },
    { title = "Question 2", content = "Answer 2" },
    { title = "Question 3", content = "Answer 3" }
  ]
  multiple = true
  bordered = false
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `items` | array | `[]` | Array of objects with `title`, `content`, and optional `open` (bool) |
| `multiple` | bool | `false` | Allow multiple items open simultaneously |
| `bordered` | bool | `true` | Show borders between items |

### `alert`

Displays an important status message with a variant-colored indicator.

```hcl
component "error_alert" {
  type    = "alert"
  title   = "Error"
  message = "Something went wrong."
  variant = "error"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | | Alert heading |
| `message` | string | `""` | Alert body text |
| `variant` | string | `"info"` | Visual style: `"info"`, `"success"`, `"warning"`, `"error"` |

### `avatar`

Displays user initials derived from a name.

```hcl
component "user_avatar" {
  type = "avatar"
  name = "Alice Johnson"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | string | | Full name (initials are generated automatically) |

### `badge`

Overlays a count or dot indicator on a child element.

```hcl
component "notification_badge" {
  type  = "badge"
  count = 5

  component "btn" {
    type  = "button"
    label = "Inbox"
  }
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `count` | int | | Numeric count to display |
| `dot` | bool | `false` | Show a dot indicator instead of a count |

Badge wraps its child component and renders the indicator in the top-right corner.

### `collapsible`

A single expandable/collapsible section with a clickable title bar.

```hcl
component "details" {
  type  = "collapsible"
  title = "Show Details"
  open  = false

  component "content" {
    type    = "text"
    content = "Hidden content revealed when expanded."
  }
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | string | `""` | Clickable header text |
| `open` | bool | `false` | Initial expanded state |

### `dropdown_button`

A button that opens a dropdown menu with selectable items.

```hcl
component "actions" {
  type    = "dropdown_button"
  label   = "Actions"
  variant = "primary"
  items   = ["Copy", "Paste", "Cut"]
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Action"` | Button text |
| `variant` | string | `"secondary"` | Button variant: `"primary"`, `"secondary"`, `"danger"`, etc. |
| `items` | array | `[]` | List of menu item strings |

### `radio`

A group of mutually exclusive options.

```hcl
component "size_picker" {
  type      = "radio"
  options   = ["Small", "Medium", "Large"]
  value     = "Medium"
  direction = "horizontal"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `options` | array | `[]` | List of option strings |
| `value` | string | | Initially selected option |
| `direction` | string | `"vertical"` | Layout: `"vertical"` or `"horizontal"` |

### `slider`

A draggable range input for numeric values.

```hcl
component "volume" {
  type  = "slider"
  min   = 0
  max   = 100
  step  = 1
  value = 50
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `min` | float | `0.0` | Minimum value |
| `max` | float | `100.0` | Maximum value |
| `step` | float | `1.0` | Step increment |
| `value` | float | `0.0` | Initial value |

### `spinner`

An animated loading indicator.

```hcl
component "loading" {
  type = "spinner"
  size = "lg"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `size` | string | `"md"` | Size: `"xs"`, `"sm"`, `"md"`, `"lg"` |

### `switch`

A toggle control with a sliding visual style.

```hcl
component "dark_mode" {
  type    = "switch"
  label   = "Dark Mode"
  checked = true
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Switch label text |
| `checked` | bool | `false` | Initial state |
| `disabled` | bool | `false` | Disable interaction |

### `tag`

A small colored label for categorization.

```hcl
component "status_tag" {
  type    = "tag"
  label   = "Active"
  variant = "success"
  outline = true
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `"Tag"` | Tag text |
| `variant` | string | `"secondary"` | Color: `"primary"`, `"secondary"`, `"danger"`, `"success"`, `"warning"`, `"info"` |
| `outline` | bool | `false` | Use outline style instead of filled |

### `toggle`

A button that toggles between on/off states, optionally with an icon.

```hcl
component "favorite" {
  type    = "toggle"
  label   = "Favorite"
  icon    = "star"
  checked = false
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `label` | string | `""` | Toggle button text |
| `icon` | string | | Optional icon name |
| `checked` | bool | `false` | Initial state |
| `disabled` | bool | `false` | Disable interaction |

### `sidenav_bar`

A vertical navigation sidebar with collapsible icon+label items. When collapsed, only icons are shown. When expanded, icons and labels are shown side by side. Has a 1px border on left and right by default.

```hcl
component "sidebar" {
  type      = "sidenav_bar"
  collapsed = false
  width     = 200

  component "nav_home" {
    type  = "sidenav_bar_item"
    icon  = "globe"
    label = "Home"
  }

  component "nav_inbox" {
    type  = "sidenav_bar_item"
    icon  = "inbox"
    label = "Inbox"
  }

  component "nav_settings" {
    type  = "sidenav_bar_item"
    icon  = "settings"
    label = "Settings"
  }
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `collapsed` | bool | `false` | When `true`, show icons only (narrow bar). When `false`, show icons and labels. |
| `width` | int | `200` | Width of the sidebar in pixels when expanded. Collapsed width is fixed at 48px. |

The `collapsed` property can be toggled from RHAI scripts using `set_component_property()` to dynamically expand or collapse the sidebar.

Non-`sidenav_bar_item` children (such as buttons) are rendered at the bottom of the sidebar, useful for placing a collapse/expand toggle button.

### `sidenav_bar_item`

A navigation item for use inside a `sidenav_bar`. Displays an icon and a label. When the parent `sidenav_bar` is collapsed, only the icon is shown.

```hcl
component "nav_home" {
  type  = "sidenav_bar_item"
  icon  = "globe"
  label = "Home"
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `icon` | string | `"info"` | Icon name (see [icon names](#icon)) |
| `label` | string | `""` | Display text shown when the parent sidenav is expanded |

Items use the `theme.sidebar_foreground` text color and `theme.list_hover` background on hover.

### `table`

Tabular data display with columns, sorting, and striped rows.

```hcl
component "users" {
  type   = "table"
  stripe = true
  columns = [
    { key = "name",  label = "Name",  width = 150 },
    { key = "email", label = "Email", width = 220 },
    { key = "role",  label = "Role",  width = 120 }
  ]
  data = [
    { name = "Alice", email = "alice@example.com", role = "Admin" },
    { name = "Bob",   email = "bob@example.com",   role = "Editor" }
  ]
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `columns` | array | `[]` | Array of column objects with `key`, `label`, and optional `width` (int) |
| `data` | array | `[]` | Array of row objects keyed by column `key` values |
| `stripe` | bool | `false` | Alternate row background colors |
| `height` | int | `300` | Container height in pixels (required for scrollable content) |

!!! note
    Table requires a parent with definite height. If you don't set `height`, the default is 300px. Headers will always be visible, but rows may not render without sufficient height.

### `tree`

Hierarchical tree view with expand/collapse and keyboard navigation.

```hcl
component "file_tree" {
  type = "tree"
  items = [
    {
      id       = "src"
      label    = "src"
      expanded = true
      children = [
        { id = "src/main.rs", label = "main.rs" },
        { id = "src/lib.rs",  label = "lib.rs" }
      ]
    },
    { id = "Cargo.toml", label = "Cargo.toml" }
  ]
}
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

Nemo includes five chart types for data visualization. All charts read data from a `data` array property and map fields via axis properties.

#### `line_chart`

```hcl
component "revenue" {
  type    = "line_chart"
  x_field = "month"
  y_field = "revenue"
  dot     = true
  height  = 300
  data = [
    { month = "Jan", revenue = 186 },
    { month = "Feb", revenue = 305 },
    { month = "Mar", revenue = 237 }
  ]
}
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

```hcl
component "visitors" {
  type       = "bar_chart"
  x_field    = "month"
  y_field    = "visitors"
  show_label = true
  height     = 300
  data = [
    { month = "Jan", visitors = 275 },
    { month = "Feb", visitors = 200 }
  ]
}
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

```hcl
component "traffic" {
  type     = "area_chart"
  x_field  = "month"
  y_fields = ["desktop", "mobile"]
  height   = 300
  data = [
    { month = "Jan", desktop = 186, mobile = 80 },
    { month = "Feb", desktop = 305, mobile = 200 }
  ]
}
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

```hcl
component "browsers" {
  type        = "pie_chart"
  value_field = "amount"
  height      = 300
  data = [
    { label = "Chrome",  amount = 275 },
    { label = "Safari",  amount = 200 },
    { label = "Firefox", amount = 187 }
  ]
}
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

```hcl
component "stocks" {
  type        = "candlestick_chart"
  x_field     = "date"
  open_field  = "open"
  high_field  = "high"
  low_field   = "low"
  close_field = "close"
  height      = 300
  data = [
    { date = "Mon", open = 100.0, high = 110.0, low = 95.0,  close = 108.0 },
    { date = "Tue", open = 108.0, high = 115.0, low = 102.0, close = 104.0 }
  ]
}
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

---

## Data Binding

Connect data sources to component properties so that components update automatically when data changes.

### Binding Block

```hcl
component "display" {
  type = "label"
  text = "Waiting..."

  binding {
    source    = "data.ticker"
    target    = "text"
    transform = "tick"
  }
}
```

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `source` | string | (required) | Data path (e.g., `"data.source_name"`) |
| `target` | string | (required) | Component property to update |
| `transform` | string | | Field name to extract from the data |
| `mode` | string | `"one_way"` | Binding mode: `"one_way"` or `"two_way"` |

### Shorthand Binding

Use the `bind_` prefix as a shortcut:

```hcl
component "raw_data" {
  type         = "text"
  content      = "No data yet"
  bind_content = "data.api"
}
```

This is equivalent to:

```hcl
binding {
  source = "data.api"
  target = "content"
}
```

### Transform

The `transform` attribute extracts a field from the source data. Given data `{ tick: 42, timestamp: "2026-01-01" }`:

- `transform = "tick"` produces `42`
- `transform = "timestamp"` produces `"2026-01-01"`

Nested paths are supported: `transform = "payload.temperature"`.

### Multiple Bindings

A component can have multiple bindings:

```hcl
component "sensor" {
  type = "label"
  text = "Loading..."

  binding {
    source    = "data.sensors"
    target    = "text"
    transform = "payload"
  }
}
```

---

## Event Handlers

Attach RHAI functions to component events using `on_<event>` attributes.

```hcl
component "btn" {
  type     = "button"
  label    = "Click"
  on_click = "handle_click"
}
```

The handler name references a function defined in a RHAI script. By default, Nemo looks for the function in the `handlers` script (loaded from `scripts/handlers.rhai`). To call a function in a different script, use the `script_id::function_name` format:

```hcl
on_click = "utils::format_data"
```

---

## Expression Language

HCL attributes support expressions inside `${}` delimiters.

### Variable References

```hcl
min_height = "${var.button_height}"
url        = "${var.api_base}/endpoint"
```

### Environment Variables

```hcl
path = "${env.HOME}/config"
```

### String Interpolation

```hcl
title = "Hello ${var.user_name}!"
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

```hcl
text = "${var.enabled ? \"Active\" : \"Inactive\"}"
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

```hcl
border_color = "#FF6600"
border_color = "#FF660080"   # with alpha
```
