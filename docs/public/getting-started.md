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

Create a directory for your application and add an `app.xml` file:

```bash
mkdir my-app && cd my-app
```

### 1. Define the Application

Create `app.xml`:

```xml
<nemo>
  <app>
    <window title="My First App" width="800" height="600">
      <header-bar theme-toggle="true" />
    </window>
    <theme name="kanagawa" mode="dark" />
  </app>

  <script src="./scripts" />

  <layout type="stack">
    <stack id="main" direction="vertical" margin="16" spacing="8">
      <label id="title" text="My First Nemo App" size="xl" />

      <panel id="content" padding="16">
        <label id="greeting" text="Click the button below to see scripting in action." />
      </panel>

      <button id="action" label="Say Hello" variant="primary" on-click="on_say_hello" />

      <label id="output_label" text="" />
    </stack>
  </layout>
</nemo>
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
nemo --app-config app.xml
```

You should see a window with a title, a panel, a button, and a label. Clicking the button updates the label text via the RHAI script.

## Core Concepts

### Configuration (XML)

Nemo applications are defined in XML files. The configuration contains up to six top-level elements:

| Block | Purpose |
|-------|---------|
| `variable` | Reusable variables with type and default value |
| `app` | Window settings, theme selection |
| `scripts` | Path to RHAI script files |
| `templates` | Reusable component templates to reduce duplication |
| `data` | Data source and sink definitions |
| `layout` | UI component tree |

### Components

Components are the building blocks of your UI. Each component has a `type` and a set of properties specific to that type.

```xml
<button id="my_button" label="Click Me" variant="primary" on-click="handle_click" />
```

Available component types:

| Category | Types |
|----------|-------|
| Layout | `stack`, `panel`, `collapsible`, `accordion` |
| Display | `label`, `text`, `icon`, `image`, `progress`, `spinner`, `avatar`, `badge`, `tag`, `alert` |
| Input | `button`, `input`, `checkbox`, `select`, `radio`, `slider`, `switch`, `toggle`, `dropdown_button` |
| Data | `list`, `table`, `tree` |
| Charts | `line_chart`, `bar_chart`, `area_chart`, `pie_chart`, `candlestick_chart` |
| Overlay | `modal`, `tooltip`, `tabs`, `notification` |

### Data Sources

Data sources connect external systems to your UI. When a source emits new data, bound components update automatically.

```xml
<data>
  <source name="ticker" type="timer" interval="1" />
</data>
```

Supported source types: `timer`, `http`, `websocket`, `mqtt`, `redis`, `nats`, `file`.

### Data Binding

Bind data source output directly to component properties:

```xml
<label id="counter" text="Waiting...">
  <binding source="data.ticker" target="text" transform="tick" />
</label>
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

```xml
<button id="btn" label="Refresh" on-click="handle_click" />
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

```xml
<app>
  <theme name="catppuccin" mode="dark" />
</app>
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
nemo --app-config examples/basic/app.xml
nemo --app-config examples/calculator/app.xml
```

## Next Steps

- [CLI Reference](cli.md) -- All command-line options
- [Configuration Reference](configuration.md) -- Every block, attribute, and component type
- [Plugins](plugins.md) -- Extending Nemo with native plugins
- [Development Guide](develop.md) -- Writing scripts and building plugins
