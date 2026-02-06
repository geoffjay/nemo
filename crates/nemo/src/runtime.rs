//! Nemo runtime - manages all subsystems.

use anyhow::{Context, Result};
use nemo_config::{ConfigurationLoader, SchemaRegistry, Value};
use nemo_data::DataFlowEngine;
use nemo_events::EventBus;
use nemo_extension::ExtensionManager;
use nemo_integration::IntegrationGateway;
use nemo_layout::{LayoutConfig, LayoutManager, LayoutNode, LayoutType};
use nemo_registry::{register_all_builtins, ComponentRegistry};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// The Nemo runtime manages all subsystems.
pub struct NemoRuntime {
    /// Main configuration file path.
    config_path: PathBuf,
    /// Additional configuration directories.
    config_dirs: Vec<PathBuf>,
    /// Extension directories.
    extension_dirs: Vec<PathBuf>,
    /// The event bus.
    pub event_bus: Arc<EventBus>,
    /// The schema registry.
    pub schema_registry: Arc<SchemaRegistry>,
    /// The configuration loader.
    pub config_loader: ConfigurationLoader,
    /// Loaded configuration.
    pub config: Arc<RwLock<Value>>,
    /// The component registry.
    pub registry: Arc<ComponentRegistry>,
    /// The layout manager.
    pub layout_manager: Arc<RwLock<LayoutManager>>,
    /// The data flow engine.
    pub data_engine: Arc<DataFlowEngine>,
    /// The extension manager.
    pub extension_manager: Arc<RwLock<ExtensionManager>>,
    /// The integration gateway.
    pub integration: Arc<IntegrationGateway>,
    /// The tokio runtime for async operations.
    pub tokio_runtime: TokioRuntime,
}

