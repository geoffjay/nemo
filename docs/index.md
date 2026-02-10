# Nemo

Nemo is a configuration-driven desktop application framework. Define your UI layout, data sources, and event handlers in HCL configuration files and RHAI scripts -- Nemo handles rendering, data flow, and state management.

Built on [GPUI](https://gpui.rs), Nemo produces native, GPU-accelerated desktop applications from declarative configuration.

## What Nemo Does

- **Declarative UI** -- Define component trees in HCL instead of writing rendering code
- **Live Data Binding** -- Connect HTTP APIs, MQTT brokers, Redis, NATS, WebSockets, and timers directly to UI components
- **Scripted Logic** -- Write event handlers in RHAI, a lightweight embedded scripting language
- **Theming** -- Ship with built-in themes (Kanagawa, Catppuccin, Tokyo Night, Gruvbox, Nord) or define your own
- **Extensible** -- Add custom components and data sources via native plugins

## Quick Example

A minimal Nemo application consists of a single `app.hcl` file:

```hcl
app {
  window {
    title = "Hello Nemo"
  }

  theme {
    name = "kanagawa"
    mode = "dark"
  }
}

layout {
  type = "stack"

  component "greeting" {
    type = "label"
    text = "Hello, World!"
  }

  component "action" {
    type = "button"
    label = "Click Me"
    on_click = "on_button_click"
  }
}
```

Run it:

```bash
nemo --config app.hcl
```

## Project Structure

A typical Nemo application looks like this:

```
my-app/
  app.hcl            # Main configuration
  scripts/
    handlers.rhai     # Event handler scripts
  plugins/            # Optional native plugins
```

## Where to Go Next

- [Getting Started](public/getting-started.md) -- Install Nemo and build your first application
- [CLI Reference](public/cli.md) -- Command-line options and usage
- [Configuration](public/configuration.md) -- Complete HCL configuration reference
- [Architecture](public/architecture.md) -- System design and internal diagrams
- [Development](public/develop.md) -- Extending Nemo with scripts and plugins
