---
description: Scaffold a new native or WASM plugin for Nemo
---

# Add Nemo Plugin

Scaffold a new plugin for the Nemo framework. Supports both native (cdylib) and WASM (Component Model) plugin types.

## Arguments

The user should provide:
- **Plugin name** (e.g., `weather-data`, `custom-transform`) — kebab-case
- **Plugin type** — `native` or `wasm`
- **Description** — what the plugin does
- **Capabilities** — what it provides: DataSource, Transform, Action, Component, EventHandler

If not provided, ask the user.

## Steps

### 1. Read existing plugin patterns

Read the appropriate example:
- Native: `plugins/mock-data/src/lib.rs` and `plugins/mock-data/Cargo.toml`
- WASM: `crates/nemo-wasm-guest/src/lib.rs` and `plugins/mock-data-wasm/` if it exists
- Also read `crates/nemo-plugin-api/src/lib.rs` for the API types

### 2. Create plugin directory and Cargo.toml

Create `plugins/<name>/Cargo.toml`:

**For native plugins:**
```toml
[package]
name = "<name>-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nemo-plugin-api = { path = "../../crates/nemo-plugin-api" }
semver = { workspace = true }
```

**For WASM plugins:**
```toml
[package]
name = "<name>-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nemo-wasm-guest = { path = "../../crates/nemo-wasm-guest" }
wit-bindgen = { workspace = true }
```

### 3. Create plugin source

Create `plugins/<name>/src/lib.rs`:

**For native plugins:**
```rust
use nemo_plugin_api::*;

fn init(registrar: &mut dyn PluginRegistrar) {
    let ctx = registrar.context_arc();
    ctx.log(LogLevel::Info, "<Name> plugin initialized");

    // TODO: Implement plugin logic
    // For data sources: spawn background thread or use ctx.set_data()
    // For transforms: register with registrar.register_transform()
    // For actions: register with registrar.register_action()
}

declare_plugin!(
    PluginManifest::new(
        "<name>",
        "<Display Name>",
        semver::Version::new(0, 1, 0)
    )
    .with_description("<description>")
    .with_capability(Capability::DataSource("<name>".to_string())),
    init
);
```

**For WASM plugins:**
```rust
wit_bindgen::generate!({
    path: "../../crates/nemo-wasm/wit",
    world: "nemo-plugin",
});

struct MyPlugin;

impl Guest for MyPlugin {
    fn get_manifest() -> PluginManifest {
        PluginManifest {
            id: "<name>".to_string(),
            name: "<Display Name>".to_string(),
            version: "0.1.0".to_string(),
            description: "<description>".to_string(),
        }
    }

    fn init() {
        host_api::log(LogLevel::Info, "<Name> WASM plugin initialized");
    }

    fn tick() -> u64 {
        // Return ms until next tick, 0 = stop ticking
        // TODO: Implement tick logic
        0
    }
}

export_nemo_plugin!(MyPlugin);
```

### 4. Add to workspace

Edit the root `Cargo.toml` and add `"plugins/<name>"` to the `[workspace] members` array.

### 5. Verify

- Native: `cargo check -p <name>-plugin`
- WASM: `cargo component check -p <name>-plugin` (requires cargo-component)

## Important Notes

- Native plugins have full Rust power but no sandboxing — be careful with thread spawning
- WASM plugins are sandboxed but use tick-based scheduling (no threads)
- The `declare_plugin!` macro generates the required extern "C" entry points
- Plugin must be `crate-type = ["cdylib"]` — this is critical
- Native plugins are `.dylib` (macOS), `.so` (Linux), `.dll` (Windows)
- The `PluginContext` API is the same for both native and WASM
