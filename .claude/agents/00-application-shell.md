---
name: application-shell
description: Application Shell (Main Binary)
tools: Read, Glob, Grep
model: claude-sonnet-4-5
---

# Application Shell Specification

> **Component:** Application Shell (Main Binary)  
> **Priority:** 0 (Bootstrap)  
> **Dependencies:** All subsystems  
> **This is the executable that ties everything together**

---

## Overview

The Application Shell is the `nemo` binary—a minimal host that:
1. Provides a window with a titlebar
2. Initializes all subsystems
3. Loads configuration
4. Renders the configured layout

**The shell does almost nothing itself.** All functionality comes from configuration.

---

## Crate Structure

Create: `nemo` (main binary)

```
nemo/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # NemoApp (GPUI Application)
│   ├── root.rs              # Root component
│   ├── titlebar.rs          # TitleBar component
│   └── bootstrap.rs         # Subsystem initialization
└── assets/
    └── icons/               # Lucide icons for gpui-component
```

---

## Cargo.toml

```toml
[package]
name = "nemo"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nemo"
path = "src/main.rs"

[dependencies]
# GPUI
gpui = "0.2.2"
gpui-component = "0.5.1"

# Nemo subsystems
nemo-config = { path = "../nemo-config" }
nemo-registry = { path = "../nemo-registry" }
nemo-layout = { path = "../nemo-layout" }
nemo-data = { path = "../nemo-data" }
nemo-extension = { path = "../nemo-extension" }
nemo-integration = { path = "../nemo-integration" }
nemo-events = { path = "../nemo-events" }

# Runtime
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }

# Utils
anyhow = "1"
```

---

## main.rs

```rust
use clap::Parser;
use gpui::*;
use std::path::PathBuf;

mod app;
mod bootstrap;
mod root;
mod titlebar;

use app::NemoApp;

#[derive(Parser, Debug)]
#[command(name = "nemo", about = "Configuration-driven desktop applications")]
struct Args {
    /// Configuration file to load
    #[arg(short, long, default_value = "app.hcl")]
    config: PathBuf,
    
    /// Enable development mode (hot reload)
    #[arg(long)]
    dev: bool,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();
    
    tracing::info!("Starting Nemo with config: {:?}", args.config);
    
    // Start GPUI application
    let app = Application::new();
    
    app.run(move |cx| {
        // Initialize gpui-component
        gpui_component::init(cx);
        
        // Bootstrap Nemo subsystems
        let nemo = match bootstrap::initialize(cx, &args) {
            Ok(nemo) => nemo,
            Err(e) => {
                tracing::error!("Failed to initialize: {}", e);
                cx.quit();
                return;
            }
        };
        
        // Open main window
        cx.spawn(|cx| async move {
            let options = WindowOptions {
                titlebar: None,  // We provide custom titlebar
                focus: true,
                show: true,
                kind: WindowKind::Normal,
                bounds: WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1200.), px(800.)),
                }),
                ..Default::default()
            };
            
            cx.open_window(options, |window, cx| {
                let root = cx.new(|cx| root::Root::new(nemo, window, cx));
                cx.new(|cx| gpui_component::Root::new(root, window, cx))
            })?;
            
            Ok::<_, anyhow::Error>(())
        }).detach();
    });
}
```

---

## bootstrap.rs

