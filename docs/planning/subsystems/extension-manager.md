# Extension Manager Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Extension Manager enables Nemo's "progressive complexity" model by providing two extension mechanisms: RHAI scripts for lightweight customization and native plugins for performance-critical or deep integrations. This subsystem discovers, loads, initializes, and coordinates extensions while maintaining safety guarantees.

## Responsibilities

1. **Discovery:** Find scripts and plugins in configured locations
2. **Loading:** Load RHAI scripts and dynamic libraries safely
3. **Sandboxing:** Provide secure execution environment for scripts
4. **API Exposure:** Make Nemo capabilities available to extensions
5. **Lifecycle Management:** Initialize, update, and unload extensions
6. **Error Isolation:** Prevent extension failures from crashing the host

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Extension Manager                                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Extension Loader                                   │ │
│  │  ┌─────────────────────┐    ┌─────────────────────┐                    │ │
│  │  │   Script Loader     │    │   Plugin Loader     │                    │ │
│  │  │   (RHAI files)      │    │   (*.so/*.dylib)    │                    │ │
│  │  └─────────────────────┘    └─────────────────────┘                    │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    Extension Registry                                   │ │
│  │  ┌─────────────────────────────────────────────────────────────────┐    │ │
│  │  │  Scripts  │  Plugins  │  Manifests  │  Capabilities             │    │ │
│  │  └─────────────────────────────────────────────────────────────────┘    │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    Extension Runtime                                    │ │
│  │  ┌─────────────────────┐    ┌─────────────────────┐                    │ │
│  │  │    RHAI Engine      │    │    Plugin Host      │                    │ │
│  │  │   (sandboxed)       │    │   (libloading)      │                    │ │
│  │  └─────────────────────┘    └─────────────────────┘                    │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    Extension Context (API)                              │ │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐   │ │
│  │  │  Data   │ │  UI     │ │ Events  │ │ Config  │ │     Logger      │   │ │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Extension Types

### 1. RHAI Scripts

**Use Cases:**
- Event handlers
- Data transformations
- Custom validation logic
- Simple actions and automations
- Configuration-time computations

**Characteristics:**
- Sandboxed execution
- No direct system access
- Hot-reloadable
- JavaScript+Rust-like syntax
- Good for rapid iteration

### 2. Native Plugins

**Use Cases:**
- Custom UI components
- High-performance data sources
- Protocol implementations
- Hardware integrations
- Compute-intensive transforms

**Characteristics:**
- Full Rust capabilities
- Direct system access (if permitted)
- Compiled .so/.dylib/.dll files
- Higher performance ceiling
- Requires compilation

---

## Core Components

### 1. ExtensionLoader

**Purpose:** Discover and load extensions from configured paths.

```rust
pub struct ExtensionLoader {
    script_paths: Vec<PathBuf>,
    plugin_paths: Vec<PathBuf>,
}

impl ExtensionLoader {
    /// Discover all extensions in configured paths
    pub fn discover(&self) -> Result<Vec<ExtensionManifest>, DiscoveryError>;
    
    /// Load a RHAI script
    pub fn load_script(&self, path: &Path) -> Result<LoadedScript, LoadError>;
    
    /// Load a native plugin
    pub fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin, LoadError>;
    
    /// Watch for changes (dev mode)
    pub fn watch(&self, on_change: impl Fn(ExtensionChange));
}

pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub extension_type: ExtensionType,
    pub path: PathBuf,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<Dependency>,
    pub config_schema: Option<ConfigSchema>,
}

pub enum ExtensionType {
    Script,
    Plugin,
}

pub enum Capability {
    DataSource,
    Transform,
    Component,
    Action,
    EventHandler,
}
```

### 2. RhaiEngine

**Purpose:** Configure and manage the RHAI scripting runtime.

```rust
pub struct RhaiEngine {
    engine: rhai::Engine,
    scripts: HashMap<String, rhai::AST>,
    scopes: HashMap<String, rhai::Scope<'static>>,
}

impl RhaiEngine {
    /// Create a new sandboxed engine
    pub fn new(config: RhaiConfig) -> Self;
    
    /// Register Nemo API functions
    pub fn register_api(&mut self, context: Arc<ExtensionContext>);
    
    /// Load and compile a script
    pub fn load_script(&mut self, id: &str, source: &str) -> Result<(), CompileError>;
    
    /// Execute a script function
    pub fn call<T: rhai::Variant + Clone>(
        &self,
        script_id: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, ExecutionError>;
    
    /// Evaluate an expression
    pub fn eval<T: rhai::Variant + Clone>(
        &self,
        expr: &str,
        scope: &rhai::Scope,
    ) -> Result<T, EvalError>;
    
    /// Reload a script (hot reload)
    pub fn reload_script(&mut self, id: &str, source: &str) -> Result<(), CompileError>;
}

pub struct RhaiConfig {
    /// Maximum script execution time
    pub max_execution_time: Duration,
    
    /// Maximum call stack depth
    pub max_call_depth: usize,
    
    /// Maximum number of operations
    pub max_operations: u64,
    
    /// Maximum string length
    pub max_string_size: usize,
    
    /// Maximum array length
    pub max_array_size: usize,
    
    /// Enable/disable specific features
    pub features: RhaiFeatures,
}

pub struct RhaiFeatures {
    pub loops: bool,
    pub functions: bool,
    pub closures: bool,
    pub modules: bool,
}
```

### 3. PluginHost

**Purpose:** Load and manage native plugins using libloading.

```rust
pub struct PluginHost {
    loaded_plugins: HashMap<String, LoadedPlugin>,
    registrar: PluginRegistrar,
}

impl PluginHost {
    /// Load a plugin from path
    pub fn load(&mut self, path: &Path) -> Result<String, PluginError>;
    
    /// Unload a plugin
    pub fn unload(&mut self, id: &str) -> Result<(), PluginError>;
    
    /// Get plugin by ID
    pub fn get(&self, id: &str) -> Option<&LoadedPlugin>;
    
    /// List loaded plugins
    pub fn list(&self) -> Vec<&LoadedPlugin>;
}

pub struct LoadedPlugin {
    pub id: String,
    pub manifest: ExtensionManifest,
    library: libloading::Library,
    vtable: PluginVTable,
}

/// Plugin entry point signature
type PluginEntryFn = unsafe extern "C" fn(&mut dyn PluginRegistrar);

/// V-table for plugin capabilities
pub struct PluginVTable {
    pub components: Vec<Box<dyn ComponentFactory>>,
    pub data_sources: Vec<Box<dyn DataSourceFactory>>,
    pub transforms: Vec<Box<dyn TransformFactory>>,
    pub actions: Vec<Box<dyn ActionFactory>>,
}
```

**Plugin Interface (core crate):**

```rust
// This is the shared interface between host and plugins
// Plugins link against this crate

/// Registrar passed to plugin entry point
pub trait PluginRegistrar {
    fn register_component(&mut self, name: &str, factory: Box<dyn ComponentFactory>);
    fn register_data_source(&mut self, name: &str, factory: Box<dyn DataSourceFactory>);
    fn register_transform(&mut self, name: &str, factory: Box<dyn TransformFactory>);
    fn register_action(&mut self, name: &str, factory: Box<dyn ActionFactory>);
}

/// Macro to declare plugin entry point
#[macro_export]
macro_rules! declare_plugin {
    ($init:expr) => {
        #[no_mangle]
        pub extern "C" fn nemo_plugin_entry(registrar: &mut dyn PluginRegistrar) {
            $init(registrar)
        }
    };
}
```

**Example Plugin:**

```rust
// my_plugin/src/lib.rs
use nemo_plugin_api::*;

struct MyCustomChart;

impl ComponentFactory for MyCustomChart {
    fn create(&self, config: &ComponentConfig, cx: &mut Context) -> Result<Box<dyn PanelView>, ComponentError> {
        // Create custom chart component
    }
    
    fn schema(&self) -> &ConfigSchema {
        // Return schema for configuration validation
    }
}

fn init_plugin(registrar: &mut dyn PluginRegistrar) {
    registrar.register_component("my-chart", Box::new(MyCustomChart));
}

declare_plugin!(init_plugin);
```

### 4. ExtensionContext

**Purpose:** API surface exposed to extensions.

```rust
pub struct ExtensionContext {
    data: Arc<DataRepository>,
    events: Arc<EventBus>,
    config: Arc<ConfigurationLoader>,
    logger: Arc<Logger>,
}

impl ExtensionContext {
    // ---- Data API ----
    
    /// Get data by path
    pub fn get_data(&self, path: &str) -> Option<Value>;
    
    /// Set data at path
    pub fn set_data(&self, path: &str, value: Value) -> Result<(), DataError>;
    
    /// Subscribe to data changes
    pub fn subscribe_data(&self, path: &str, callback: impl Fn(Value) + Send + 'static);
    
    // ---- Event API ----
    
    /// Emit an event
    pub fn emit_event(&self, event_type: &str, payload: Value);
    
    /// Subscribe to events
    pub fn subscribe_event(&self, event_type: &str, callback: impl Fn(Value) + Send + 'static);
    
    // ---- Config API ----
    
    /// Get configuration value
    pub fn get_config(&self, path: &str) -> Option<Value>;
    
    // ---- Logging API ----
    
    pub fn log_debug(&self, message: &str);
    pub fn log_info(&self, message: &str);
    pub fn log_warn(&self, message: &str);
    pub fn log_error(&self, message: &str);
}
```

**RHAI API Registration:**

```rust
impl RhaiEngine {
    pub fn register_api(&mut self, context: Arc<ExtensionContext>) {
        let ctx = context.clone();
        self.engine.register_fn("get_data", move |path: &str| {
            ctx.get_data(path)
        });
        
        let ctx = context.clone();
        self.engine.register_fn("set_data", move |path: &str, value: rhai::Dynamic| {
            ctx.set_data(path, dynamic_to_value(value))
        });
        
        let ctx = context.clone();
        self.engine.register_fn("emit", move |event_type: &str, payload: rhai::Dynamic| {
            ctx.emit_event(event_type, dynamic_to_value(payload))
        });
        
        let ctx = context.clone();
        self.engine.register_fn("log", move |level: &str, message: &str| {
            match level {
                "debug" => ctx.log_debug(message),
                "info" => ctx.log_info(message),
                "warn" => ctx.log_warn(message),
                "error" => ctx.log_error(message),
                _ => ctx.log_info(message),
            }
        });
        
        // Register types
        self.engine.register_type_with_name::<Value>("Value");
        // ... more registrations
    }
}
```

---

## Configuration

### Script Configuration

```hcl
script "data_transformer" {
  path = "./scripts/transform.rhai"
  
  # Optional: Configuration passed to script
  config {
    threshold = 100
    format    = "compact"
  }
  
  # Which events this script handles
  handles = ["data.updated", "user.action"]
  
  # Which transforms this script provides
  transforms = ["custom_filter", "format_output"]
}
```

### Plugin Configuration

```hcl
extension "chart_plugin" {
  path = "./plugins/libchart.so"
  
  # Plugin-specific configuration
  config {
    license_key = "${env.CHART_LICENSE}"
    theme       = "dark"
  }
  
  # Permissions granted to this plugin
  permissions {
    network     = false
    filesystem  = false
    subprocess  = false
  }
}
```

---

## RHAI Script Examples

### Event Handler

```rhai
// scripts/handlers.rhai

// Handle data updates
fn on_data_updated(event) {
    let data = event.payload;
    
    if data.value > get_config("threshold") {
        emit("alert", #{
            level: "warning",
            message: `Value ${data.value} exceeded threshold`
        });
    }
    
    log("info", `Processed data update: ${data.id}`);
}

// Handle user actions
fn on_user_action(event) {
    let action = event.payload.action;
    
    switch action {
        "refresh" => {
            set_data("ui.loading", true);
            // Trigger refresh
        },
        "export" => {
            let data = get_data("table.rows");
            // Process export
        },
        _ => log("warn", `Unknown action: ${action}`)
    }
}
```

### Data Transform

```rhai
// scripts/transforms.rhai

// Transform function used in data pipeline
fn format_currency(value) {
    let amount = value.amount;
    let currency = value.currency ?? "USD";
    
    `${currency} ${amount.to_string()}`
}

// Filter function
fn filter_active(items) {
    items.filter(|item| item.status == "active")
}

// Aggregation function
fn calculate_totals(items) {
    #{
        count: items.len(),
        sum: items.map(|i| i.value).reduce(|a, b| a + b, 0),
        avg: if items.len() > 0 {
            items.map(|i| i.value).reduce(|a, b| a + b, 0) / items.len()
        } else {
            0
        }
    }
}
```

### Custom Action

```rhai
// scripts/actions.rhai

fn export_to_csv(params) {
    let data = get_data(params.source);
    let columns = params.columns;
    
    let csv = columns.join(",") + "\n";
    
    for row in data {
        let values = columns.map(|col| row[col].to_string());
        csv += values.join(",") + "\n";
    }
    
    // Return result to be handled by host
    #{
        type: "file_download",
        filename: params.filename ?? "export.csv",
        content: csv,
        mime_type: "text/csv"
    }
}
```

---

## Safety & Sandboxing

### RHAI Sandboxing

| Protection | Implementation |
|------------|----------------|
| Infinite loops | `max_operations` limit |
| Stack overflow | `max_call_depth` limit |
| Memory exhaustion | `max_string_size`, `max_array_size` |
| Long execution | `max_execution_time` timeout |
| File system access | Not exposed in API |
| Network access | Only through Nemo data sources |
| Code injection | AST pre-compilation, no `eval` |

### Plugin Security

| Concern | Mitigation |
|---------|------------|
| Memory safety | Rust's ownership system |
| Undefined behavior | `unsafe` blocks reviewed |
| Resource exhaustion | Capability-based permissions |
| Malicious plugins | Manifest declaration of capabilities |
| ABI compatibility | Stable ABI at plugin boundary |

**Permission Model:**

```rust
pub struct PluginPermissions {
    /// Can make network requests
    pub network: bool,
    
    /// Can access filesystem
    pub filesystem: bool,
    
    /// Can spawn subprocesses
    pub subprocess: bool,
    
    /// Can access specific data paths
    pub data_paths: Vec<String>,
    
    /// Can emit specific events
    pub event_types: Vec<String>,
}
```

---

## Lifecycle Management

### Initialization Sequence

1. **Discovery:** Find all extensions in configured paths
2. **Validation:** Check manifests and dependencies
3. **Dependency Resolution:** Order extensions by dependencies
4. **Loading:** Load scripts and plugins
5. **API Registration:** Register extension capabilities
6. **Initialization:** Call extension init functions
7. **Ready:** Extensions available for use

### Hot Reload (Scripts)

```rust
impl ExtensionManager {
    pub fn reload_script(&mut self, script_id: &str) -> Result<(), ReloadError> {
        // 1. Compile new version
        let source = self.loader.read_script(script_id)?;
        let new_ast = self.rhai_engine.compile(&source)?;
        
        // 2. Swap in new version
        self.rhai_engine.replace_script(script_id, new_ast);
        
        // 3. Notify subscribers
        self.events.emit(ExtensionReloaded { id: script_id.into() });
        
        Ok(())
    }
}
```

### Shutdown Sequence

1. **Pre-shutdown Event:** Notify extensions
2. **Script Cleanup:** Clear RHAI scopes
3. **Plugin Shutdown:** Call plugin shutdown functions
4. **Unload Libraries:** Drop libloading handles
5. **Resource Cleanup:** Free associated resources

---

## Error Handling

### Error Types

```rust
pub enum ExtensionError {
    /// Extension not found
    NotFound { id: String },
    
    /// Failed to load
    LoadError { id: String, reason: String },
    
    /// Script compilation failed
    CompileError { script_id: String, error: rhai::ParseError },
    
    /// Script execution failed
    ExecutionError { script_id: String, function: String, error: Box<rhai::EvalAltResult> },
    
    /// Plugin initialization failed
    PluginInitError { plugin_id: String, reason: String },
    
    /// Permission denied
    PermissionDenied { plugin_id: String, capability: String },
    
    /// Dependency not satisfied
    DependencyError { id: String, missing: Vec<String> },
}
```

### Error Recovery

| Error Type | Recovery Strategy |
|------------|-------------------|
| Script compile error | Log error, skip script, continue |
| Script runtime error | Catch, log, return default/null |
| Plugin load failure | Log error, continue without plugin |
| Permission denied | Log warning, deny operation |
| Timeout | Kill execution, return error |

---

## Testing Extensions

### Script Testing

```rhai
// tests/transform_test.rhai

fn test_format_currency() {
    let result = format_currency(#{ amount: 100, currency: "EUR" });
    assert_eq(result, "EUR 100");
}

fn test_filter_active() {
    let items = [
        #{ id: 1, status: "active" },
        #{ id: 2, status: "inactive" },
        #{ id: 3, status: "active" }
    ];
    
    let result = filter_active(items);
    assert_eq(result.len(), 2);
}
```

### Plugin Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nemo_plugin_api::testing::*;
    
    #[test]
    fn test_custom_chart_creation() {
        let factory = MyCustomChart;
        let config = ComponentConfig::from_json(r#"{ "type": "bar" }"#);
        
        let mut cx = MockContext::new();
        let result = factory.create(&config, &mut cx);
        
        assert!(result.is_ok());
    }
}
```

---

## Agent Prompt Considerations

When creating an agent to implement the Extension Manager:

- **Security focus:** Sandboxing is critical—test escape attempts
- **API design:** Extension API should be minimal but sufficient
- **Error messages:** Script errors need helpful diagnostics
- **RHAI expertise:** Understand RHAI's type system and limitations
- **FFI knowledge:** Plugin boundary requires careful ABI handling
- **Testing:** Test malicious inputs, resource exhaustion, reentrancy

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
