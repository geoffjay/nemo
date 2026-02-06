# Nemo: System Architecture

> **Status:** Draft  
> **Last Updated:** 2026-02-05

## Architecture Overview

Nemo is structured as a layered architecture with six primary subsystems. Each subsystem has clear responsibilities and communicates through defined interfaces.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION LAYER                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                         User Application (HCL)                          │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│                              EXTENSION LAYER                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────────────────┐  │
│  │   RHAI Scripts   │  │  Native Plugins  │  │   Custom Components       │  │
│  └──────────────────┘  └──────────────────┘  └───────────────────────────┘  │
├─────────────────────────────────────────────────────────────────────────────┤
│                               CORE LAYER                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │  Configuration  │  │     Layout      │  │        Data Flow            │  │
│  │     Engine      │  │     Engine      │  │         Engine              │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │   Component     │  │     Event       │  │       Integration           │  │
│  │    Registry     │  │      Bus        │  │         Gateway             │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────────────────┤
│                            FOUNDATION LAYER                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │      GPUI       │  │ gpui-component  │  │      Rust Runtime           │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Subsystem Definitions

### 1. Configuration Engine

**Purpose:** Parse, validate, and provide access to HCL configuration files.

**Responsibilities:**
- Parse HCL files into an internal representation
- Validate configuration against registered schemas
- Resolve configuration references and expressions
- Support configuration inheritance and composition
- Provide hot-reload capabilities for development
- Emit clear, actionable error messages

**Key Interfaces:**
- `ConfigurationLoader` — Reads and parses configuration files
- `SchemaRegistry` — Stores and retrieves component schemas
- `ConfigurationValidator` — Validates config against schemas
- `ConfigurationResolver` — Resolves expressions and references
- `ConfigurationWatcher` — Monitors files for changes (dev mode)

**Dependencies:**
- HCL parser (hcl-rs or custom)
- Schema validation library
- File system watcher

---

### 2. Layout Engine

**Purpose:** Transform configuration into GPUI component trees and manage dynamic UI construction.

**Responsibilities:**
- Instantiate UI components from configuration
- Construct component hierarchies (parent-child relationships)
- Manage dock layouts, panels, and tabs
- Handle responsive sizing and constraints
- Support layout serialization/deserialization
- Coordinate with Data Flow for bindings

**Key Interfaces:**
- `LayoutBuilder` — Constructs component trees from config
- `ComponentFactory` — Creates component instances by type
- `LayoutManager` — Manages runtime layout state
- `DockController` — Controls dock area behavior
- `BindingResolver` — Connects components to data sources

**Dependencies:**
- Configuration Engine (for layout config)
- Component Registry (for component types)
- Data Flow Engine (for bindings)
- gpui-component (DockArea, Resizable, etc.)

---

### 3. Data Flow Engine

**Purpose:** Manage the complete data lifecycle: collection, transformation, storage, action triggers, and display binding.

**Responsibilities:**
- Define and manage data sources (collectors)
- Transform and aggregate data through pipelines
- Store data in typed repositories
- Trigger actions based on data conditions
- Bind data to UI component properties
- Provide reactive updates when data changes

**Key Interfaces:**
- `DataSource` trait — Defines data collection interface
- `DataTransformer` trait — Defines transformation interface
- `DataRepository` — Typed storage with change notification
- `DataBinding` — Connects data to component properties
- `ActionTrigger` — Fires actions based on data conditions
- `DataPipeline` — Chains sources, transforms, and sinks

**Sub-components:**
- **Collectors:** HTTP polling, WebSocket streams, file watchers, timer-based
- **Transformers:** Map, filter, aggregate, join, window
- **Repositories:** In-memory, persistent (SQLite), cached
- **Bindings:** One-way, two-way, computed

**Dependencies:**
- Event Bus (for change propagation)
- Integration Gateway (for external data)
- RHAI Engine (for script-based transforms)

---

### 4. Component Registry

**Purpose:** Maintain a catalog of available UI components with their schemas, factories, and metadata.

**Responsibilities:**
- Register built-in gpui-component types
- Register custom components from plugins
- Provide component schemas for validation
- Store component metadata (documentation, examples)
- Support component versioning
- Enable component discovery and introspection

**Key Interfaces:**
- `ComponentDescriptor` — Describes a component type
- `ComponentSchema` — JSON Schema for component configuration
- `ComponentFactory` — Creates component instances
- `ComponentRegistry` — Central registration and lookup

**Dependencies:**
- Configuration Engine (provides schemas)
- Layout Engine (consumes factories)
- Extension Manager (registers plugin components)

---

### 5. Event Bus

**Purpose:** Provide decoupled communication between all system components through a typed event system.

**Responsibilities:**
- Support publish/subscribe for typed events
- Enable request/response patterns
- Provide event filtering and routing
- Support synchronous and asynchronous delivery
- Enable event logging and replay (debugging)
- Integrate with GPUI's action system

**Key Interfaces:**
- `EventBus` — Central pub/sub coordinator
- `EventEmitter` — Publishes events
- `EventSubscriber` — Receives events
- `EventFilter` — Routes events selectively
- `EventLog` — Records events for debugging

**Event Categories:**
- **UI Events:** User interactions, focus changes
- **Data Events:** Source updates, binding changes
- **System Events:** Lifecycle, errors, configuration changes
- **Custom Events:** Application-defined events

**Dependencies:**
- GPUI's Context and action system
- Async runtime for deferred delivery

---

### 6. Integration Gateway

**Purpose:** Provide standardized protocols for external system communication.

