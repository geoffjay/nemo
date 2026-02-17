# nemo-plugin-api

Stable API interface for Nemo native plugins. This crate defines the boundary
between the Nemo host and dynamically loaded plugin libraries.

## Overview

Plugins are compiled as `cdylib` crates that export two symbols via the
[`declare_plugin!`] macro:

- `nemo_plugin_manifest` — returns a `PluginManifest` describing the plugin
- `nemo_plugin_entry` — called with a `PluginRegistrar` to register components,
  data sources, transforms, actions, and templates

## Quick Start

```rust
use nemo_plugin_api::*;
use semver::Version;

fn init(registrar: &mut dyn PluginRegistrar) {
    registrar.register_component(
        "my_counter",
        ComponentSchema::new("my_counter")
            .with_description("A counter component")
            .with_property("initial", PropertySchema::integer())
            .require("initial"),
    );
}

declare_plugin!(
    PluginManifest::new("my-plugin", "My Plugin", Version::new(0, 1, 0))
        .with_description("Example Nemo plugin")
        .with_capability(Capability::Component("my_counter".into())),
    init
);
```

For a higher-level builder API, see [`nemo-plugin`](https://crates.io/crates/nemo-plugin).

## License

MIT OR Apache-2.0
