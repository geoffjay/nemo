# Nemo

Nemo is a configuration-driven desktop application framework. Define your UI layout, data sources, and event handlers in `.nemo` single-file components -- Nemo handles rendering, data flow, and state management.

Built on [GPUI](https://gpui.rs), Nemo produces native, GPU-accelerated desktop applications from declarative configuration.

## What Nemo Does

- **Declarative UI** -- Define component trees in XML instead of writing rendering code
- **Live Data Binding** -- Connect HTTP APIs, MQTT brokers, Redis, NATS, WebSockets, and timers directly to UI components
- **Scripted Logic** -- Write event handlers in RHAI, a lightweight embedded scripting language
- **Theming** -- Ship with built-in themes (Ayu, Kanagawa, Catppuccin, Tokyo Night, Gruvbox, Nord) or define your own
- **Extensible** -- Add custom components and data sources via native plugins

## Quick Example

A minimal Nemo application is a single `app.nemo` file:

```xml
<app title="Hello Nemo">
  <window title="Hello Nemo" width="480" height="240" />
  <theme name="kanagawa" mode="dark" />
</app>

<script src="./scripts" />

<template name="app">
  <stack id="main" direction="vertical" spacing="8" padding="16">
    <label id="greeting" text="Hello, World!" size="xl" />
    <button id="action" label="Click Me" on-click="on_button_click" />
  </stack>
</template>
```

The `<template name="app">` block is the UI tree; `<script src="./scripts" />`
loads the RHAI handler directory. Run it:

```bash
nemo --app-config app.nemo
```

## Project Structure

A typical Nemo application looks like this:

```
my-app/
  app.nemo            # Application entry (single-file component)
  scripts/
    handlers.rhai     # Event handler scripts
  plugins/            # Optional native plugins
```

## Where to Go Next

- [Getting Started](public/getting-started.md) -- Install Nemo and build your first application
- [CLI Reference](public/cli.md) -- Command-line options and usage
- [Configuration](public/configuration.md) -- Complete XML configuration reference
- [Plugins](public/plugins.md) -- Extending Nemo with native plugins
- [Architecture](public/architecture.md) -- System design and internal diagrams
- [Development](public/develop.md) -- Extending Nemo with scripts and plugins
- [Packaging & Distribution](public/packaging.md) -- How Nemo is built, packaged, and released
