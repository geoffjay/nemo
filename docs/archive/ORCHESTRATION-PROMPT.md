# Nemo Project Build Orchestration

## Project Context

Build **Nemo**, a Rust meta-application framework that constructs desktop applications from HCL configuration. The complete architecture and implementation specifications are in `docs/planning/`. Eight specialized agents are available in `.claude/agents/` to implement each subsystem.

## Planning Documents

Read these documents to understand the full architecture:

- `docs/planning/nemo-project-vision.md` - Project scope, principles, success criteria
- `docs/planning/nemo-system-architecture.md` - Seven subsystems, interactions, data flow diagrams
- `docs/planning/subsystems/*.md` - Detailed specifications for each subsystem

## Workspace Structure

Create this Cargo workspace with 9 crates:

```
nemo/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── nemo/               # Main binary (application shell)
│   ├── nemo-config/        # Configuration Engine
│   ├── nemo-registry/      # Component Registry
│   ├── nemo-layout/        # Layout Engine
│   ├── nemo-data/          # Data Flow Engine
│   ├── nemo-extension/     # Extension Manager
│   ├── nemo-integration/   # Integration Gateway
│   ├── nemo-events/        # Event Bus
│   └── nemo-plugin-api/    # Plugin Author API
```

## Build Order & Agent Assignments

Execute in this order, respecting dependencies. Each agent has detailed implementation specs in their prompt.

### Phase 1: Foundation (No Dependencies)

**1. Event Bus** - Use the subagent task `event-bus`
- Create `crates/nemo-events`
- Implement EventBus, Event, EventFilter, EventSubscription, EventTracer
- This runs in parallel with Configuration Engine since neither depends on the other

**2. Configuration Engine** - Use the subagent task `configuration-engine`
- Create `crates/nemo-config`
- Implement HCL parsing (use `hcl-rs`), schema validation, expression resolution
- Core types: Value, ConfigPath, SourceLocation, ConfigSchema, PropertySchema
- Components: HclParser, SchemaRegistry, ConfigValidator, ConfigResolver, ConfigurationLoader

### Phase 2: Registry (Depends on Phase 1)

**3. Component Registry** - Use the subagent task `component-registry`
- Create `crates/nemo-registry`
- Implement ComponentRegistry with descriptors for Component, DataSource, Transform, Action
- Create placeholder factories and schemas for all gpui-component types
- Depends on: nemo-config (for ConfigSchema types)

### Phase 3: Core Systems (Depends on Phase 2)

**4. Layout Engine** - Use the subagent task `layout-engine`
- Create `crates/nemo-layout`
- Implement LayoutBuilder, LayoutManager, DockController, BindingManager
- Create ComponentFactory implementations for all gpui-component types
- Register factories with ComponentRegistry
- Depends on: nemo-config, nemo-registry, nemo-events

**5. Data Flow Engine** - Use the subagent task `data-flow-engine`
- Create `crates/nemo-data`
- Implement DataSource trait and built-ins: HttpSource, WebSocketSource, FileSource, TimerSource
- Implement Transform trait and built-ins: MapTransform, FilterTransform, SelectTransform, etc.
- Implement DataRepository, ActionSystem, ActionTrigger
- Depends on: nemo-config, nemo-events

### Phase 4: Extensions (Depends on Phase 3)

**6. Extension Manager** - Use the subagent task `extension-manager`
- Create `crates/nemo-extension` and `crates/nemo-plugin-api`
- Implement RhaiEngine with sandboxing, PluginHost with libloading
- Create ExtensionContext API surface
- Depends on: nemo-config, nemo-registry, nemo-data, nemo-events

**7. Integration Gateway** - Use the subagent task `integration-gateway`
- Create `crates/nemo-integration`
- Implement ProtocolAdapter trait and adapters: JsonRpcAdapter, MqttAdapter, RedisAdapter
- Implement ConnectionManager, CircuitBreaker, retry logic
- Create client APIs: RpcClient, PubSubClient, QueueClient
- Depends on: nemo-config, nemo-data, nemo-events

### Phase 5: Application Shell (Depends on All)

**8. Application Shell** - Use the subagent task `application-shell`
- Create `crates/nemo` (main binary)
- Implement bootstrap sequence that initializes all subsystems
- Create Root component with TitleBar and content area
- Wire everything together
- Depends on: ALL crates

## Workspace Cargo.toml

Create this as the workspace root:

```toml
[workspace]
resolver = "2"
members = [
    "crates/nemo",
    "crates/nemo-config",
    "crates/nemo-registry",
    "crates/nemo-layout",
    "crates/nemo-data",
    "crates/nemo-extension",
    "crates/nemo-integration",
    "crates/nemo-events",
    "crates/nemo-plugin-api",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/nemo"

[workspace.dependencies]
# GPUI
gpui = "0.2.2"
gpui-component = "0.5.1"

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"
anyhow = "1"
miette = { version = "7", features = ["fancy"] }

# Configuration
hcl-rs = "0.18"

# Scripting
rhai = "1.19"

# Plugin loading
libloading = "0.8"

# Networking
reqwest = { version = "0.12", features = ["json"] }
tokio-tungstenite = "0.24"

# Messaging
rumqttc = "0.24"
redis = { version = "0.27", features = ["tokio-comp"] }
async-nats = "0.37"

# File watching
notify = "6"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
indexmap = { version = "2", features = ["serde"] }
semver = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }

# Internal crates
nemo-config = { path = "crates/nemo-config" }
nemo-registry = { path = "crates/nemo-registry" }
nemo-layout = { path = "crates/nemo-layout" }
nemo-data = { path = "crates/nemo-data" }
nemo-extension = { path = "crates/nemo-extension" }
nemo-integration = { path = "crates/nemo-integration" }
nemo-events = { path = "crates/nemo-events" }
nemo-plugin-api = { path = "crates/nemo-plugin-api" }
```

## Execution Instructions

1. **First**, set up the workspace structure and root Cargo.toml

2. **Then**, execute phases in order. Within each phase, agents can work in parallel if they don't depend on each other:
   - Phase 1: `event-bus` and `configuration-engine` can run in parallel
   - Phase 2: `component-registry` after Phase 1 completes
   - Phase 3: `layout-engine` and `data-flow-engine` can run in parallel
   - Phase 4: `extension-manager` and `integration-gateway` can run in parallel
   - Phase 5: `application-shell` after all others complete

3. **Each agent should**:
   - Read their detailed prompt in `.claude/agents/`
   - Read relevant architecture docs in `docs/planning/`
   - Create their crate with proper Cargo.toml using workspace dependencies
   - Implement all types, traits, and functions specified
   - Write unit tests
   - Ensure `cargo check` passes before completing

4. **After each phase**, run `cargo check --workspace` to verify integration

5. **Final verification**: `cargo build --workspace` should succeed

## Success Criteria

- [ ] All 9 crates created with proper structure
- [ ] Workspace compiles without errors
- [ ] Each crate has basic tests
- [ ] Main binary starts and shows empty window with titlebar
- [ ] Configuration loads from HCL file
- [ ] Layout renders from configuration

## Minimal Test Configuration

Create `examples/minimal.hcl` to test the final build:

```hcl
application {
  name    = "Nemo Test"
  version = "0.1.0"
}

layout {
  type = "dock"
  
  center {
    panel "welcome" {
      component = "label"
      config {
        text = "Welcome to Nemo!"
        size = "xl"
      }
    }
  }
}
```

Run with: `cargo run -- --config examples/minimal.hcl`