**Responsibilities:**
- Implement RPC client/server capabilities
- Provide PubSub broker integration
- Support message queue patterns
- Handle protocol-specific serialization
- Manage connection lifecycle and reconnection
- Provide health checking and circuit breaking

**Key Interfaces:**
- `RpcClient` / `RpcServer` — Request/response communication
- `PubSubClient` — Topic-based messaging
- `MessageQueueClient` — Queue-based messaging
- `ConnectionManager` — Handles connections and health
- `ProtocolAdapter` — Adapts specific protocols

**Supported Protocols (initial):**
- **RPC:** JSON-RPC over HTTP, gRPC (optional)
- **PubSub:** MQTT, Redis Pub/Sub, custom WebSocket
- **Message Queue:** Redis Streams, NATS (optional)

**Dependencies:**
- Data Flow Engine (as data sources)
- Event Bus (for internal event bridging)
- Async runtime (tokio)

---

### 7. Extension Manager

**Purpose:** Load, initialize, and coordinate RHAI scripts and native plugins.

**Responsibilities:**
- Discover and load RHAI scripts
- Discover and load native plugin libraries
- Provide sandboxed execution for scripts
- Expose Nemo APIs to extensions
- Manage extension lifecycle
- Handle extension errors gracefully

**Key Interfaces:**
- `ExtensionLoader` — Discovers and loads extensions
- `RhaiEngine` — Configured RHAI runtime
- `PluginHost` — Native plugin loading via libloading
- `ExtensionContext` — API exposed to extensions
- `ExtensionManifest` — Describes extension metadata

**Extension Types:**
- **RHAI Scripts:** Event handlers, data transforms, custom actions
- **Native Plugins:** Custom components, data sources, protocols

**Dependencies:**
- RHAI crate
- libloading crate
- Component Registry (for plugin components)
- Data Flow Engine (for plugin data sources)
- Event Bus (for plugin events)

---

## Cross-Cutting Concerns

### Error Handling

All subsystems follow a consistent error handling strategy:
- Errors are typed and informative
- Configuration errors include source location
- Runtime errors are recoverable where possible
- Errors are reported through the Event Bus
- User-facing errors are clear and actionable

### Observability

Built-in observability includes:
- Structured logging with configurable levels
- Metrics collection for performance monitoring
- Event tracing for debugging data flow
- Configuration-defined log sinks

### Lifecycle Management

Application lifecycle stages:
1. **Bootstrap:** Load core configuration, initialize subsystems
2. **Extension Load:** Discover and load scripts/plugins
3. **Configuration Load:** Parse and validate application config
4. **Layout Build:** Construct initial UI
5. **Run:** Event loop, data flow, user interaction
6. **Shutdown:** Graceful cleanup, state persistence

---

## Data Flow Diagram

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   External   │────▶│  Integration │────▶│    Data      │
│   Systems    │     │   Gateway    │     │   Sources    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                 │
                                                 ▼
                     ┌──────────────────────────────────────┐
                     │           Data Flow Engine           │
                     │  ┌─────────┐  ┌─────────┐  ┌──────┐  │
                     │  │Transform│─▶│Repository│─▶│Binding│ │
                     │  └─────────┘  └─────────┘  └──────┘  │
                     └──────────────────────────────────────┘
                                                 │
            ┌────────────────────────────────────┼────────────────────┐
            │                                    │                    │
            ▼                                    ▼                    ▼
   ┌──────────────┐                     ┌──────────────┐     ┌──────────────┐
   │    Event     │                     │    Layout    │     │    Action    │
   │     Bus      │◀───────────────────▶│    Engine    │     │   Triggers   │
   └──────────────┘                     └──────────────┘     └──────────────┘
            │                                    │
            ▼                                    ▼
   ┌──────────────┐                     ┌──────────────┐
   │  Extensions  │                     │     GPUI     │
   │(RHAI/Plugins)│                     │  Components  │
   └──────────────┘                     └──────────────┘
```

---

## Configuration Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  HCL Files  │────▶│Configuration│────▶│   Schema    │
│             │     │   Parser    │     │ Validation  │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────┐
                    │       Resolved Configuration        │
                    │  ┌──────────┐  ┌──────────────────┐ │
                    │  │ Layouts  │  │ Data Definitions │ │
                    │  └──────────┘  └──────────────────┘ │
                    │  ┌──────────┐  ┌──────────────────┐ │
                    │  │Components│  │    Extensions    │ │
                    │  └──────────┘  └──────────────────┘ │
                    └─────────────────────────────────────┘
                           │              │           │
                           ▼              ▼           ▼
                    ┌───────────┐  ┌───────────┐ ┌──────────┐
                    │  Layout   │  │ Data Flow │ │Extension │
                    │  Engine   │  │  Engine   │ │ Manager  │
                    └───────────┘  └───────────┘ └──────────┘
```

---

## Subsystem Interaction Matrix

| From \ To | Config | Layout | DataFlow | Registry | EventBus | Integration | Extension |
|-----------|--------|--------|----------|----------|----------|-------------|-----------|
| Config | - | Provides layout config | Provides data config | Provides schemas | Emits config events | Provides integration config | Provides extension config |
| Layout | Reads config | - | Binds to data | Looks up components | Emits UI events | - | Uses custom components |
| DataFlow | Reads config | Notifies bindings | - | - | Emits data events | Receives external data | Uses script transforms |
| Registry | Stores schemas | Provides factories | - | - | - | - | Registers plugin components |
| EventBus | - | Delivers events | Delivers events | - | - | Bridges external events | Delivers to extensions |
| Integration | Reads config | - | Provides data sources | - | Emits connection events | - | - |
| Extension | - | Provides components | Provides transforms | Registers components | Subscribes to events | - | - |

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
