# Nemo Architecture Diagrams

> **Project:** Nemo - Configuration-Driven Application Framework  
> **Created:** February 2026  
> **Location:** `~/Projects/nemo/`

This document contains Mermaid diagrams illustrating Nemo's system architecture at various levels of detail.

---

## 1. System Context Diagram

Shows Nemo's position in relation to external systems and users.

```mermaid
C4Context
    title Nemo System Context

    Person(developer, "Application Developer", "Writes HCL config, RHAI scripts, and native plugins")
    Person(enduser, "End User", "Uses Nemo-built applications")
    
    System(nemo, "Nemo Framework", "Configuration-driven desktop application framework built on GPUI")
    
    System_Ext(http_api, "HTTP APIs", "REST/JSON endpoints")
    System_Ext(websocket, "WebSocket Services", "Real-time data streams")
    System_Ext(mqtt, "MQTT Broker", "IoT/messaging")
    System_Ext(redis, "Redis", "Caching and pub/sub")
    System_Ext(nats, "NATS", "Cloud messaging")
    System_Ext(filesystem, "File System", "Config files, plugins, data")

    Rel(developer, nemo, "Configures via HCL, extends via RHAI/plugins")
    Rel(enduser, nemo, "Interacts with built applications")
    Rel(nemo, http_api, "Fetches data, calls APIs")
    Rel(nemo, websocket, "Streams real-time data")
    Rel(nemo, mqtt, "Subscribes/publishes messages")
    Rel(nemo, redis, "Caches data, pub/sub")
    Rel(nemo, nats, "Cloud messaging")
    Rel(nemo, filesystem, "Reads config, loads plugins")
```

---

## 2. Container Diagram (Crate Structure)

Shows the Rust crate organization and dependencies.

```mermaid
graph TB
    subgraph "Application Layer"
        nemo[nemo<br/>Application Shell]
    end
    
    subgraph "Core Layer"
        config[nemo-config<br/>Configuration Engine]
        layout[nemo-layout<br/>Layout Engine]
        data[nemo-data<br/>Data Flow Engine]
        registry[nemo-registry<br/>Component Registry]
        events[nemo-events<br/>Event Bus]
    end
    
    subgraph "Integration Layer"
        integration[nemo-integration<br/>Integration Gateway]
        extension[nemo-extension<br/>Extension Manager]
    end
    
    subgraph "Plugin API"
        plugin_api[nemo-plugin-api<br/>Plugin Author API]
        macros[nemo-macros<br/>Proc Macros]
    end
    
    subgraph "Foundation"
        gpui[GPUI<br/>GPU UI Framework]
        gpui_comp[gpui-component<br/>Component Library]
        rhai[RHAI<br/>Scripting Engine]
        libloading[libloading<br/>Dynamic Loading]
    end
    
    nemo --> config
    nemo --> layout
    nemo --> data
    nemo --> events
    nemo --> extension
    nemo --> integration
    
    layout --> config
    layout --> registry
    layout --> data
    layout --> events
    
    data --> events
    data --> integration
    
    extension --> plugin_api
    extension --> rhai
    extension --> libloading
    extension --> registry
    extension --> events
    
    registry --> config
    
    integration --> events
    
    plugin_api --> macros
    
    nemo --> gpui
    layout --> gpui_comp
    
    style nemo fill:#e1f5fe
    style config fill:#fff3e0
    style layout fill:#fff3e0
    style data fill:#fff3e0
    style registry fill:#fff3e0
    style events fill:#fff3e0
    style integration fill:#e8f5e9
    style extension fill:#e8f5e9
    style plugin_api fill:#fce4ec
    style macros fill:#fce4ec
```

---

## 3. Subsystem Architecture

### 3.1 Configuration Engine

