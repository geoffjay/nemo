# Development Guide

This guide covers extending Nemo with RHAI scripts and native plugins.

## RHAI Scripting

RHAI is an embedded scripting language used for event handlers, data transformation, and application logic. Scripts are loaded from the directory specified in the `scripts` block of your configuration.

### Script Structure

A script file is a collection of functions. Each function that serves as an event handler takes two arguments:

```rhai
fn handler_name(component_id, event_data) {
    // component_id: the ID of the component that triggered the event
    // event_data: context string (e.g., "click", "true"/"false" for checkbox)
}
```

Scripts are identified by their filename without the `.rhai` extension. The file `scripts/handlers.rhai` produces script ID `"handlers"`.

### Available APIs

#### Data Access

| Function | Description |
|----------|-------------|
| `get_data(path)` | Read a value from the data repository |
| `set_data(path, value)` | Write a value to the data repository |

```rhai
fn on_refresh(component_id, event_data) {
    let ticker = get_data("ticker");
    if ticker != () {
        log_info("Current tick: " + ticker.to_string());
    }
}
```

#### Component Control

| Function | Description |
|----------|-------------|
| `set_component_text(id, text)` | Set a component's text/label property |
| `get_component_property(id, prop)` | Read any property from a component |
| `set_component_property(id, prop, value)` | Set any property on a component |
| `get_component_label(id)` | Get a component's label text |
| `show_component(id)` | Make a component visible |
| `hide_component(id)` | Hide a component |
| `enable_component(id)` | Enable a component |
| `disable_component(id)` | Disable a component |

```rhai
fn on_toggle(component_id, event_data) {
    let is_visible = get_component_property("detail_panel", "visible");
    if is_visible == "true" {
        hide_component("detail_panel");
    } else {
        show_component("detail_panel");
    }
}
```

#### Logging

| Function | Description |
|----------|-------------|
| `log_info(msg)` | Log at INFO level |
| `log_warn(msg)` | Log at WARN level |
| `log_error(msg)` | Log at ERROR level |
| `log_debug(msg)` | Log at DEBUG level |

#### Math

| Function | Description |
|----------|-------------|
| `abs(x)` | Absolute value (int or float) |
| `min(a, b)` | Minimum of two values |
| `max(a, b)` | Maximum of two values |
| `clamp(x, min, max)` | Clamp value to range |
| `round(f)` | Round to nearest integer |
| `floor(f)` | Round down |
| `ceil(f)` | Round up |

#### String

| Function | Description |
|----------|-------------|
| `len(s)` | String length |
| `substring(s, start, len)` | Extract substring |
| `trim(s)` | Remove whitespace |
| `split(s, sep)` | Split into array |
| `parse_float(s)` | Parse string to float |
| `to_string(v)` | Convert value to string |

### Sandbox Limits

RHAI scripts run in a sandboxed environment with the following limits:

| Limit | Value |
|-------|-------|
| Max operations | 100,000 |
| Max string size | 64 KB |
| Max array size | 10,000 elements |
| Max map size | 10,000 entries |
| Max call stack depth | 64 |
| File I/O | Disabled |
| Network access | Disabled |
| System calls | Disabled |

These limits prevent runaway scripts from affecting application performance.

### Complete Example: Calculator

This example shows a calculator's event handlers. The calculator stores state as component properties and uses helper functions for shared logic.

```rhai
// State helpers using component properties for storage
fn get_state(key) {
    let val = get_component_property("display_result", key);
    if val == () {
        if key == "current_value" { return "0"; }
        if key == "stored_value" { return "0"; }
        if key == "pending_operator" { return ""; }
        if key == "start_new_input" { return "true"; }
    }
    val
}

fn set_state(key, value) {
    set_component_property("display_result", key, value);
}

fn update_display(value) {
    set_component_text("display_result", value);
}

// Digit button handler (shared by all digit buttons)
fn on_digit(component_id, event_data) {
    let digit = get_component_label(component_id);
    let current = get_state("current_value");
    let new_input = get_state("start_new_input");

    if new_input == "true" {
        current = digit;
        set_state("start_new_input", "false");
    } else if current == "0" {
        current = digit;
    } else {
        current = current + digit;
    }

    set_state("current_value", current);
    update_display(current);
}

// Operator handler
fn on_operator(component_id, event_data) {
    let op = get_component_label(component_id);
    let current = get_state("current_value");

    set_state("stored_value", current);
    set_state("pending_operator", op);
    set_state("start_new_input", "true");
}

// Equals handler
fn on_equals(component_id, event_data) {
    let op = get_state("pending_operator");
    if op != "" {
        let a = parse_float(get_state("stored_value"));
        let b = parse_float(get_state("current_value"));
        let result = 0.0;

        if op == "+" { result = a + b; }
        else if op == "-" { result = a - b; }
        else if op == "*" { result = a * b; }
        else if op == "/" {
            if b == 0.0 {
                update_display("Error");
                return;
            }
            result = a / b;
        }

        let text = to_string(result);
        set_state("current_value", text);
        update_display(text);
        set_state("pending_operator", "");
    }
    set_state("start_new_input", "true");
}

fn on_clear(component_id, event_data) {
    set_state("current_value", "0");
    set_state("stored_value", "0");
    set_state("pending_operator", "");
    set_state("start_new_input", "true");
    update_display("0");
}
```

---

## Native Plugins

For capabilities beyond RHAI scripting, Nemo supports native plugins compiled as dynamic libraries.

### Plugin API

Plugins link against the `nemo-plugin-api` crate and implement the plugin interface.

#### Plugin Manifest