impl NemoRuntime {
    /// Creates a new Nemo runtime.
    pub fn new(config_path: &Path) -> Result<Self> {
        let tokio_runtime = TokioRuntime::new().context("Failed to create tokio runtime")?;

        let event_bus = Arc::new(EventBus::with_default_capacity());
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);

        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(Arc::clone(&registry))));
        let data_engine = Arc::new(DataFlowEngine::new());
        let extension_manager = Arc::new(RwLock::new(ExtensionManager::new()));
        let integration = Arc::new(IntegrationGateway::new());
        let schema_registry = Arc::new(SchemaRegistry::new());
        let config_loader = ConfigurationLoader::new(Arc::clone(&schema_registry));
        let config = Arc::new(RwLock::new(Value::Null));

        Ok(Self {
            config_path: config_path.to_path_buf(),
            config_dirs: Vec::new(),
            extension_dirs: Vec::new(),
            event_bus,
            schema_registry,
            config_loader,
            config,
            registry,
            layout_manager,
            data_engine,
            extension_manager,
            integration,
            tokio_runtime,
        })
    }

    /// Adds a configuration directory.
    pub fn add_config_dir(&self, dir: &Path) -> Result<()> {
        debug!("Adding config directory: {:?}", dir);
        Ok(())
    }

    /// Adds an extension directory.
    pub fn add_extension_dir(&self, dir: &Path) -> Result<()> {
        debug!("Adding extension directory: {:?}", dir);
        self.tokio_runtime.block_on(async {
            let mut ext = self.extension_manager.write().await;
            ext.add_script_path(dir.join("scripts"));
            ext.add_plugin_path(dir.join("plugins"));
        });
        Ok(())
    }

    /// Loads configuration from files.
    pub fn load_config(&self) -> Result<()> {
        info!("Loading configuration...");

        if self.config_path.exists() {
            let loaded = self.config_loader.load(&self.config_path).map_err(|e| {
                anyhow::anyhow!("Failed to load config file: {}", e)
            })?;

            self.tokio_runtime.block_on(async {
                let mut config = self.config.write().await;
                *config = loaded;
            });
        } else {
            debug!(
                "Config file {:?} not found, using defaults",
                self.config_path
            );
        }

        info!("Configuration loaded successfully");
        Ok(())
    }

    /// Initializes all subsystems.
    pub fn initialize(&self) -> Result<()> {
        info!("Initializing Nemo runtime...");

        self.tokio_runtime.block_on(async {
            // Initialize extensions
            let ext = self.extension_manager.read().await;
            let manifests = ext.discover().unwrap_or_default();
            info!("Discovered {} extensions", manifests.len());
            drop(ext);

            // Load discovered scripts
            let mut ext = self.extension_manager.write().await;
            for manifest in manifests {
                match manifest.extension_type {
                    nemo_extension::ExtensionType::Script => {
                        if let Err(e) = ext.load_script(&manifest.path) {
                            tracing::warn!("Failed to load script {:?}: {}", manifest.path, e);
                        }
                    }
                    nemo_extension::ExtensionType::Plugin => {
                        if let Err(e) = ext.load_plugin(&manifest.path) {
                            tracing::warn!("Failed to load plugin {:?}: {}", manifest.path, e);
                        }
                    }
                }
            }
            drop(ext);

            // Set up event subscriptions
            self.setup_event_handlers().await;

            Ok::<(), anyhow::Error>(())
        })?;

        // Load scripts from configuration
        self.load_scripts_from_config()?;

        // Apply layout from configuration
        self.apply_layout_from_config()?;

        info!("Runtime initialization complete");
        Ok(())
    }

    /// Loads scripts specified in configuration.
    fn load_scripts_from_config(&self) -> Result<()> {
        let scripts_config = self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            config.get("scripts").cloned()
        });

        if let Some(scripts) = scripts_config {
            // Handle scripts.path for directory-based loading
            if let Some(path_str) = scripts.get("path").and_then(|v| v.as_str()) {
                let scripts_path = if path_str.starts_with("./") || path_str.starts_with("../") {
                    // Relative to config file
                    self.config_path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join(path_str)
                } else {
                    std::path::PathBuf::from(path_str)
                };

                if scripts_path.exists() && scripts_path.is_dir() {
                    info!("Loading scripts from: {:?}", scripts_path);
                    self.tokio_runtime.block_on(async {
                        let mut ext = self.extension_manager.write().await;
                        ext.add_script_path(&scripts_path);

                        // Load all .rhai files in the directory
                        if let Ok(entries) = std::fs::read_dir(&scripts_path) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().map(|e| e == "rhai").unwrap_or(false) {
                                    match ext.load_script(&path) {
                                        Ok(id) => info!("Loaded script: {}", id),
                                        Err(e) => {
                                            tracing::warn!("Failed to load script {:?}: {}", path, e)
                                        }
                                    }
                                }
                            }
                        }
                    });
                } else {
                    debug!("Scripts path does not exist: {:?}", scripts_path);
                }
            }

            // Handle individual script files specified in the config
            if let Some(files) = scripts.get("files").and_then(|v| v.as_array()) {
                for file_value in files {
                    if let Some(file_path) = file_value.as_str() {
                        let script_path = if file_path.starts_with("./") || file_path.starts_with("../") {
                            self.config_path
                                .parent()
                                .unwrap_or(std::path::Path::new("."))
                                .join(file_path)
                        } else {
                            std::path::PathBuf::from(file_path)
                        };

                        if script_path.exists() {
                            self.tokio_runtime.block_on(async {
                                let mut ext = self.extension_manager.write().await;
                                match ext.load_script(&script_path) {
                                    Ok(id) => info!("Loaded script: {}", id),
                                    Err(e) => {
                                        tracing::warn!("Failed to load script {:?}: {}", script_path, e)
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Sets up internal event handlers.
    async fn setup_event_handlers(&self) {
        // Subscribe to configuration changes
        let _config = Arc::clone(&self.config);
        let mut config_sub = self.event_bus.subscribe_type("config.changed");

        tokio::spawn(async move {
            while let Some(event) = config_sub.recv().await {
                debug!("Configuration changed: {:?}", event);
            }
        });
    }

    /// Runs in headless mode (no UI).
    pub fn run_headless(&self) -> Result<()> {
        info!("Running in headless mode...");

        self.tokio_runtime.block_on(async {
            tokio::signal::ctrl_c()
                .await
                .context("Failed to listen for ctrl-c")?;
            info!("Received shutdown signal");
            Ok(())
        })
    }

    /// Returns the event bus.
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// Returns the component registry.
    pub fn registry(&self) -> &Arc<ComponentRegistry> {
        &self.registry
    }

    /// Gets a configuration value by path.
    pub fn get_config(&self, path: &str) -> Option<Value> {
        self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            get_nested_value(&config, path).cloned()
        })
    }

    /// Sets a configuration value (not implemented - config is read-only).
    pub fn set_config(&self, _path: &str, _value: Value) -> Result<()> {
        // Configuration is typically read-only after loading
        Ok(())
    }

    /// Calls an event handler.
    ///
    /// Handler format: "script_id::function_name" or just "function_name" (uses default script)
    pub fn call_handler(&self, handler: &str, component_id: &str, event_data: &str) {
        // Parse handler format: "script_id::function_name" or "function_name"
        let (script_id, function_name) = if let Some(pos) = handler.find("::") {
            (&handler[..pos], &handler[pos + 2..])
        } else {
            // Default to "handlers" script if no script specified
            ("handlers", handler)
        };

        debug!(
            "Calling handler: {}::{} for component {} with data: {}",
            script_id, function_name, component_id, event_data
        );

        self.tokio_runtime.block_on(async {
            let ext = self.extension_manager.read().await;
            match ext.call_script::<()>(
                script_id,
                function_name,
                (component_id.to_string(), event_data.to_string()),
            ) {
                Ok(_) => debug!("Handler {}::{} executed successfully", script_id, function_name),
                Err(e) => tracing::warn!(
                    "Handler {}::{} failed: {}",
                    script_id,
                    function_name,
                    e
                ),
            }
        });
    }

    /// Parses and applies the layout configuration.
    pub fn apply_layout_from_config(&self) -> Result<()> {
        let layout_config = self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            parse_layout_config(&config)
        });

        if let Some(layout_config) = layout_config {
            info!(
                "Applying layout configuration ({} root children)...",
                layout_config.root.children.len()
            );

            self.tokio_runtime.block_on(async {
                let mut layout_manager = self.layout_manager.write().await;
                layout_manager
                    .apply_layout(layout_config)
                    .map_err(|e| anyhow::anyhow!("Failed to apply layout: {}", e))
            })?;

            let component_count = self.tokio_runtime.block_on(async {
                let layout_manager = self.layout_manager.read().await;
                layout_manager.component_count()
            });
            info!("Layout applied with {} components", component_count);
        } else {
            debug!("No layout configuration found, using default view");
        }

        Ok(())
    }
}

/// Gets a nested value from a configuration tree using dot notation.
fn get_nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current)
}

/// Parses layout configuration from a Value.
fn parse_layout_config(config: &Value) -> Option<LayoutConfig> {
    let layout = config.get("layout")?;

    // Get layout type
    let layout_type = layout
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "dock" => LayoutType::Dock,
            "grid" => LayoutType::Grid,
            "tiles" => LayoutType::Tiles,
            _ => LayoutType::Stack,
        })
        .unwrap_or(LayoutType::Stack);

    // Parse root node - the layout block itself acts as a container
    let root = parse_layout_node_as_root(layout, &layout_type)?;

    Some(LayoutConfig::new(layout_type, root))
}

