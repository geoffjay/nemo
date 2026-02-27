---
description: Scaffold a new data source type across nemo-data, nemo-registry, and nemo-integration
---

# Add Nemo Data Source

Scaffold a new data source type for the Nemo framework across all required crates.

## Arguments

The user should provide:
- **Source type name** (e.g., `grpc`, `graphql`, `kafka`) — snake_case
- **Description** — what the source does
- **Connection properties** — URL, credentials, topics, etc.
- **Behavior** — polling (interval-based) or streaming (persistent connection)

If not provided, ask the user.

## Steps

### 1. Read existing source patterns

Read an appropriate existing source based on the behavior:
- Polling: `crates/nemo-data/src/sources/http.rs` and `crates/nemo-data/src/sources/timer.rs`
- Streaming: `crates/nemo-data/src/sources/websocket.rs` or `crates/nemo-data/src/sources/mqtt.rs`
- Also read: `crates/nemo-data/src/source.rs` for the `DataSource` trait
- Also read: `crates/nemo-data/src/sources/mod.rs` for the module structure

### 2. Create the source implementation

Create `crates/nemo-data/src/sources/<name>.rs`:

```rust
use crate::error::DataError;
use crate::source::DataSource;
use async_trait::async_trait;
use nemo_config::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct <Name>Source {
    // Connection config fields
}

impl <Name>Source {
    pub fn new(config: &HashMap<String, Value>) -> Result<Self, DataError> {
        // Extract config properties
        Ok(Self { /* fields */ })
    }
}

#[async_trait]
impl DataSource for <Name>Source {
    async fn start(&self, sender: mpsc::Sender<Value>) -> Result<(), DataError> {
        // Implement polling or streaming logic
        todo!()
    }

    async fn stop(&self) -> Result<(), DataError> {
        // Cleanup
        Ok(())
    }

    fn name(&self) -> &str {
        "<name>"
    }
}
```

### 3. Update sources/mod.rs

Edit `crates/nemo-data/src/sources/mod.rs`:
- Add `pub mod <name>;`
- Add `pub use <name>::<Name>Source;`

### 4. Register in builtins.rs

Edit `crates/nemo-registry/src/builtins.rs` in `register_builtin_data_sources()`:

```rust
let mut src = DataSourceDescriptor::new("<name>");
src.metadata = DataSourceMetadata {
    display_name: "<Display Name>".into(),
    description: "<description>".into(),
    supports_polling: true,  // or supports_streaming: true
    supports_manual_refresh: true,
    ..Default::default()
};
src.schema = ConfigSchema::new("<name>")
    .property("url", PropertySchema::string())
    // Add other properties
    .require("url");
let _ = registry.register_data_source(src);
```

### 5. Add integration client (if new protocol)

If this requires a new protocol client, create `crates/nemo-integration/src/<name>.rs`:
- Implement the protocol client
- Add to `IntegrationGateway` struct in `crates/nemo-integration/src/lib.rs`

### 6. Add dependencies

If new crates are needed (e.g., a protocol library), add to the appropriate `Cargo.toml`:
- `crates/nemo-data/Cargo.toml` for the data source
- `crates/nemo-integration/Cargo.toml` for the protocol client
- Consider adding to `[workspace.dependencies]` in root `Cargo.toml`

### 7. Wire up in runtime

Check `crates/nemo/src/runtime.rs` to ensure the new source type is handled in the data source startup sequence.

### 8. Verify

Run `cargo check -p nemo-data` and `cargo test -p nemo-data`.