```rust
use std::sync::Arc;
use gpui::Context;
use anyhow::Result;

use nemo_config::{ConfigurationLoader, SchemaRegistry};
use nemo_registry::ComponentRegistry;
use nemo_layout::{LayoutBuilder, LayoutManager, BindingManager};
use nemo_data::DataFlowEngine;
use nemo_extension::ExtensionManager;
use nemo_integration::IntegrationGateway;
use nemo_events::EventBus;

use crate::Args;

pub struct Nemo {
    pub config_loader: Arc<ConfigurationLoader>,
    pub registry: Arc<ComponentRegistry>,
    pub layout_manager: LayoutManager,
    pub data_engine: Arc<DataFlowEngine>,
    pub extension_manager: Arc<ExtensionManager>,
    pub integration_gateway: Arc<IntegrationGateway>,
    pub event_bus: Arc<EventBus>,
    pub resolved_config: nemo_config::ResolvedConfig,
}

pub fn initialize(cx: &mut Context<'_>, args: &Args) -> Result<Nemo> {
    tracing::info!("Initializing Nemo subsystems...");
    
    // 1. Event Bus (needed by others)
    let event_bus = Arc::new(EventBus::new(4096));
    tracing::debug!("Event Bus initialized");
    
    // 2. Schema Registry
    let schema_registry = Arc::new(SchemaRegistry::new());
    
    // 3. Component Registry (with built-ins)
    let registry = Arc::new(ComponentRegistry::with_builtins());
    
    // Register schemas with schema registry
    for (entity_type, name) in registry.list_all() {
        if let Some(schema) = registry.get_schema(entity_type, &name) {
            schema_registry.register(schema)?;
        }
    }
    tracing::debug!("Component Registry initialized with {} components", 
        registry.list_components().len());
    
    // 4. Configuration Loader
    let config_loader = Arc::new(ConfigurationLoader::new(schema_registry.clone()));
    
    // 5. Load configuration
    let resolved_config = config_loader.load(&args.config)?;
    tracing::info!("Configuration loaded from {:?}", args.config);
    
    if let Some(app) = &resolved_config.application {
        tracing::info!("Application: {} v{}", app.name, app.version);
    }
    
    // 6. Data Flow Engine
    let data_engine = Arc::new(DataFlowEngine::new(event_bus.clone()));
    
    // 7. Integration Gateway
    let integration_gateway = Arc::new(IntegrationGateway::new(event_bus.clone()));
    
    // 8. Extension Manager
    let extension_context = nemo_extension::ExtensionContext::new(
        data_engine.repository(),
        event_bus.clone(),
        Arc::new(resolved_config.clone()),
    );
    let extension_manager = Arc::new(ExtensionManager::new(Arc::new(extension_context)));
    
    // 9. Register Layout Engine factories
    nemo_layout::register_component_factories(&registry);
    tracing::debug!("Layout factories registered");
    
    // 10. Binding Manager
    let binding_manager = Arc::new(BindingManager::new());
    
    // 11. Layout Builder and Manager
    let layout_builder = LayoutBuilder::new(registry.clone(), binding_manager.clone());
    let layout_manager = LayoutManager::new(layout_builder);
    
    // 12. Load extensions from config
    for ext in &resolved_config.extensions {
        match ext.extension_type.as_str() {
            "script" => {
                if let Err(e) = extension_manager.load_script(&ext.path) {
                    tracing::warn!("Failed to load script {:?}: {}", ext.path, e);
                }
            }
            "plugin" => {
                if let Err(e) = unsafe { extension_manager.load_plugin(&ext.path) } {
                    tracing::warn!("Failed to load plugin {:?}: {}", ext.path, e);
                }
            }
            _ => {
                tracing::warn!("Unknown extension type: {}", ext.extension_type);
            }
        }
    }
    
    // 13. Set up integrations from config
    for integration in &resolved_config.integrations {
        if let Err(e) = integration_gateway.add_from_config(integration) {
            tracing::warn!("Failed to add integration {}: {}", integration.id, e);
        }
    }
    
    // 14. Set up data sources from config
    for data_source in &resolved_config.data_sources {
        if let Err(e) = data_engine.add_from_config(data_source) {
            tracing::warn!("Failed to add data source {}: {}", data_source.id, e);
        }
    }
    
    // 15. Start data flow
    tokio::spawn({
        let engine = data_engine.clone();
        async move {
            if let Err(e) = engine.start().await {
                tracing::error!("Data flow engine error: {}", e);
            }
        }
    });
    
    // 16. If dev mode, set up hot reload
    if args.dev {
        tracing::info!("Development mode enabled - hot reload active");
        // Set up file watcher
    }
    
    tracing::info!("Nemo initialization complete");
    
    // Emit startup event
    event_bus.emit_simple(nemo_events::events::SYSTEM_STARTUP, serde_json::json!({
        "config_path": args.config.to_string_lossy(),
        "dev_mode": args.dev,
    }));
    
    Ok(Nemo {
        config_loader,
        registry,
        layout_manager,
        data_engine,
        extension_manager,
        integration_gateway,
        event_bus,
        resolved_config,
    })
}
```

