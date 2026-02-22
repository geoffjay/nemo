[![Documentation][docsrs-badge]][docsrs-url]

[docsrs-badge]: https://docs.rs/nemo-wasm-guest/badge.svg
[docsrs-url]: https://docs.rs/nemo-wasm-guest

# nemo-wasm-guest

WASM guest SDK for building Nemo plugins as WebAssembly components.

This crate ships the WIT interface definition and re-exports `wit-bindgen` so
plugin authors have everything they need to implement a Nemo WASM plugin.

## Quick Start

```rust
wit_bindgen::generate!({
    path: "wit/nemo-plugin.wit",  // shipped with this crate
    world: "nemo-plugin",
});

use nemo::plugin::host_api;
use nemo::plugin::types::{LogLevel, PluginManifest, PluginValue};

struct MyPlugin;

impl Guest for MyPlugin {
    fn get_manifest() -> PluginManifest {
        PluginManifest {
            id: "my-plugin".into(),
            name: "My Plugin".into(),
            version: "0.1.0".into(),
            description: "A sample plugin".into(),
            author: Some("Author".into()),
        }
    }

    fn init() {
        host_api::log(LogLevel::Info, "Plugin initialized");
    }

    fn tick() -> u64 {
        0
    }
}

export!(MyPlugin);
```

## Locating the WIT file

This crate sets the `DEP_NEMO_WASM_GUEST_WIT_DIR` environment variable via
Cargo `links` metadata, so build scripts in consuming crates can locate the
bundled WIT directory automatically.

## License

MIT OR Apache-2.0
