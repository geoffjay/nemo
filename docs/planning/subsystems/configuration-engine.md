# Configuration Engine Subsystem

> **Status:** Draft  
> **Last Updated:** 2026-02-05  
> **Parent:** [System Architecture](../nemo-system-architecture.md)

## Overview

The Configuration Engine is the foundation of Nemo. It transforms human-authored HCL configuration files into validated, resolved data structures that other subsystems consume. Every aspect of a Nemo application—layouts, data flows, extensions, integrations—is defined through configuration.

## Responsibilities

1. **Parsing:** Read HCL files and produce an Abstract Syntax Tree (AST)
2. **Schema Management:** Maintain schemas for all configurable entities
3. **Validation:** Ensure configurations conform to their schemas
4. **Resolution:** Evaluate expressions, resolve references, merge includes
5. **Hot Reload:** Watch for file changes and propagate updates (dev mode)
6. **Error Reporting:** Produce clear, actionable error messages with source locations

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Configuration Engine                                 │
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   Loader    │───▶│   Parser    │───▶│  Validator  │───▶│  Resolver   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│         │                                     │                   │         │
│         │                                     │                   │         │
│         ▼                                     ▼                   ▼         │
│  ┌─────────────┐                      ┌─────────────┐    ┌─────────────┐   │
│  │   Watcher   │                      │   Schema    │    │  Resolved   │   │
│  │  (dev mode) │                      │  Registry   │    │   Config    │   │
│  └─────────────┘                      └─────────────┘    └─────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. ConfigurationLoader

**Purpose:** Orchestrate the loading process from file paths to resolved configuration.

```rust
pub trait ConfigurationLoader {
    /// Load configuration from a file path
    fn load(&self, path: &Path) -> Result<ResolvedConfig, ConfigError>;
    
    /// Load configuration from a string (for testing)
    fn load_string(&self, content: &str, source_name: &str) -> Result<ResolvedConfig, ConfigError>;
    
    /// Reload configuration (for hot reload)
    fn reload(&self, path: &Path) -> Result<ConfigDiff, ConfigError>;
}
```

**Implementation Notes:**
- Handles file I/O and encoding
- Manages include/import resolution
- Caches parsed results for performance
- Coordinates Parser → Validator → Resolver pipeline

### 2. HclParser

**Purpose:** Parse HCL text into an internal representation.

```rust
pub trait HclParser {
    /// Parse HCL content into an unvalidated configuration
    fn parse(&self, content: &str) -> Result<RawConfig, ParseError>;
}

pub struct ParseError {
    pub message: String,
    pub location: SourceLocation,
    pub suggestions: Vec<String>,
}

pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub span: Range<usize>,
}
```

**Implementation Notes:**
- Use `hcl-rs` crate for parsing
- Preserve source locations for error reporting
- Handle HCL expressions (not just literal values)
- Support HCL functions for configuration-time computation

### 3. SchemaRegistry

**Purpose:** Store and retrieve schemas for validation.

```rust
pub trait SchemaRegistry {
    /// Register a schema for a configuration type
    fn register(&mut self, type_name: &str, schema: ConfigSchema);
    
    /// Get schema by type name
    fn get(&self, type_name: &str) -> Option<&ConfigSchema>;
    
    /// List all registered types
    fn list_types(&self) -> Vec<&str>;
    
    /// Validate that a schema is well-formed
    fn validate_schema(&self, schema: &ConfigSchema) -> Result<(), SchemaError>;
}

pub struct ConfigSchema {
    pub name: String,
    pub version: Version,
    pub properties: HashMap<String, PropertySchema>,
    pub required: Vec<String>,
    pub additional_properties: bool,
}

pub struct PropertySchema {
    pub property_type: PropertyType,
    pub description: Option<String>,
    pub default: Option<Value>,
    pub validation: Option<ValidationRule>,
}

pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<PropertyType>),
    Object(Box<ConfigSchema>),
    Reference(String),  // Reference to another config type
    OneOf(Vec<PropertyType>),
    Enum(Vec<String>),
}
```

**Implementation Notes:**
- Schemas are registered at startup (built-in) and by extensions
- Support schema inheritance/composition
- Consider JSON Schema compatibility for tooling
- Version schemas for migration support

### 4. ConfigurationValidator

**Purpose:** Validate raw configuration against schemas.

```rust
pub trait ConfigurationValidator {
    /// Validate configuration against its schema
    fn validate(&self, config: &RawConfig, schema: &ConfigSchema) -> ValidationResult;
}

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

pub struct ValidationError {
    pub path: ConfigPath,          // e.g., "layout.panels[0].size"
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub location: SourceLocation,
}
```

**Validation Rules:**
- Type checking (string, number, boolean, etc.)
- Required property presence
- Enum value membership
- Range constraints (min/max for numbers)
- Pattern matching (regex for strings)
- Array length constraints
- Reference validity (referenced items exist)
- Custom validation functions

### 5. ConfigurationResolver

**Purpose:** Evaluate expressions and resolve references.

```rust
pub trait ConfigurationResolver {
    /// Resolve a raw configuration into a fully-evaluated configuration
    fn resolve(&self, config: RawConfig, context: &ResolveContext) -> Result<ResolvedConfig, ResolveError>;
}

pub struct ResolveContext {
    pub variables: HashMap<String, Value>,
    pub functions: HashMap<String, Box<dyn ConfigFunction>>,
    pub environment: HashMap<String, String>,
}
```

