# Plugins

Nemo supports two plugin architectures: **native plugins** compiled as dynamic libraries, and **WASM plugins** compiled as WebAssembly Component Model binaries. Both share the same runtime API surface for reading/writing data, emitting events, and controlling components. Native plugins have full OS access and can spawn threads; WASM plugins run in a sandboxed single-threaded environment with a tick-based execution model.

## Native Plugins

Native plugins are compiled as dynamic libraries and loaded into the Nemo process. They are the right choice when you need to:

- **Provide custom data sources** that produce live data for UI bindings (e.g., hardware sensors, proprietary APIs, database queries)
- **Run background processing** on a separate thread (e.g., polling an internal service, generating computed values)
- **Use native Rust libraries** that aren't available in the RHAI sandbox
- **Register custom components, transforms, or actions** that extend Nemo's built-in capabilities

Plugins run as native code inside the Nemo process, so they have full access to the Rust ecosystem while still interacting with Nemo through a structured API.

### How Native Plugins Work

```
┌───────────────────────────────────────────────────────┐
│                    Nemo Application                   │
│                                                       │
│  ┌──────────────┐    ┌─────────────────────────────┐  │
│  │ Plugin Host  │───▶│ ExtensionLoader             │  │
│  │              │    │  • Scans plugin directories │  │
│  │  • load()    │    │  • Discovers .dylib/.so     │  │
│  │  • unload()  │    └─────────────────────────────┘  │
│  └──────┬───────┘                                     │
│         │ dlopen                                      │
│         ▼                                             │
│  ┌───────────────┐    ┌────────────────────────────┐  │
│  │ Your Plugin   │───▶│ PluginRegistrar            │  │
│  │  (.dylib/.so) │    │  • register_component()    │  │
│  │               │    │  • register_data_source()  │  │
│  │  nemo_plugin_ │    │  • register_transform()    │  │
│  │  manifest()   │    │  • register_action()       │  │
│  │               │    └────────────────────────────┘  │
│  │  nemo_plugin_ │    ┌─────────────────────────────┐ │
│  │  entry()      │───▶│ PluginContext               │ │
│  └───────────────┘    │  • get_data() / set_data()  │ │
│                       │  • emit_event()             │ │
│                       │  • get/set_component_prop() │ │
│                       │  • log()                    │ │
│                       └─────────────────────────────┘ │
└───────────────────────────────────────────────────────┘
```

At startup, Nemo scans directories passed via `--extension-dirs` for native libraries (`.dylib` on macOS, `.so` on Linux, `.dll` on Windows). For each library found, it:

1. Calls `nemo_plugin_manifest()` to read the plugin's identity and capabilities
2. Calls `nemo_plugin_entry()` with a `PluginRegistrar` to let the plugin register its features
3. Stores the loaded plugin for the application's lifetime

### Building a Native Plugin: Step by Step

This walkthrough creates a plugin that provides simulated sensor data. This is based on the `mock-data` plugin in `examples/data-binding/plugins/`.

#### 1. Create the Crate

```bash
mkdir -p my-app/plugins/my-sensor
cd my-app/plugins/my-sensor
cargo init --lib
```

#### 2. Configure `Cargo.toml`

The key requirement is `crate-type = ["cdylib"]`, which tells Cargo to produce a dynamic library instead of a Rust library.

```toml
[package]
name = "my-sensor-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nemo-plugin-api = { path = "../../../crates/nemo-plugin-api" }
semver = "1"
```

If your project is inside the Nemo workspace, use a relative path to `nemo-plugin-api`. Otherwise, you can publish it or use a git dependency.

#### 3. Write the Plugin

Edit `src/lib.rs`:

```rust
use nemo_plugin_api::*;

fn init(registrar: &mut dyn PluginRegistrar) {
    // Get an Arc<dyn PluginContext> for use in background threads
    let ctx = registrar.context_arc();

    // Set initial data values
    let _ = ctx.set_data("sensor.temperature", PluginValue::Float(22.5));
    let _ = ctx.set_data("sensor.humidity", PluginValue::Float(45.0));
    let _ = ctx.set_data("sensor.counter", PluginValue::Integer(0));

    ctx.log(LogLevel::Info, "Sensor plugin initialized");

    // Spawn a background thread to update values periodically
    std::thread::spawn(move || {
        let mut counter: i64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            counter += 1;

            // Sine wave temperature (20-25°C)
            let temp = 22.5 + 2.5 * (counter as f64 * 0.1).sin();
            let _ = ctx.set_data("sensor.temperature", PluginValue::Float(temp));

            // Humidity with cosine variation
            let humidity = 50.0 + 10.0 * (counter as f64 * 0.07).cos();
            let _ = ctx.set_data("sensor.humidity", PluginValue::Float(humidity));

            // Incrementing counter
            let _ = ctx.set_data("sensor.counter", PluginValue::Integer(counter));
        }
    });
}

// Declare the plugin entry point
declare_plugin!(
    PluginManifest::new(
        "my-sensor",                        // Unique plugin ID
        "My Sensor Plugin",                 // Display name
        semver::Version::new(0, 1, 0)       // Version
    )
    .with_description("Provides simulated sensor data")
    .with_capability(Capability::DataSource("sensor".to_string())),
    init
);
```