```mermaid
flowchart TB
    subgraph Input
        hcl_files[HCL Files]
        env_vars[Environment Variables]
        cli_args[CLI Arguments]
    end
    
    subgraph "Configuration Engine"
        loader[ConfigurationLoader]
        parser[HCL Parser<br/>hcl-rs]
        validator[SchemaValidator]
        resolver[ExpressionResolver]
        watcher[FileWatcher<br/>notify crate]
        
        loader --> parser
        parser --> validator
        validator --> resolver
        watcher -.->|hot reload| loader
    end
    
    subgraph Output
        resolved[ResolvedConfiguration]
        layout_cfg[Layout Config]
        data_cfg[Data Config]
        ext_cfg[Extension Config]
        int_cfg[Integration Config]
    end
    
    hcl_files --> loader
    env_vars --> resolver
    cli_args --> resolver
    
    resolver --> resolved
    resolved --> layout_cfg
    resolved --> data_cfg
    resolved --> ext_cfg
    resolved --> int_cfg
    
    subgraph "Schema Registry"
        builtin[Built-in Schemas]
        plugin[Plugin Schemas]
    end
    
    builtin --> validator
    plugin --> validator
```

### 3.2 Layout Engine

```mermaid
flowchart TB
    subgraph Input
        layout_cfg[Layout Configuration]
        registry[Component Registry]
        data_bindings[Data Bindings]
    end
    
    subgraph "Layout Engine"
        builder[LayoutBuilder]
        factory[ComponentFactory]
        tree[Component Tree]
        binder[BindingResolver]
        
        builder -->|creates| tree
        factory -->|instantiates| tree
        binder -->|connects| tree
    end
    
    subgraph "GPUI Integration"
        root[NemoRootView]
        render[render() method]
        gpui_tree[GPUI Element Tree]
    end
    
    layout_cfg --> builder
    registry --> factory
    data_bindings --> binder
    
    tree --> root
    root --> render
    render --> gpui_tree
    
    subgraph "Component Types"
        stack[Stack/Flex]
        panel[Panel]
        label[Label]
        button[Button]
        input[Input]
        table[Table]
        custom[Custom Plugin]
    end
    
    factory -.-> stack
    factory -.-> panel
    factory -.-> label
    factory -.-> button
    factory -.-> input
    factory -.-> table
    factory -.-> custom
```

### 3.3 Data Flow Engine

```mermaid
flowchart LR
    subgraph Sources["Data Sources"]
        http[HTTP Polling]
        ws[WebSocket Stream]
        file[File Watcher]
        timer[Timer]
        static[Static Data]
    end
    
    subgraph Transforms["Transformers"]
        map[Map]
        filter[Filter]
        agg[Aggregate]
        join[Join]
        script[RHAI Script]
    end
    
    subgraph Storage["Repositories"]
        memory[In-Memory Store]
        cache[LRU Cache]
    end
    
    subgraph Bindings["Data Bindings"]
        oneway[One-Way Binding]
        twoway[Two-Way Binding]
        computed[Computed Binding]
    end
    
    subgraph Output["Consumers"]
        ui[UI Components]
        actions[Action Triggers]
        events[Event Bus]
    end
    
    Sources --> Transforms
    Transforms --> Storage
    Storage --> Bindings
    Bindings --> Output
```

### 3.4 Event Bus

```mermaid
flowchart TB
    subgraph Publishers
        ui_pub[UI Events]
        data_pub[Data Events]
        sys_pub[System Events]
        ext_pub[Extension Events]
    end
    
    subgraph "Event Bus Core"
        dispatcher[EventDispatcher]
        filter[EventFilter]
        router[EventRouter]
        log[EventLog]
        
        dispatcher --> filter
        filter --> router
        dispatcher --> log
    end
    
    subgraph Subscribers
        layout_sub[Layout Engine]
        data_sub[Data Flow Engine]
        ext_sub[Extensions]
        int_sub[Integration Gateway]
    end
    
    Publishers --> dispatcher
    router --> Subscribers
    
    subgraph "Event Types"
        direction LR
        click[ClickEvent]
        change[DataChangeEvent]
        error[ErrorEvent]
        lifecycle[LifecycleEvent]
        custom[CustomEvent]
    end
```

### 3.5 Integration Gateway