Every plugin declares a manifest describing its identity and capabilities:

```rust
use nemo_plugin_api::{PluginManifest, Capability};
use semver::Version;

fn manifest() -> PluginManifest {
    PluginManifest {
        name: "my-plugin".to_string(),
        version: Version::new(0, 1, 0),
        description: "A custom Nemo plugin".to_string(),
        capabilities: vec![
            Capability::Component,
            Capability::DataSource,
        ],
        permissions: Default::default(),
    }
}
```

#### Plugin Context

Plugins receive a `PluginContext` that provides runtime access:

| Method | Description |
|--------|-------------|
| `get_data(path)` | Read from the data repository |
| `set_data(path, value)` | Write to the data repository |
| `emit_event(type, payload)` | Emit an event on the event bus |
| `get_config(path)` | Read application configuration |
| `log(level, message)` | Write to the application log |
| `get_component_property(id, prop)` | Read a component property |
| `set_component_property(id, prop, val)` | Update a component property |

#### Plugin Value Types

The `PluginValue` enum is the FFI-safe equivalent of Nemo's internal `Value` type:

```rust
enum PluginValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<PluginValue>),
    Object(HashMap<String, PluginValue>),
}
```

### Plugin Loading

Place compiled plugin libraries in a directory and pass it to Nemo:

```bash
nemo --app-config app.hcl --extension-dirs ./plugins
```

Nemo scans for `.so` (Linux), `.dylib` (macOS), or `.dll` (Windows) files and loads them at startup.

---

## Custom Components (Rust)

Application developers who want to contribute built-in components or build Nemo from source can define new components using the `NemoComponent` derive macro.

### The `NemoComponent` Macro

The `#[derive(NemoComponent)]` macro generates a constructor that extracts properties from the HCL configuration.

```rust
use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct MyComponent {
    #[source]
    source: nemo_layout::BuiltComponent,

    #[property(default = "Hello")]
    title: String,

    #[property]
    subtitle: Option<String>,

    #[property]
    count: Option<i64>,

    #[children]
    children: Vec<AnyElement>,
}
```

#### Macro Attributes

| Attribute | Field Type | Description |
|-----------|-----------|-------------|
| `#[property]` | `Option<T>` | Extract an optional property from config |
| `#[property(default = "val")]` | `T` | Extract with a default value |
| `#[property(name = "key")]` | any | Use a custom property key instead of field name |
| `#[children]` | `Vec<AnyElement>` | Accept child elements |
| `#[source]` | `BuiltComponent` | Store the full component definition (for handler access) |

**Supported property types:** `String`, `i64`, `f64`, `bool`, and `Option<T>` variants of each.

#### Generated Code

The macro generates:

1. `pub fn new(component: BuiltComponent) -> Self` -- Extracts all `#[property]` fields from the component's properties map
2. `pub fn children(mut self, children: Vec<AnyElement>) -> Self` -- Builder method (only if `#[children]` is present)

Fields without a macro attribute receive `Default::default()`.

### Implementing `RenderOnce`

All Nemo components implement `RenderOnce` (not `Render`), making them stateless elements consumed on render:

```rust
impl RenderOnce for MyComponent {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut base = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.title);

        if let Some(subtitle) = self.subtitle {
            base = base.child(div().text_sm().child(subtitle));
        }

        base.children(self.children)
    }
}
```

### Registering a Component

To make a component available in HCL configurations, register it in the component registry and add a match arm in `app.rs`:

```rust
// In render_component():
"my_component" => {
    let children = self.render_children(component, entity_id, window, cx);
    MyComponent::new(component.clone())
        .children(children)
        .into_any_element()
}
```

---

## Data Flow Architecture

Understanding the data flow helps when building complex applications:

```
Data Sources (timer, HTTP, MQTT, ...)
    |
    v
DataFlowEngine -- stores in DataRepository
    |
    v
Dirty flag set (atomic bool)
    |
    v  (every 50ms)
App poll -- apply_pending_data_updates()
    |
    v
BindingManager -- propagates through bindings
    |
    v
Component properties updated
    |
    v
GPUI re-renders affected components
```

### Key Points

- Data sources run on a Tokio async runtime, separate from the GPUI render thread
- The UI polls for data changes every 50ms via an atomic dirty flag
- Bindings track their last value to avoid redundant updates
- RHAI handlers can both read and write data, triggering new binding updates
- Component state (like input values) is managed separately from data source state

---

## Project Structure

The Nemo codebase is organized as a Cargo workspace:

| Crate | Purpose |
|-------|---------|
| `nemo` | Application shell, GPUI integration, components |
| `nemo-config` | HCL parsing, validation, expression resolution |
| `nemo-layout` | Component tree building, binding management |
| `nemo-data` | Data sources, repository, binding system |
| `nemo-registry` | Component/source/transform catalog |
| `nemo-events` | Event bus (typed pub/sub) |
| `nemo-extension` | RHAI engine, plugin host |
| `nemo-integration` | HTTP, WebSocket, MQTT, Redis, NATS clients |
| `nemo-plugin-api` | Stable plugin author interface |
| `nemo-macros` | `#[derive(NemoComponent)]` proc macro |

### Building from Source

```bash
git clone https://github.com/geoffjay/nemo.git
cd nemo
cargo build --release
```

### Running Tests

```bash
cargo test --workspace
```

### Running Examples

```bash
cargo run -- --app-config examples/basic/app.hcl
cargo run -- --app-config examples/calculator/app.hcl
cargo run -- --app-config examples/components/app.hcl
cargo run -- --app-config examples/data-binding/app.hcl
```