#### 4. Build the Plugin

```bash
cargo build -p my-sensor-plugin
```

This produces `target/debug/libmy_sensor_plugin.dylib` (macOS) or `target/debug/libmy_sensor_plugin.so` (Linux).

#### 5. Load the Plugin

Pass the plugin directory to Nemo:

```bash
nemo --app-config app.hcl --extension-dirs ./target/debug
```

Nemo discovers and loads the library, calls the manifest and entry functions, and the plugin begins publishing data.

#### 6. Bind Plugin Data to UI

In your `app.hcl`, bind components to the data paths the plugin sets:

```hcl
layout {
  type = "stack"

  component "temp_display" {
    type = "label"
    text = "Temperature: waiting..."
    bind_text = "sensor.temperature"
  }

  component "humidity_display" {
    type = "label"
    text = "Humidity: waiting..."
    bind_text = "sensor.humidity"
  }

  component "counter_display" {
    type = "label"
    text = "Counter: 0"
    bind_text = "sensor.counter"
  }
}
```

The `bind_text` shorthand creates a one-way binding from the plugin's data path to the label's `text` property. As the plugin updates values via `set_data()`, the UI automatically refreshes.

---

### Plugin API Reference

#### `declare_plugin!` Macro

The entry point for every plugin. It generates the two `extern "C"` functions that Nemo looks for when loading a library:

```rust
declare_plugin!(
    PluginManifest::new("id", "Name", semver::Version::new(0, 1, 0))
        .with_description("What this plugin does")
        .with_capability(Capability::DataSource("prefix".to_string())),
    init_function
);
```

This generates:

- `nemo_plugin_manifest() -> PluginManifest` — Returns the plugin's identity
- `nemo_plugin_entry(&mut dyn PluginRegistrar)` — Called to initialize the plugin

#### `PluginManifest`

Describes the plugin's identity and capabilities.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique identifier |
| `name` | `String` | Display name |
| `version` | `Version` | Semantic version |
| `description` | `String` | Human-readable description |
| `capabilities` | `Vec<Capability>` | What the plugin provides |
| `permissions` | `PluginPermissions` | Requested permissions |

**Builder methods:**

- `.with_description(text)` — Set the description
- `.with_capability(cap)` — Add a capability

#### `Capability`

What a plugin provides:

| Variant | Description |
|---------|-------------|
| `Capability::Component(name)` | Registers a custom UI component |
| `Capability::DataSource(name)` | Provides a data source |
| `Capability::Transform(name)` | Provides a data transform |
| `Capability::Action(name)` | Provides a custom action |
| `Capability::EventHandler(name)` | Provides an event handler |

#### `PluginRegistrar` Trait

Passed to the init function. Used to register plugin features and access the runtime context.

| Method | Description |
|--------|-------------|
| `register_component(name, schema)` | Register a UI component with its schema |
| `register_data_source(name, schema)` | Register a data source |
| `register_transform(name, schema)` | Register a data transform |
| `register_action(name, schema)` | Register an action |
| `context()` | Get a `&dyn PluginContext` reference |
| `context_arc()` | Get an `Arc<dyn PluginContext>` for use in threads |

#### `PluginContext` Trait

The runtime API available to plugins. The `context_arc()` method returns an `Arc<dyn PluginContext>` that is `Send + Sync`, safe to move into background threads.

| Method | Description |
|--------|-------------|
| `get_data(path)` | Read a value from the data repository |
| `set_data(path, value)` | Write a value (triggers binding updates) |
| `emit_event(type, payload)` | Emit an event on the event bus |
| `get_config(path)` | Read application configuration |
| `log(level, message)` | Write to the application log |
| `get_component_property(id, prop)` | Read a component property |
| `set_component_property(id, prop, val)` | Update a component property |

#### `PluginValue`

FFI-safe value type used for all data exchange between plugins and Nemo:

```rust
enum PluginValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<PluginValue>),
    Object(HashMap<String, PluginValue>),
}
```

#### `LogLevel`

| Level | Description |
|-------|-------------|
| `LogLevel::Debug` | Detailed diagnostic information |
| `LogLevel::Info` | General informational messages |
| `LogLevel::Warn` | Warning conditions |
| `LogLevel::Error` | Error conditions |

---

### Plugin Permissions

Plugins can declare the permissions they need via `PluginPermissions`:

