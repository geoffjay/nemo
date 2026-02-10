# Getting Started

This guide walks through installing Nemo, creating your first application, and understanding the core concepts.

## Prerequisites

- Rust toolchain (1.75+)
- A supported platform: macOS, Linux, or Windows

## Installation

Clone the repository and build from source:

```bash
git clone https://github.com/geoffjay/nemo.git
cd nemo
cargo build --release
```

The binary is produced at `target/release/nemo`.

## Your First Application

Create a directory for your application and add an `app.hcl` file:

```bash
mkdir my-app && cd my-app
```

### 1. Define the Application

Create `app.hcl`:

```hcl
app {
  window {
    title = "My First App"
    width = 800
    height = 600

    header_bar {
      theme_toggle = true
    }
  }

  theme {
    name = "kanagawa"
    mode = "dark"
  }
}

scripts {
  path = "./scripts"
}

layout {
  type = "stack"

  component "main" {
    type = "stack"
    direction = "vertical"
    margin = 16
    spacing = 8

    component "title" {
      type = "label"
      text = "My First Nemo App"
      size = "xl"
    }

    component "content" {
      type = "panel"
      padding = 16

      component "greeting" {
        type = "label"
        text = "Click the button below to see scripting in action."
      }
    }

    component "action" {
      type = "button"
      label = "Say Hello"
      variant = "primary"
      on_click = "on_say_hello"
    }

    component "output" {
      type = "label"
      id = "output_label"
      text = ""
    }
  }
}
```

### 2. Add an Event Handler

Create `scripts/handlers.rhai`:

```rhai
fn on_say_hello(component_id, event_data) {
    set_component_text("output_label", "Hello from RHAI!");
    log_info("Button clicked!");
}
```

### 3. Run It

```bash
nemo --app-config app.hcl
```

You should see a window with a title, a panel, a button, and a label. Clicking the button updates the label text via the RHAI script.

## Core Concepts

### Configuration (HCL)

Nemo applications are defined in [HCL](https://github.com/hashicorp/hcl) files. The configuration contains four top-level blocks:

| Block | Purpose |
|-------|---------|
| `app` | Window settings, theme selection |
| `scripts` | Path to RHAI script files |
| `layout` | UI component tree |
| `data` | Data source and sink definitions |

### Components

Components are the building blocks of your UI. Each component has a `type` and a set of properties specific to that type.

```hcl
component "my_button" {
  type = "button"
  label = "Click Me"
  variant = "primary"
  on_click = "handle_click"
}
```

Available component types:

| Category | Types |
|----------|-------|
| Layout | `stack`, `panel` |
| Display | `label`, `text`, `icon`, `image`, `progress` |
| Input | `button`, `input`, `checkbox`, `select` |
| Data | `list`, `table`, `tree` |
| Overlay | `modal`, `tooltip`, `tabs`, `notification` |

### Data Sources

Data sources connect external systems to your UI. When a source emits new data, bound components update automatically.

```hcl
data {
  source "ticker" {
    type     = "timer"
    interval = 1
  }
}
```

Supported source types: `timer`, `http`, `websocket`, `mqtt`, `redis`, `nats`, `file`.

### Data Binding

Bind data source output directly to component properties:

```hcl
component "counter" {
  type = "label"
  text = "Waiting..."

  binding {
    source    = "data.ticker"
    target    = "text"
    transform = "tick"
  }
}
```

When the `ticker` source emits `{ tick: 42, timestamp: "..." }`, the binding extracts the `tick` field and sets the label text to `42`.

### Event Handlers (RHAI Scripts)

RHAI scripts handle user interactions. Handler functions receive two arguments: the component ID and event data.

```rhai
fn handle_click(component_id, event_data) {
    let data = get_data("ticker");
    set_component_text("output", "Tick: " + data.to_string());
    log_info("Handled click on " + component_id);
}
```

Link handlers to components with `on_click`:

```hcl
component "btn" {
  type     = "button"
  label    = "Refresh"
  on_click = "handle_click"
}
```

### Themes

Nemo ships with several built-in themes:

| Theme | Variants |
|-------|----------|
| Kanagawa | Wave, Dragon |
| Catppuccin | Latte, Macchiato |
| Tokyo Night | Default |
| Gruvbox | Default |
| Nord | Default |

Set the theme in your `app` block:

```hcl
app {
  theme {
    name = "catppuccin"
    mode = "dark"
  }
}
```

## Example Applications

The `examples/` directory contains complete applications:

| Example | Description |
|---------|-------------|
| `basic/` | Minimal application with a button and label |
| `calculator/` | Calculator with digit buttons, operators, and display |
| `components/` | Gallery showcasing every built-in component |
| `data-binding/` | Live data from timer, HTTP, MQTT, Redis, and NATS sources |

Run any example:

```bash
nemo --app-config examples/basic/app.hcl
nemo --app-config examples/calculator/app.hcl
```

## Next Steps

- [CLI Reference](cli.md) -- All command-line options
- [Configuration Reference](configuration.md) -- Every block, attribute, and component type
- [Development Guide](develop.md) -- Writing scripts and building plugins