/// Parses the layout block as the root node, extracting components as children.
fn parse_layout_node_as_root(layout: &Value, layout_type: &LayoutType) -> Option<LayoutNode> {
    // The root node type matches the layout type
    let root_type = match layout_type {
        LayoutType::Stack => "stack",
        LayoutType::Dock => "dock",
        LayoutType::Grid => "grid",
        LayoutType::Tiles => "tiles",
    };

    let mut root = LayoutNode::new(root_type).with_id("root");

    // Parse component children from the layout object
    if let Some(layout_obj) = layout.as_object() {
        // In HCL, labeled blocks like `component "header" { ... }` are parsed as:
        // layout.component = { "header": {...}, "content": {...} }
        // So we look for the "component" key which is an object of named components
        if let Some(components) = layout_obj.get("component") {
            if let Some(comp_obj) = components.as_object() {
                // Each key is a component ID, value is the component config
                for (component_id, component_config) in comp_obj {
                    if let Some(child) = parse_component_from_value(component_config, Some(component_id)) {
                        root = root.with_child(child);
                    }
                }
            } else if let Some(comp_arr) = components.as_array() {
                // Array of anonymous components
                for item in comp_arr {
                    if let Some(child) = parse_component_from_value(item, None) {
                        root = root.with_child(child);
                    }
                }
            }
        }
    }

    Some(root)
}

/// Parses a component from a Value.
fn parse_component_from_value(value: &Value, default_id: Option<&str>) -> Option<LayoutNode> {
    let obj = value.as_object()?;

    // Get component type (required)
    let component_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("panel");

    let mut node = LayoutNode::new(component_type);

    // Set ID if provided in the value or use default
    if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
        node = node.with_id(id);
    } else if let Some(id) = default_id {
        node = node.with_id(id);
    }

    // Add all other properties (excluding type, id, and component children)
    for (key, val) in obj {
        match key.as_str() {
            "type" | "id" => continue,
            "component" => {
                // Nested components - HCL labeled blocks are parsed as objects
                // e.g., component "button" { ... } becomes component: { button: {...} }
                if let Some(comp_obj) = val.as_object() {
                    for (child_id, child_config) in comp_obj {
                        if let Some(child) = parse_component_from_value(child_config, Some(child_id)) {
                            node = node.with_child(child);
                        }
                    }
                } else if let Some(arr) = val.as_array() {
                    // Array of anonymous components
                    for item in arr {
                        if let Some(child) = parse_component_from_value(item, None) {
                            node = node.with_child(child);
                        }
                    }
                }
            }
            _ => {
                // Check if this is an event handler (on_* attributes)
                if key.starts_with("on_") {
                    if let Some(handler) = val.as_str() {
                        // Extract event name (e.g., "on_click" -> "click")
                        let event_name = &key[3..];
                        node = node.with_handler(event_name, handler);
                    }
                } else {
                    // Regular property
                    node = node.with_prop(key.clone(), val.clone());
                }
            }
        }
    }

    Some(node)
}