```mermaid
flowchart TB
    subgraph "Protocol Adapters"
        http_client[HttpClient<br/>reqwest]
        ws_client[WebSocketClient<br/>tokio-tungstenite]
        mqtt_client[MqttClient<br/>rumqttc]
        redis_client[RedisClient<br/>redis-rs]
        nats_client[NatsClient<br/>async-nats]
    end
    
    subgraph "Gateway Core"
        gateway[IntegrationGateway]
        conn_mgr[ConnectionManager]
        health[HealthChecker]
        circuit[CircuitBreaker<br/>NOT IMPLEMENTED]
        retry[RetryStrategy<br/>NOT IMPLEMENTED]
        
        gateway --> conn_mgr
        conn_mgr --> health
        conn_mgr -.-> circuit
        conn_mgr -.-> retry
    end
    
    subgraph "External Systems"
        rest_api[REST APIs]
        ws_server[WebSocket Servers]
        mqtt_broker[MQTT Brokers]
        redis_server[Redis Server]
        nats_server[NATS Server]
    end
    
    http_client --> rest_api
    ws_client --> ws_server
    mqtt_client --> mqtt_broker
    redis_client --> redis_server
    nats_client --> nats_server
    
    gateway --> http_client
    gateway --> ws_client
    gateway --> mqtt_client
    gateway --> redis_client
    gateway --> nats_client
    
    style circuit fill:#ffcdd2,stroke:#c62828
    style retry fill:#ffcdd2,stroke:#c62828
```

### 3.6 Extension Manager

```mermaid
flowchart TB
    subgraph "Extension Discovery"
        script_dir[Script Directory]
        plugin_dir[Plugin Directory]
        manifest[Extension Manifests]
    end
    
    subgraph "RHAI Runtime"
        rhai_engine[RhaiEngine]
        sandbox[Sandbox Config]
        stdlib[Standard Functions]
        context_api[Context API]
        
        rhai_engine --> sandbox
        rhai_engine --> stdlib
        rhai_engine --> context_api
    end
    
    subgraph "Plugin Host"
        plugin_host[PluginHost]
        libload[libloading]
        plugin_registry[Plugin Registry]
        
        plugin_host --> libload
        plugin_host --> plugin_registry
    end
    
    subgraph "Extension Manager"
        ext_mgr[ExtensionManager]
        loader[ExtensionLoader]
        registry[ExtensionRegistry]
        
        ext_mgr --> loader
        ext_mgr --> registry
        loader --> rhai_engine
        loader --> plugin_host
    end
    
    script_dir --> loader
    plugin_dir --> loader
    manifest --> loader
    
    subgraph "Capabilities"
        comp_reg[Register Components]
        data_src[Provide Data Sources]
        transforms[Provide Transforms]
        actions[Provide Actions]
        handlers[Event Handlers]
    end
    
    registry --> comp_reg
    registry --> data_src
    registry --> transforms
    registry --> actions
    registry --> handlers
```

---

## 4. Data Flow Sequence

Shows how data flows from external source to UI update.

```mermaid
sequenceDiagram
    participant Ext as External API
    participant IG as Integration Gateway
    participant DF as Data Flow Engine
    participant Repo as Repository
    participant EB as Event Bus
    participant LE as Layout Engine
    participant UI as GPUI Component

    Ext->>IG: HTTP Response / WS Message
    IG->>DF: Raw Data
    DF->>DF: Transform (map, filter, etc.)
    DF->>Repo: Store Transformed Data
    Repo->>EB: Emit DataChanged Event
    EB->>LE: Notify Binding Update
    LE->>UI: Update Component Props
    UI->>UI: Re-render
```

---

## 5. Configuration Loading Sequence

```mermaid
sequenceDiagram
    participant CLI as CLI Args
    participant Loader as ConfigLoader
    participant Parser as HCL Parser
    participant Schema as SchemaRegistry
    participant Validator as Validator
    participant Resolver as Resolver
    participant App as Application

    CLI->>Loader: config path
    Loader->>Parser: parse HCL files
    Parser->>Loader: AST
    Loader->>Schema: get schemas
    Schema->>Loader: component schemas
    Loader->>Validator: validate config
    Validator->>Loader: validation result
    
    alt Validation Failed
        Loader->>CLI: Error with location
    else Validation Passed
        Loader->>Resolver: resolve expressions
        Resolver->>Loader: resolved config
        Loader->>App: ResolvedConfiguration
    end
```