---

## root.rs

```rust
use gpui::*;
use gpui_component::dock::DockArea;

use crate::titlebar::TitleBar;
use crate::bootstrap::Nemo;

pub struct Root {
    nemo: Nemo,
    titlebar: Entity<TitleBar>,
    content: Option<Entity<DockArea>>,
}

impl Root {
    pub fn new(mut nemo: Nemo, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create titlebar
        let app_name = nemo.resolved_config.application
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Nemo".to_string());
        
        let titlebar = cx.new(|_| TitleBar::new(app_name));
        
        // Build layout from configuration
        let content = if let Some(layout_config) = &nemo.resolved_config.layout {
            match nemo.layout_manager.initialize(layout_config, window, cx) {
                Ok(()) => nemo.layout_manager.dock_area().cloned(),
                Err(e) => {
                    tracing::error!("Failed to build layout: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("No layout configured");
            None
        };
        
        Self {
            nemo,
            titlebar,
            content,
        }
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            // Titlebar
            .child(self.titlebar.clone())
            // Content area
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(if let Some(content) = &self.content {
                        content.clone().into_any_element()
                    } else {
                        // Empty state
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("No layout configured")
                            .into_any_element()
                    })
            )
    }
}
```

---

## titlebar.rs

```rust
use gpui::*;
use gpui_component::titlebar::{TitleBar as GpuiTitleBar, TrafficLights};

pub struct TitleBar {
    title: String,
}

impl TitleBar {
    pub fn new(title: String) -> Self {
        Self { title }
    }
}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        GpuiTitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(TrafficLights::new())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(self.title.clone())
                    )
            )
    }
}
```

---

## Minimal Configuration

The simplest valid Nemo configuration:

```hcl
# minimal.hcl
application {
  name = "My App"
}

layout {
  type = "dock"
  
  center {
    panel "main" {
      component = "label"
      config {
        text = "Hello from Nemo!"
        size = "xl"
      }
    }
  }
}
```

---

## Running Nemo

```bash
# Run with default config (app.hcl)
cargo run

# Run with specific config
cargo run -- --config my-app.hcl

# Development mode with hot reload
cargo run -- --config my-app.hcl --dev

# With debug logging
cargo run -- --config my-app.hcl --log-level debug
```

---

## Project Structure (Complete)

```
nemo/
├── Cargo.toml              # Workspace
├── nemo/                   # Main binary
├── nemo-config/            # Configuration Engine
├── nemo-registry/          # Component Registry
├── nemo-layout/            # Layout Engine
├── nemo-data/              # Data Flow Engine
├── nemo-extension/         # Extension Manager
├── nemo-integration/       # Integration Gateway
├── nemo-events/            # Event Bus
└── nemo-plugin-api/        # Plugin Author API
```

Workspace `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "nemo",
    "nemo-config",
    "nemo-registry",
    "nemo-layout",
    "nemo-data",
    "nemo-extension",
    "nemo-integration",
    "nemo-events",
    "nemo-plugin-api",
]

[workspace.dependencies]
gpui = "0.2.2"
gpui-component = "0.5.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
tracing = "0.1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
```

---

## Success Criteria

- [ ] `cargo run` starts the application
- [ ] Titlebar displays with traffic lights
- [ ] Configuration loads and validates
- [ ] Layout renders from configuration
- [ ] Hot reload works in dev mode
- [ ] Graceful error handling for invalid configs