**Resolution Capabilities:**
- **Variable substitution:** `${var.name}`
- **Environment variables:** `${env.HOME}`
- **References:** `${component.other_panel.id}`
- **Expressions:** `${1 + 2}`, `${list.length > 0}`
- **Functions:** `${upper(var.name)}`, `${file("path/to/file")}`
- **Conditionals:** `${condition ? value_if_true : value_if_false}`

### 6. ConfigurationWatcher

**Purpose:** Monitor configuration files for changes (development mode).

```rust
pub trait ConfigurationWatcher {
    /// Start watching configuration files
    fn watch(&mut self, paths: &[PathBuf]) -> Result<(), WatchError>;
    
    /// Stop watching
    fn unwatch(&mut self);
    
    /// Subscribe to change events
    fn on_change(&mut self, callback: Box<dyn Fn(ConfigChange)>);
}

pub struct ConfigChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub old_config: Option<ResolvedConfig>,
    pub new_config: Result<ResolvedConfig, ConfigError>,
}

pub enum ChangeType {
    Modified,
    Created,
    Deleted,
}
```

**Implementation Notes:**
- Use `notify` crate for file watching
- Debounce rapid changes
- Support granular change detection (what specifically changed)
- Enable/disable via configuration

---

## HCL Configuration Language

### Why HCL?

| Feature | HCL | JSON | YAML | TOML |
|---------|-----|------|------|------|
| Human readability | Excellent | Poor | Good | Good |
| Comments | Yes | No | Yes | Yes |
| Multi-line strings | Yes | Limited | Yes | Yes |
| Expressions | Yes | No | No | No |
| Functions | Yes | No | No | No |
| Block structure | Native | Nested objects | Indentation | Tables |
| Tooling | Good | Excellent | Good | Good |

HCL provides the best balance of human-friendliness and expressive power for complex configuration.

### Configuration Structure

```hcl
# Application metadata
application {
  name    = "My Dashboard"
  version = "1.0.0"
}

# Variables for reuse
variable "api_base" {
  default = "https://api.example.com"
}

# Layout definition
layout {
  type = "dock"
  
  center {
    panel "main-view" {
      component = "data-table"
      title     = "Data View"
      
      config {
        columns = ["name", "value", "timestamp"]
        source  = data.api_data.records
      }
    }
  }
  
  left {
    width = 250
    
    panel "sidebar" {
      component = "tree-view"
      title     = "Navigation"
    }
  }
}

# Data source definition
data "http" "api_data" {
  url      = "${var.api_base}/data"
  interval = "30s"
  
  transform {
    type = "jq"
    query = ".items | map({name: .n, value: .v})"
  }
}

# Event handler
on "data.api_data.updated" {
  action = "refresh"
  target = "panel.main-view"
}

# Extension loading
extension "my-plugin" {
  path = "./plugins/my-plugin.so"
}

script "helpers" {
  path = "./scripts/helpers.rhai"
}
```

### Block Types

| Block Type | Purpose | Example |
|------------|---------|---------|
| `application` | App metadata | name, version, theme |
| `variable` | Reusable values | API URLs, constants |
| `layout` | UI structure | Dock areas, panels |
| `panel` | Individual panel | Component, config |
| `data` | Data sources | HTTP, WebSocket, file |
| `transform` | Data transformation | Map, filter, aggregate |
| `binding` | Data-to-UI binding | Source, target, mode |
| `on` | Event handlers | Event, action, target |
| `action` | Named actions | Sequences, conditions |
| `extension` | Native plugins | Path, config |
| `script` | RHAI scripts | Path, exports |
| `integration` | External systems | RPC, PubSub endpoints |

---

## Error Handling

### Error Categories

1. **Parse Errors:** Invalid HCL syntax
2. **Schema Errors:** Configuration doesn't match schema
3. **Resolution Errors:** Cannot resolve expression or reference
4. **Semantic Errors:** Valid syntax but invalid meaning

### Error Format

```
Error: Invalid component reference

  --> config/main.hcl:45:15
   |
45 |       source = data.nonexistent.records
   |               ^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Available data sources: api_data, local_cache
   = note: Data sources must be defined before they can be referenced
```

### Error Recovery

- Continue parsing after errors to report multiple issues
- Provide "did you mean?" suggestions for typos
- Show context around errors
- Link to documentation for complex errors

---

## Performance Considerations

1. **Lazy Loading:** Only parse files when needed
2. **Caching:** Cache parsed and validated configurations
3. **Incremental Updates:** On hot reload, only re-process changed sections
4. **Parallel Validation:** Validate independent sections concurrently

---

## Testing Strategy

### Unit Tests
- Parser handles all HCL constructs
- Validator catches schema violations
- Resolver evaluates all expression types

### Integration Tests
- Full pipeline from file to resolved config
- Hot reload detects and applies changes
- Error messages are helpful

### Property Tests
- Valid configs always parse and validate
- Invalid configs always produce errors
- Resolver is deterministic

---

## Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `hcl-rs` | HCL parsing | latest |
| `serde` | Serialization | 1.x |
| `notify` | File watching | 6.x |
| `thiserror` | Error types | 1.x |

---

## Open Questions

1. **Schema Format:** Use JSON Schema for compatibility, or define our own?
2. **Expression Language:** Use HCL's native expressions, or embed a more powerful language?
3. **Include Mechanism:** How do configurations compose? Inheritance? Mixins?
4. **Versioning:** How do we handle schema migration between versions?

---

## Agent Prompt Considerations

When creating an agent to implement the Configuration Engine:

- **Clear boundaries:** The agent handles parsing and validation, not UI construction
- **Test-first:** Schema validation is critical—comprehensive tests required
- **Error quality:** Error messages are a user interface—treat them as first-class
- **Extensibility:** Other agents will register schemas—provide clean APIs

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