---

## 6. Extension Loading Sequence

```mermaid
sequenceDiagram
    participant App as Application
    participant EM as ExtensionManager
    participant Loader as ExtensionLoader
    participant RHAI as RhaiEngine
    participant Plugin as PluginHost
    participant Registry as ComponentRegistry

    App->>EM: initialize(extension_dirs)
    EM->>Loader: discover extensions
    
    loop For each .rhai file
        Loader->>RHAI: load_script(path)
        RHAI->>RHAI: compile & validate
        RHAI->>EM: Script loaded
    end
    
    loop For each .so/.dylib file
        Loader->>Plugin: load_plugin(path)
        Plugin->>Plugin: dlopen & get manifest
        Plugin->>Registry: register components
        Plugin->>EM: Plugin loaded
    end
    
    EM->>App: Extensions ready
```

---

## 7. Application Bootstrap Sequence

```mermaid
sequenceDiagram
    participant Main as main()
    participant CLI as Clap CLI
    participant Runtime as NemoRuntime
    participant Config as ConfigEngine
    participant Events as EventBus
    participant Registry as ComponentRegistry
    participant Extensions as ExtensionManager
    participant Integration as IntegrationGateway
    participant Data as DataFlowEngine
    participant Layout as LayoutEngine
    participant App as NemoApp
    participant GPUI as GPUI

    Main->>CLI: parse args
    CLI->>Runtime: new(args)
    
    Runtime->>Events: initialize()
    Runtime->>Config: load(config_path)
    Runtime->>Registry: initialize(schemas)
    Runtime->>Extensions: load(extension_dirs)
    Extensions->>Registry: register plugin components
    
    Runtime->>Integration: initialize(config)
    Runtime->>Data: initialize(config, integration)
    Runtime->>Layout: build(config, registry, data)
    
    Runtime->>App: new(runtime)
    App->>GPUI: run_app()
    GPUI->>App: render loop
```

---

## 8. Component Hierarchy (Typical Application)

```mermaid
graph TB
    subgraph "Window"
        root[NemoRootView]
        
        subgraph "Layout"
            main_stack[Stack - Vertical]
            
            subgraph "Header"
                header_panel[Panel]
                title[Label - Title]
                toolbar[Stack - Horizontal]
                btn1[Button - Action 1]
                btn2[Button - Action 2]
            end
            
            subgraph "Content"
                content_panel[Panel - Flex Grow]
                sidebar[Stack - Sidebar]
                main[Stack - Main Content]
                table[Table - Data]
            end
            
            subgraph "Footer"
                footer_panel[Panel]
                status[Label - Status]
            end
        end
    end
    
    root --> main_stack
    main_stack --> header_panel
    main_stack --> content_panel
    main_stack --> footer_panel
    
    header_panel --> title
    header_panel --> toolbar
    toolbar --> btn1
    toolbar --> btn2
    
    content_panel --> sidebar
    content_panel --> main
    main --> table
    
    footer_panel --> status
```

---

## 9. State Management

```mermaid
stateDiagram-v2
    [*] --> Initializing: App Start
    
    Initializing --> LoadingConfig: Bootstrap Complete
    LoadingConfig --> ValidatingConfig: Config Parsed
    
    ValidatingConfig --> ConfigError: Validation Failed
    ValidatingConfig --> LoadingExtensions: Validation Passed
    
    ConfigError --> [*]: Exit with Error
    
    LoadingExtensions --> BuildingLayout: Extensions Loaded
    BuildingLayout --> Running: Layout Built
    
    Running --> Running: Event Loop
    Running --> Reloading: Hot Reload Triggered
    
    Reloading --> LoadingConfig: Config Changed
    Reloading --> Running: Reload Complete
    
    Running --> ShuttingDown: Shutdown Signal
    ShuttingDown --> [*]: Cleanup Complete
```