```rust
PluginPermissions {
    network: bool,        // Can make network requests
    filesystem: bool,     // Can access the filesystem
    subprocess: bool,     // Can spawn subprocesses
    data_paths: Vec<String>,   // Allowed data paths
    event_types: Vec<String>,  // Allowed event types
}
```

By default, all permissions are `false` / empty. Set them on the manifest if your plugin requires specific access.

---

### Tips

- **Data path conventions:** Use a dotted prefix matching your plugin ID (e.g., `sensor.temperature`) to avoid collisions with other plugins or built-in data sources.
- **Thread safety:** `PluginContext` is `Send + Sync`. Use `context_arc()` to get an `Arc` you can move into `std::thread::spawn`.
- **Error handling:** `set_data()` returns `Result<(), PluginError>`. In background threads, log errors rather than panicking.
- **Hot reload:** Plugins are loaded once at startup. To reload, restart the application.
- **Platform libraries:** The compiled library extension varies by platform: `.dylib` (macOS), `.so` (Linux), `.dll` (Windows). Nemo detects the correct extension automatically and strips the `lib` prefix on Unix.

---

## WASM Plugins

WASM plugins use the [WebAssembly Component Model](https://component-model.bytecodealliance.org/) to run plugin code in a sandboxed environment. They share the same runtime API as native plugins (data access, event emission, component control) but execute inside a Wasmtime sandbox with no direct OS access.

WASM plugins are the right choice when you need:

- **Sandboxed execution** — plugins cannot access the filesystem, network, or spawn threads
- **Language flexibility** — any language that compiles to WASM components can be used (Rust examples below)
- **Safe distribution** — WASM binaries are portable and can be loaded without trust concerns

### How WASM Plugins Work

WASM plugins communicate with Nemo through a WIT (WebAssembly Interface Type) contract. The plugin exports three functions and imports a host API:

```
┌─────────────────────────────────────────────────────────┐
│                    Nemo Application                      │
│                                                          │
│  ┌──────────────┐    ┌────────────────────────────────┐  │
│  │  WasmHost    │    │ Wasmtime Engine                │  │
│  │              │───▶│  • Component Model enabled     │  │
│  │  • load()    │    │  • WASI support                │  │
│  │  • unload()  │    └────────────────────────────────┘  │
│  │  • tick_all()│                                        │
│  └──────┬───────┘                                        │
│         │ instantiate                                    │
│         ▼                                                │
│  ┌───────────────┐    ┌────────────────────────────────┐ │
│  │ WASM Plugin   │───▶│ Exported functions             │ │
│  │  (.wasm)      │    │  • get_manifest() -> manifest  │ │
│  │               │    │  • init()                      │ │
│  │               │    │  • tick() -> u64 (ms interval) │ │
│  └───────────────┘    └────────────────────────────────┘ │
│         │                                                │
│         │ imports                                        │
│         ▼                                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Host API (same surface as native PluginContext)    │  │
│  │  • get_data() / set_data()                        │  │
│  │  • emit_event()                                   │  │
│  │  • get/set_component_property()                   │  │
│  │  • log()                                          │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

Unlike native plugins which spawn background threads, WASM plugins use a **tick-based model**:

1. Nemo calls `init()` once after loading the plugin
2. Nemo calls `tick()` periodically; the return value is the number of milliseconds until the next tick
3. Each `tick()` call can read/write data, emit events, and update component properties via the host API

### The WIT Interface

The plugin contract is defined in a WIT file (`nemo-plugin.wit`):

```wit
package nemo:plugin@0.1.0;

interface types {
    enum log-level { debug, info, warn, error }

    variant plugin-value {
        null,
        bool-val(bool),
        integer-val(s64),
        float-val(f64),
        string-val(string),
        json-val(string),       // JSON-encoded arrays/objects
    }

    record plugin-manifest {
        id: string,
        name: string,
        version: string,
        description: string,
        author: option<string>,
    }
}

interface host-api {
    use types.{log-level, plugin-value};

    get-data: func(path: string) -> option<plugin-value>;
    set-data: func(path: string, value: plugin-value) -> result<_, string>;
    emit-event: func(event-type: string, payload: plugin-value);
    get-config: func(path: string) -> option<plugin-value>;
    log: func(level: log-level, message: string);
    get-component-property: func(id: string, prop: string) -> option<plugin-value>;
    set-component-property: func(id: string, prop: string, value: plugin-value) -> result<_, string>;
}

world nemo-plugin {
    use types.{plugin-manifest};
    import host-api;

    export get-manifest: func() -> plugin-manifest;
    export init: func();
    export tick: func() -> u64;
}
```

The `plugin-value` variant uses `json-val` for complex types (arrays and objects) since WIT does not support recursive types. The host automatically handles JSON serialization/deserialization.

### Building a WASM Plugin: Step by Step

#### 1. Prerequisites

Install the WASM Component Model target:

```bash
rustup target add wasm32-wasip2
```

#### 2. Create the Crate

```bash
mkdir -p my-app/plugins/my-wasm-plugin
cd my-app/plugins/my-wasm-plugin
cargo init --lib
```

#### 3. Configure `Cargo.toml`

```toml
[package]
name = "my-wasm-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nemo-wasm-guest = { path = "../../crates/nemo-wasm-guest" }
wit-bindgen = "0.41"
```

The `nemo-wasm-guest` crate re-exports `wit-bindgen` and provides the WIT file path.

#### 4. Write the Plugin

Edit `src/lib.rs`:

```rust
wit_bindgen::generate!({
    path: "../../crates/nemo-wasm/wit/nemo-plugin.wit",
    world: "nemo-plugin",
});

use nemo::plugin::host_api;
use nemo::plugin::types::{LogLevel, PluginValue};

struct MyPlugin;

static mut COUNTER: i64 = 0;

impl Guest for MyPlugin {
    fn get_manifest() -> PluginManifest {
        PluginManifest {
            id: "my-wasm-plugin".into(),
            name: "My WASM Plugin".into(),
            version: "0.1.0".into(),
            description: "A WASM plugin example".into(),
            author: None,
        }
    }

    fn init() {
        let _ = host_api::set_data("myplugin.value", &PluginValue::FloatVal(0.0));
        host_api::log(LogLevel::Info, "WASM plugin initialized");
    }

    fn tick() -> u64 {
        // Safety: WASM is single-threaded, no concurrent access.
        let counter = unsafe {
            COUNTER += 1;
            COUNTER
        };

        let value = (counter as f64 * 0.1).sin();
        let _ = host_api::set_data("myplugin.value", &PluginValue::FloatVal(value));

        // Return milliseconds until next tick
        2000
    }
}

export!(MyPlugin);
```

Key differences from native plugins:

- Use `wit_bindgen::generate!` to import the WIT bindings
- Implement the `Guest` trait instead of using `declare_plugin!`
- Use `export!(MyPlugin)` to register the implementation
- Host API functions are called as free functions (`host_api::set_data(...)`) rather than through a context object
- `static mut` is safe because WASM execution is single-threaded
- `tick()` returns the interval in milliseconds until it should be called again

#### 5. Build the Plugin

```bash
cargo build -p my-wasm-plugin --target wasm32-wasip2
```

This produces `target/wasm32-wasip2/debug/my_wasm_plugin.wasm`.

#### 6. Load the Plugin

Pass the directory containing the `.wasm` file to Nemo:

```bash
nemo --app-config app.hcl --extension-dirs ./target/wasm32-wasip2/debug
```

Nemo discovers `.wasm` files alongside native libraries and loads them via Wasmtime.

### WASM Plugin Value Types

The `PluginValue` variant maps to Nemo's internal value type:

| WIT Variant | Rust Usage | Description |
|-------------|-----------|-------------|
| `null` | `PluginValue::Null` | Null value |
| `bool-val(bool)` | `PluginValue::BoolVal(true)` | Boolean |
| `integer-val(s64)` | `PluginValue::IntegerVal(42)` | 64-bit signed integer |
| `float-val(f64)` | `PluginValue::FloatVal(3.14)` | 64-bit float |
| `string-val(string)` | `PluginValue::StringVal("hello".into())` | String |
| `json-val(string)` | `PluginValue::JsonVal(json_string)` | JSON-encoded arrays or objects |

For complex values like arrays or objects, encode them as JSON strings using the `json-val` variant. The host automatically converts between JSON and Nemo's internal `Value` type.

### Tips

- **Tick interval:** Return `0` from `tick()` to disable further ticking. Return a positive value for the delay in milliseconds.
- **No threads:** WASM plugins cannot spawn threads. Use the tick model for periodic work.
- **Shared API:** The host API (`get_data`, `set_data`, `emit_event`, `log`, `get/set_component_property`) is identical to the native plugin API.
- **Debugging:** Use `host_api::log(LogLevel::Debug, "message")` for diagnostics. Logs appear in the Nemo console output.
- **Data paths:** Follow the same dotted-prefix convention as native plugins (e.g., `myplugin.temperature`).

---

## Choosing Between Native and WASM Plugins

| | Native Plugins | WASM Plugins |
|---|---|---|
| **Execution** | In-process, full OS access | Sandboxed via Wasmtime |
| **Threading** | Can spawn background threads | Single-threaded, tick-based |
| **Performance** | Native speed | Near-native (WASM overhead) |
| **Safety** | Full trust required | Sandboxed, safe to distribute |
| **Portability** | Platform-specific binary | Single `.wasm` binary runs anywhere |
| **Dependencies** | Full Rust ecosystem | WASM-compatible crates only |
| **API** | `PluginContext` trait object | WIT-generated free functions |
