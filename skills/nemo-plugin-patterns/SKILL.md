---
name: nemo-plugin-patterns
description: Native and WASM plugin development patterns for Nemo including PluginContext API, plugin lifecycle, manifest configuration, and build/deploy workflows. Use when creating or debugging Nemo plugins.
---

# Nemo Plugin Patterns

Use this skill when creating, modifying, or debugging Nemo plugins (native or WASM).

## Three-Tier Extension Model

```
XML (declarative) → Rhai scripts (lightweight logic) → Native/WASM plugins (full power)
```

| Tier | Sandboxing | Use Case |
|------|-----------|----------|
| **Rhai scripts** | Operation limits, no I/O | Event handlers, simple transforms, UI logic |
| **WASM plugins** | Memory isolation, capability-based | Data sources, complex transforms, sandboxed logic |
| **Native plugins** | None (Rust safety only) | Full system access, performance-critical, background threads |

## PluginContext API

Available to all plugins at runtime:

```rust
trait PluginContext: Send + Sync {
    fn get_data(&self, path: &str) -> Option<PluginValue>;
    fn set_data(&self, path: &str, value: PluginValue) -> Result<(), PluginError>;
    fn emit_event(&self, event_type: &str, payload: PluginValue);
    fn get_config(&self, path: &str) -> Option<PluginValue>;
    fn log(&self, level: LogLevel, message: &str);
    fn get_component_property(&self, component_id: &str, property: &str) -> Option<PluginValue>;
    fn set_component_property(&self, component_id: &str, property: &str, value: PluginValue) -> Result<(), PluginError>;
}
```

### PluginValue Types
```rust
enum PluginValue {
    Null, Bool(bool), Integer(i64), Float(f64),
    String(String), Array(Vec<PluginValue>),
    Object(HashMap<String, PluginValue>),
}
```

## Native Plugin Pattern

### Cargo.toml
```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # CRITICAL — must be cdylib

[dependencies]
nemo-plugin-api = { path = "../../crates/nemo-plugin-api" }
semver = { workspace = true }
```

### src/lib.rs
```rust
use nemo_plugin_api::*;

fn init(registrar: &mut dyn PluginRegistrar) {
    let ctx = registrar.context_arc();
    ctx.log(LogLevel::Info, "Plugin initialized");

    // Register capabilities
    // registrar.register_data_source("name", schema);
    // registrar.register_transform("name", schema);
    // registrar.register_action("name", schema);

    // For background data production:
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let _ = ctx.set_data("my.value", PluginValue::Float(42.0));
        }
    });
}

declare_plugin!(
    PluginManifest::new("my-plugin", "My Plugin", semver::Version::new(0, 1, 0))
        .with_description("What it does")
        .with_capability(Capability::DataSource("my".to_string())),
    init
);
```

### Build
```bash
cargo build -p my-plugin --release
# Output: target/release/libmy_plugin.dylib (macOS) / .so (Linux) / .dll (Windows)
```

### Lifecycle
1. `ExtensionLoader` scans plugin directories for `.dylib`/`.so`/`.dll`
2. `PluginHost` calls `libloading::Library::new(path)`
3. Host calls `nemo_plugin_manifest()` → gets manifest
4. Host calls `nemo_plugin_entry(registrar)` → plugin registers + starts
5. Plugin uses `PluginContext` via `registrar.context_arc()` throughout lifetime

## WASM Plugin Pattern

### Cargo.toml
```toml
[package]
name = "my-wasm-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # CRITICAL — must be cdylib

[dependencies]
nemo-wasm-guest = { path = "../../crates/nemo-wasm-guest" }
wit-bindgen = { workspace = true }
```