---

## 10. Error Handling Flow

```mermaid
flowchart TB
    subgraph "Error Sources"
        config_err[Config Parsing Error]
        validation_err[Schema Validation Error]
        runtime_err[Runtime Error]
        plugin_err[Plugin Error]
        network_err[Network Error]
    end
    
    subgraph "Error Handling"
        handler[Error Handler]
        categorize[Categorize Error]
        
        recoverable{Recoverable?}
        
        log[Log Error]
        notify[Notify User]
        retry[Retry Operation]
        fallback[Use Fallback]
        crash[Crash with Report]
    end
    
    config_err --> handler
    validation_err --> handler
    runtime_err --> handler
    plugin_err --> handler
    network_err --> handler
    
    handler --> categorize
    categorize --> recoverable
    
    recoverable -->|Yes| log
    recoverable -->|No| crash
    
    log --> notify
    notify --> retry
    retry --> fallback
    
    subgraph "Event Bus"
        error_event[ErrorEvent]
    end
    
    handler --> error_event
```

---

## 11. Plugin API Architecture

```mermaid
classDiagram
    class PluginManifest {
        +String name
        +Version version
        +String description
        +Vec~Capability~ capabilities
        +PluginPermissions permissions
    }
    
    class PluginRegistrar {
        <<trait>>
        +register_component(schema)
        +register_data_source(schema)
        +register_transform(schema)
        +register_action(schema)
    }
    
    class PluginContext {
        <<trait>>
        +get_data(key) PluginValue
        +get_config(key) PluginValue
        +emit_event(event)
        +log(level, message)
    }
    
    class ComponentSchema {
        +String name
        +String description
        +Vec~PropertySchema~ properties
        +Vec~String~ slots
    }
    
    class PropertySchema {
        +String name
        +PropertyType type
        +bool required
        +Option~PluginValue~ default
    }
    
    class PluginValue {
        <<enum>>
        Null
        Bool(bool)
        Int(i64)
        Float(f64)
        String(String)
        Array(Vec)
        Object(HashMap)
    }
    
    PluginManifest --> PluginRegistrar
    PluginRegistrar --> ComponentSchema
    ComponentSchema --> PropertySchema
    PropertySchema --> PluginValue
    PluginContext --> PluginValue
```

---

## 12. Deployment View

```mermaid
graph TB
    subgraph "Developer Machine"
        dev_code[Application Code<br/>HCL + RHAI + Plugins]
        nemo_dev[Nemo Framework]
        
        dev_code --> nemo_dev
    end
    
    subgraph "Build Output"
        app_bundle[Application Bundle]
        
        subgraph "Bundle Contents"
            binary[nemo binary]
            config_dir[config/]
            plugins_dir[plugins/]
            scripts_dir[scripts/]
            assets_dir[assets/]
        end
        
        app_bundle --> binary
        app_bundle --> config_dir
        app_bundle --> plugins_dir
        app_bundle --> scripts_dir
        app_bundle --> assets_dir
    end
    
    subgraph "Target Platforms"
        macos[macOS .app]
        linux[Linux Binary]
        windows[Windows .exe]
    end
    
    nemo_dev -->|cargo build| app_bundle
    app_bundle -->|package| macos
    app_bundle -->|package| linux
    app_bundle -->|package| windows
```

---

## Diagram Legend

| Symbol | Meaning |
|--------|---------|
| Solid arrow (→) | Direct dependency or data flow |
| Dashed arrow (⤍) | Optional or planned feature |
| Red fill | Not implemented / Gap |
| Blue fill | Application layer |
| Orange fill | Core layer |
| Green fill | Integration layer |
| Pink fill | Plugin API layer |

---

## References

- **Code Location:** `~/Projects/nemo/`
- **Architecture Doc:** `docs/planning/nemo-system-architecture.md`
- **Code Review:** `personas/kb/systems-designer/nemo-code-review.md`
- **Subsystem Docs:** `docs/planning/subsystems/`