### src/lib.rs
```rust
wit_bindgen::generate!({
    path: "../../crates/nemo-wasm/wit",
    world: "nemo-plugin",
});

struct MyPlugin;

impl Guest for MyPlugin {
    fn get_manifest() -> PluginManifest {
        PluginManifest {
            id: "my-wasm-plugin".to_string(),
            name: "My WASM Plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "What it does".to_string(),
        }
    }

    fn init() {
        host_api::log(LogLevel::Info, "WASM plugin initialized");
    }

    fn tick() -> u64 {
        // Do periodic work here
        let value = PluginValue::Float(42.0);
        let _ = host_api::set_data("my.value", value);

        2000  // Return ms until next tick. 0 = stop ticking.
    }
}

export_nemo_plugin!(MyPlugin);
```

### Build
```bash
cargo component build -p my-wasm-plugin --release
# Output: target/wasm32-wasip1/release/my_wasm_plugin.wasm
```

### Lifecycle
1. `ExtensionLoader` scans for `.wasm` files
2. `WasmHost` creates `Component::from_binary()` via wasmtime
3. Host calls `get-manifest()` export
4. Host calls `init()` export
5. Host calls `tick()` at plugin-specified intervals
6. Plugin returns ms until next tick (0 = stop)

### Key Difference: Tick-Based vs Threaded
- **WASM**: No threads. Host drives execution via `tick()`. Return interval in ms.
- **Native**: Full thread access. Can `std::thread::spawn` for background work.

## Rhai Script Pattern

### Script File
```rhai
// scripts/handlers.rhai

fn on_button_click(component_id, event_data) {
    log_info("Clicked: " + component_id);
    let value = get_data("some.path");
    set_component_text("label_id", "Updated: " + value);
}

fn on_input_change(component_id, event_data) {
    set_data("search.query", event_data);
}
```

### Available Rhai Functions

**Data:** `get_data(path)`, `set_data(path, value)`, `get_config(path)`
**Components:** `get_component_property(id, prop)`, `set_component_property(id, prop, value)`, `get_component_label(id)`, `get_component_text(id)`, `set_component_text(id, text)`, `set_component_label(id, label)`
**Math:** `abs`, `min`, `max`, `clamp`, `floor`, `ceil`, `round`, `sqrt`, `pow`
**Strings:** `trim`, `to_upper`, `to_lower`, `starts_with`, `ends_with`, `contains`, `replace`
**Type conversion:** `parse_float`, `parse_int`, `to_string`, `to_int`, `to_float`
**Logging:** `log_debug`, `log_info`, `log_warn`, `log_error`, `print`

### Sandboxing Limits
- Max operations: 100,000
- Max string size: 64KB
- Max array/map size: 10,000
- Max call stack depth: 64
- No `eval`, no file I/O, no network access

## Plugin Capabilities

```rust
enum Capability {
    Component(String),     // Register a new component type
    DataSource(String),    // Register a data source
    Transform(String),     // Register a data transform
    Action(String),        // Register an action
    EventHandler(String),  // Register an event handler
}
```

## Plugin Permissions

```rust
struct PluginPermissions {
    network: bool,          // Can make network requests
    filesystem: bool,       // Can access filesystem
    system: bool,           // Can access system resources
    data_read: bool,        // Can read from DataRepository
    data_write: bool,       // Can write to DataRepository
    component_read: bool,   // Can read component properties
    component_write: bool,  // Can modify component properties
    event_emit: bool,       // Can emit events
}
```

## XML Plugin Configuration

```xml
<nemo>
  <plugin name="my-plugin" path="./plugins/my-plugin" />
  <!-- or for WASM: -->
  <plugin name="my-wasm-plugin" path="./plugins/my-wasm-plugin.wasm" />
</nemo>
```

## Workspace Setup

Add to root `Cargo.toml`:
```toml
[workspace]
members = [
    # ...existing members...
    "plugins/my-plugin",
]
```

## Debugging Plugins

1. **Plugin not loading** → Check `crate-type = ["cdylib"]`, verify file extension matches platform
2. **Manifest errors** → Check `declare_plugin!` macro or WIT `get-manifest` return
3. **Context API failures** → Check that `PluginContext` methods are called correctly
4. **WASM type issues** → Complex types (arrays/objects) use JSON serialization in WIT bridge
5. **Extension discovery** → Check `nemo-extension/src/loader.rs` for path scanning logic
