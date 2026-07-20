//! Nemo runtime - manages all subsystems.

use anyhow::{Context, Result};
use nemo_config::{ConfigurationLoader, SchemaRegistry, Value};
use nemo_data::{DataFlowEngine, DataRepository};
use nemo_events::EventBus;
use nemo_extension::{ExtensionManager, RhaiFeatures};
use nemo_integration::IntegrationGateway;
use nemo_layout::{LayoutConfig, LayoutManager, LayoutNode, LayoutType};
use nemo_plugin_api::{LogLevel, PluginContext, PluginError, PluginValue};
use nemo_registry::{register_all_builtins, ComponentRegistry};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use tokio::runtime::Runtime as TokioRuntime;
use tracing::{debug, info};

/// A pending navigation request.
///
/// Navigation is **deferred**: `navigate()`/`back()`/`forward()` and
/// `<nav-link>` clicks only enqueue a `NavIntent` (and wake the UI poll loop);
/// the intent is applied later by [`NemoRuntime::apply_pending_navigations`],
/// which runs **outside** the `extension_manager` write lock that
/// `call_handler` holds. Applying navigation synchronously from inside a
/// handler would re-acquire that lock on the same thread and deadlock, so the
/// queue mirrors the existing `plugin_dirty_paths` reactivity path.
#[derive(Debug, Clone)]
pub(crate) enum NavIntent {
    /// Navigate a router (primary when `router` is `None`) to `path`.
    Navigate {
        router: Option<String>,
        path: String,
    },
    /// Move a router back one history entry.
    Back { router: Option<String> },
    /// Move a router forward one history entry.
    Forward { router: Option<String> },
}

impl NavIntent {
    /// The explicit router id this intent targets, if any.
    fn router(&self) -> Option<&str> {
        match self {
            NavIntent::Navigate { router, .. }
            | NavIntent::Back { router }
            | NavIntent::Forward { router } => router.as_deref(),
        }
    }
}

/// Host-side authoritative state for one `<router>`, keyed by router id in the
/// router registry. The current path is `history[index]`; `back`/`forward`
/// move `index` within `history`.
#[derive(Debug, Clone, Default)]
struct RouterState {
    /// Visited paths, oldest first.
    history: Vec<String>,
    /// Index of the current path within `history`.
    index: usize,
    /// Params captured from the current path's matching route.
    params: HashMap<String, String>,
    /// Whether the current path+params have been projected into the repository
    /// at least once (so the render pass can project lazily exactly once).
    projected: bool,
}

/// A router's routing table, read from the component tree when applying a
/// navigation.
struct RouterInfo {
    /// The router's `default` path.
    default_path: String,
    /// The `<route>` children in document order.
    routes: Vec<RouteInfo>,
}

/// One `<route>`: its path pattern and optional lifecycle handlers.
struct RouteInfo {
    pattern: String,
    on_enter: Option<String>,
    on_leave: Option<String>,
}

/// A launch-time override of a router's starting path (from `--route`).
#[derive(Debug, Clone)]
struct InitialRoute {
    /// Target router id; `None` applies to the primary router.
    router: Option<String>,
    path: String,
}

/// Sink configuration for outbound data publishing.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SinkConfig {
    /// Sink type (mqtt, redis, nats).
    pub sink_type: String,
    /// Target topic/channel/subject.
    pub target: String,
    /// Connection parameters.
    pub params: HashMap<String, String>,
}

/// The Nemo runtime manages all subsystems.
///
/// # Thread Safety
///
/// `NemoRuntime` itself is **not `Send` or `Sync`** due to two fields:
///
/// - **`extension_manager`** (`Arc<RwLock<ExtensionManager>>`): `ExtensionManager`
///   is `!Send` because it contains a `rhai::Engine` and `rhai::Scope` which are
///   not thread-safe. All access must occur on the main thread via the
///   `RwLock` guard. The `Arc` wrapper exists for shared ownership, not for
///   cross-thread transfer.
///
/// - **`integration`** (`Arc<IntegrationGateway>`): `IntegrationGateway` is
///   `!Send` because `MqttClient` contains a `rumqttc::EventLoop` which is
///   `!Send`. All MQTT operations must be driven from the main thread; other
///   integration clients (HTTP, WebSocket, Redis, NATS) are individually
///   `Send + Sync`.
///
/// The `#[allow(clippy::arc_with_non_send_sync)]` annotations on these fields
/// are intentional: the `Arc` is used for shared ownership within the main
/// thread, not for cross-thread sharing.
///
/// All async I/O (data source polling, HTTP requests, WebSocket streams) is
/// dispatched to the tokio runtime via `tokio_runtime.spawn()`, which
/// operates on `Send + Sync` handles (e.g. `Arc<DataFlowEngine>`). The
/// runtime itself is only accessed from the main/UI thread.
#[allow(dead_code)]
pub struct NemoRuntime {
    /// Main configuration file path.
    config_path: PathBuf,
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
    /// The extension manager (`!Send` — access only from main thread).
    pub extension_manager: Arc<RwLock<ExtensionManager>>,
    /// The integration gateway (`!Send` — access only from main thread).
    pub integration: Arc<IntegrationGateway>,
    /// The tokio runtime for async operations.
    pub tokio_runtime: TokioRuntime,
    /// Flag indicating data has changed and UI needs re-render.
    pub data_dirty: Arc<AtomicBool>,
    /// Notification signal for waking the UI when data changes.
    pub data_notify: Arc<tokio::sync::Notify>,
    /// Cancellation signal for graceful shutdown of background tasks.
    shutdown: Arc<tokio::sync::Notify>,
    /// Sink configurations for outbound data publishing.
    pub sink_configs: Arc<RwLock<HashMap<String, SinkConfig>>>,
    /// Paths written by plugins that need binding propagation.
    plugin_dirty_paths: Arc<RwLock<HashSet<String>>>,
    /// Host-side router state (history + params) keyed by router id.
    router_states: Arc<RwLock<HashMap<String, RouterState>>>,
    /// Queued navigation intents, applied outside the extension lock by
    /// [`Self::apply_pending_navigations`].
    nav_intents: Arc<Mutex<Vec<NavIntent>>>,
    /// Launch-time router starting-path override (from `--route`); consulted
    /// once when a router is first initialized.
    initial_route: Arc<Mutex<Option<InitialRoute>>>,
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
        // ExtensionManager is !Send (contains rhai::Engine). Wrapped in Arc for
        // shared ownership on the main thread, not for cross-thread transfer.
        #[allow(clippy::arc_with_non_send_sync)]
        let extension_manager = Arc::new(RwLock::new(ExtensionManager::new()));
        // IntegrationGateway is !Send (MqttClient contains rumqttc::EventLoop).
        // Same pattern: Arc for shared ownership, main-thread only.
        #[allow(clippy::arc_with_non_send_sync)]
        let integration = Arc::new(IntegrationGateway::new());
        let schema_registry = Arc::new(SchemaRegistry::new());
        let config_loader = ConfigurationLoader::new(Arc::clone(&schema_registry));
        let config = Arc::new(RwLock::new(Value::Null));

        Ok(Self {
            config_path: config_path.to_path_buf(),
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
            data_dirty: Arc::new(AtomicBool::new(false)),
            data_notify: Arc::new(tokio::sync::Notify::new()),
            shutdown: Arc::new(tokio::sync::Notify::new()),
            sink_configs: Arc::new(RwLock::new(HashMap::new())),
            plugin_dirty_paths: Arc::new(RwLock::new(HashSet::new())),
            router_states: Arc::new(RwLock::new(HashMap::new())),
            nav_intents: Arc::new(Mutex::new(Vec::new())),
            initial_route: Arc::new(Mutex::new(None)),
        })
    }

    /// Adds an extension directory.
    pub fn add_extension_dir(&self, dir: &Path) -> Result<()> {
        debug!("Adding extension directory: {:?}", dir);
        let mut ext = self
            .extension_manager
            .write()
            .expect("extension_manager lock poisoned");
        ext.add_script_path(dir.join("scripts"));
        ext.add_plugin_path(dir.join("plugins"));
        ext.add_wasm_path(dir.join("wasm-plugins"));
        Ok(())
    }

    /// Loads configuration from files.
    pub fn load_config(&self) -> Result<()> {
        info!("Loading configuration...");

        if self.config_path.exists() {
            let loaded = self
                .config_loader
                .load(&self.config_path)
                .map_err(|e| anyhow::anyhow!("Failed to load config file: {}", e))?;

            {
                let mut config = self.config.write().expect("config lock poisoned");
                *config = loaded;
            }
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

        // Initialize extensions (sync — no async work needed)
        {
            // Build allowed plugin set from config (<plugins> block).
            // If absent, no plugins are loaded. Plugins with load="false" are skipped.
            let allowed_plugins: HashSet<String> = {
                let config = self.config.read().expect("config lock poisoned");
                config
                    .get("app")
                    .and_then(|app| app.get("plugins"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|plugin| {
                                let obj = plugin.as_object()?;
                                // Skip if load is explicitly false
                                let load =
                                    obj.get("load").and_then(|v| v.as_bool()).unwrap_or(true);
                                if !load {
                                    return None;
                                }
                                obj.get("name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.replace('_', "-"))
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            };

            let ext = self
                .extension_manager
                .read()
                .expect("extension_manager lock poisoned");
            let manifests = ext.discover().unwrap_or_default();
            info!("Discovered {} extensions", manifests.len());
            drop(ext);

            let mut ext = self
                .extension_manager
                .write()
                .expect("extension_manager lock poisoned");
            for manifest in manifests {
                match manifest.extension_type {
                    nemo_extension::ExtensionType::Script => {
                        if let Err(e) = ext.load_script(&manifest.path) {
                            tracing::warn!("Failed to load script {:?}: {}", manifest.path, e);
                        }
                    }
                    nemo_extension::ExtensionType::Plugin
                    | nemo_extension::ExtensionType::WasmPlugin => {
                        let normalized_id = manifest
                            .id
                            .replace('_', "-")
                            .trim_end_matches("-plugin")
                            .to_string();
                        if !allowed_plugins.contains(&normalized_id) {
                            debug!("Skipping plugin {:?} (not in app.plugins)", manifest.id);
                            continue;
                        }
                        let result = match manifest.extension_type {
                            nemo_extension::ExtensionType::Plugin => {
                                ext.load_plugin(&manifest.path)
                            }
                            nemo_extension::ExtensionType::WasmPlugin => {
                                ext.load_wasm_plugin(&manifest.path)
                            }
                            _ => unreachable!(),
                        };
                        if let Err(e) = result {
                            tracing::warn!("Failed to load plugin {:?}: {}", manifest.path, e);
                        }
                    }
                }
            }
        }

        // Set up event subscriptions (async — needs tokio runtime)
        self.tokio_runtime.block_on(async {
            self.setup_event_handlers().await;
        });

        // Load scripts from configuration
        self.load_scripts_from_config()?;

        // Apply layout from configuration
        self.apply_layout_from_config()?;

        // Set up data sources from configuration
        self.setup_data_sources()?;

        // Set up data sinks from configuration
        self.setup_data_sinks()?;

        info!("Runtime initialization complete");
        Ok(())
    }

    /// Returns the configured on-load handler name, if any.
    ///
    /// Set via `<script src="…" on-load="handler_fn" />`. The handler is
    /// invoked once, after the layout is built (see `App::new`), so scripts can
    /// hydrate the UI from persisted state at startup.
    pub fn on_load_handler(&self) -> Option<String> {
        let config = self.config.read().ok()?;
        config
            .get("scripts")
            .and_then(|s| s.get("on_load"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Loads scripts specified in configuration.
    fn load_scripts_from_config(&self) -> Result<()> {
        let scripts_config = {
            let config = self.config.read().expect("config lock poisoned");
            config.get("scripts").cloned()
        };

        if let Some(scripts) = scripts_config {
            // Apply opt-in Rhai features from `<script features="file-io" />`
            // before any scripts are loaded, so the rhai-fs package is
            // registered (and the underlying engine rebuilt) before
            // compilation. Default is off — scripts are sandboxed (no I/O)
            // unless the app explicitly opts in.
            let mut features = RhaiFeatures::default();
            if let Some(arr) = scripts.get("features").and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        match s {
                            "file-io" | "file_io" => features.file_io = true,
                            "network" => features.network = true,
                            "system" => features.system = true,
                            "science" => features.science = true,
                            _ => tracing::warn!("Unknown script feature: {s}"),
                        }
                    }
                }
            }
            if features.file_io || features.network || features.system || features.science {
                info!(
                    "Applying Rhai script features: file_io={}, network={}, system={}, science={}",
                    features.file_io, features.network, features.system, features.science
                );
                let mut ext = self
                    .extension_manager
                    .write()
                    .expect("extension_manager lock poisoned");
                ext.apply_rhai_features(features);
            }

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
                    let mut ext = self
                        .extension_manager
                        .write()
                        .expect("extension_manager lock poisoned");
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
                } else {
                    debug!("Scripts path does not exist: {:?}", scripts_path);
                }
            }

            // Handle individual script files specified in the config
            if let Some(files) = scripts.get("files").and_then(|v| v.as_array()) {
                for file_value in files {
                    if let Some(file_path) = file_value.as_str() {
                        let script_path =
                            if file_path.starts_with("./") || file_path.starts_with("../") {
                                self.config_path
                                    .parent()
                                    .unwrap_or(std::path::Path::new("."))
                                    .join(file_path)
                            } else {
                                std::path::PathBuf::from(file_path)
                            };

                        if script_path.exists() {
                            let mut ext = self
                                .extension_manager
                                .write()
                                .expect("extension_manager lock poisoned");
                            match ext.load_script(&script_path) {
                                Ok(id) => info!("Loaded script: {}", id),
                                Err(e) => {
                                    tracing::warn!("Failed to load script {:?}: {}", script_path, e)
                                }
                            }
                        }
                    }
                }
            }
        }

        // Load single-file component (`.nemo` SFC) `<script>` bodies under
        // `sfc:<tag>` ids. One script serves every instance of the tag; the
        // instance is distinguished by the `component_id` its handlers receive.
        // The `sfc:` prefix keeps a single colon so `call_handler`'s first-`::`
        // split resolves `sfc:<tag>::<fn>` to (script_id=`sfc:<tag>`, fn).
        let sfc_scripts: Vec<(String, String)> = {
            let config = self.config.read().expect("config lock poisoned");
            config
                .get("sfc")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(tag, def)| {
                            def.get("script")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.trim().is_empty())
                                .map(|s| (tag.clone(), s.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        if !sfc_scripts.is_empty() {
            let mut ext = self
                .extension_manager
                .write()
                .expect("extension_manager lock poisoned");
            for (tag, source) in sfc_scripts {
                let id = format!("sfc:{}", tag);
                match ext.load_script_source(&id, &source) {
                    Ok(()) => info!("Loaded SFC script: {}", id),
                    Err(e) => tracing::warn!("Failed to load SFC script {}: {}", id, e),
                }
            }
        }

        // Register the runtime context with the extension manager for API access
        let context: Arc<dyn PluginContext> = Arc::new(RuntimeContext::new(
            Arc::clone(&self.config),
            Arc::clone(&self.layout_manager),
            Arc::clone(&self.event_bus),
            Arc::clone(&self.data_engine.repository),
            Arc::clone(&self.data_dirty),
            Arc::clone(&self.data_notify),
            Arc::clone(&self.plugin_dirty_paths),
            Arc::clone(&self.nav_intents),
        ));

        {
            let mut ext = self
                .extension_manager
                .write()
                .expect("extension_manager lock poisoned");
            ext.register_context(Arc::clone(&context));

            // Register HTTP request functions so RHAI scripts can make
            // GET/POST/PUT/DELETE calls from event handlers.
            ext.register_http_functions(self.tokio_runtime.handle().clone());

            // Initialize native plugins now that the context is available.
            // This must happen before apply_layout_from_config() so that
            // plugin-registered templates are available for layout expansion.
            ext.init_plugins(context);
        }

        // Note: WASM plugin ticking is driven by `tick_wasm_plugins()` which is
        // called from `apply_pending_data_updates()` on the main/UI thread.
        // ExtensionManager is not Send/Sync (due to RhaiEngine), so we cannot
        // spawn a background thread for this.

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
    #[allow(dead_code)]
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// Returns the component registry.
    #[allow(dead_code)]
    pub fn registry(&self) -> &Arc<ComponentRegistry> {
        &self.registry
    }

    /// Returns the path to the loaded configuration file (the project's `app.xml`).
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Gets a configuration value by path.
    pub fn get_config(&self, path: &str) -> Option<Value> {
        let config = self.config.read().expect("config lock poisoned");
        get_nested_value(&config, path).cloned()
    }

    /// Sets a configuration value (not implemented - config is read-only).
    #[allow(dead_code)]
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

        let mut ext = self
            .extension_manager
            .write()
            .expect("extension_manager lock poisoned");
        match ext.call_script::<()>(
            script_id,
            function_name,
            (component_id.to_string(), event_data.to_string()),
        ) {
            Ok(_) => debug!(
                "Handler {}::{} executed successfully",
                script_id, function_name
            ),
            Err(e) => tracing::warn!("Handler {}::{} failed: {}", script_id, function_name, e),
        }
    }

    /// Parses and applies the layout configuration.
    pub fn apply_layout_from_config(&self) -> Result<()> {
        // Collect plugin-registered templates and convert PluginValue → nemo_config::Value
        let extra_templates: TemplateMap = {
            let ext = self
                .extension_manager
                .read()
                .expect("extension_manager lock poisoned");
            ext.plugin_templates()
                .iter()
                .map(|(name, pv)| (name.clone(), plugin_value_to_config_value(pv.clone())))
                .collect()
        };

        let layout_config = {
            let config = self.config.read().expect("config lock poisoned");
            parse_layout_config(&config, &extra_templates)
        };

        if let Some(layout_config) = layout_config {
            info!(
                "Applying layout configuration ({} root children)...",
                layout_config.root.children.len()
            );

            self.layout_manager
                .write()
                .expect("layout_manager lock poisoned")
                .apply_layout(layout_config)
                .map_err(|e| anyhow::anyhow!("Failed to apply layout: {}", e))?;

            let component_count = self
                .layout_manager
                .read()
                .expect("layout_manager lock poisoned")
                .component_count();
            info!("Layout applied with {} components", component_count);
        } else {
            debug!("No layout configuration found, using default view");
        }

        Ok(())
    }

    /// Parses data source configuration and registers sources with the DataFlowEngine.
    fn setup_data_sources(&self) -> Result<()> {
        let data_config = {
            let config = self.config.read().expect("config lock poisoned");
            config.get("data").cloned()
        };

        let data_config = match data_config {
            Some(dc) => dc,
            None => {
                debug!("No data configuration found");
                return Ok(());
            }
        };

        // Parse source blocks: data { source "name" { type = "..." ... } }
        let sources = match data_config.get("source") {
            Some(s) => s.clone(),
            None => {
                debug!("No data sources configured");
                return Ok(());
            }
        };

        let source_obj = match sources.as_object() {
            Some(obj) => obj.clone(),
            None => return Ok(()),
        };

        self.tokio_runtime.block_on(async {
            for (source_name, source_config) in &source_obj {
                let source_type = source_config
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                info!(
                    "Configuring data source '{}' (type: {})",
                    source_name, source_type
                );

                match nemo_data::create_source(source_name, source_type, source_config) {
                    Some(source) => {
                        self.data_engine.register_source(source).await;
                        info!("Registered data source '{}'", source_name);
                    }
                    None => {
                        tracing::warn!(
                            "Unknown data source type '{}' for source '{}'",
                            source_type,
                            source_name
                        );
                    }
                }
            }

            // Start all registered sources
            let results = self.data_engine.start_all().await;
            for (id, result) in &results {
                match result {
                    Ok(()) => info!("Started data source '{}'", id),
                    Err(e) => tracing::warn!("Failed to start data source '{}': {}", id, e),
                }
            }

            // Start the data update loop for each source
            self.start_data_update_loop().await;

            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    /// Starts background tasks that consume data source updates and push them into the repository.
    async fn start_data_update_loop(&self) {
        let source_ids = self.data_engine.source_ids().await;

        for source_id in source_ids {
            if let Some(mut rx) = self.data_engine.subscribe_source(&source_id).await {
                let data_engine = Arc::clone(&self.data_engine);
                let data_dirty = Arc::clone(&self.data_dirty);
                let data_notify = Arc::clone(&self.data_notify);
                let shutdown = Arc::clone(&self.shutdown);

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = shutdown.notified() => {
                                debug!("Data update loop for '{}' shutting down", source_id);
                                break;
                            }
                            result = rx.recv() => {
                                match result {
                                    Ok(update) => {
                                        if let Err(e) = data_engine.process_update(update).await {
                                            tracing::warn!(
                                                "Failed to process data update for '{}': {}",
                                                source_id,
                                                e
                                            );
                                        } else {
                                            data_dirty.store(true, Ordering::Release);
                                            data_notify.notify_one();
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!(
                                            "Data update receiver for '{}' lagged by {} messages",
                                            source_id,
                                            n
                                        );
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        debug!("Data source '{}' channel closed", source_id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    /// Gracefully shuts down background tasks and stops data sources.
    pub fn shutdown(&self) {
        info!("Shutting down Nemo runtime...");

        // Signal all background tasks to stop
        self.shutdown.notify_waiters();

        // Stop all data sources
        self.tokio_runtime.block_on(async {
            let results = self.data_engine.stop_all().await;
            for (id, result) in &results {
                match result {
                    Ok(()) => debug!("Stopped data source '{}'", id),
                    Err(e) => tracing::warn!("Failed to stop data source '{}': {}", id, e),
                }
            }
        });

        info!("Runtime shutdown complete");
    }

    /// Checks for pending data updates and propagates them through bindings.
    /// Returns true if any updates were applied (indicating the UI needs re-render).
    pub fn apply_pending_data_updates(&self) -> bool {
        // Tick WASM plugins (driven from the UI thread since ExtensionManager is !Send).
        // Each plugin tracks its own interval internally so calling this frequently is fine.
        if let Ok(mut ext) = self.extension_manager.try_write() {
            ext.tick_wasm_plugins();
        }

        // Check and clear the dirty flag
        if !self.data_dirty.swap(false, Ordering::AcqRel) {
            return false;
        }

        let mut any_updates = false;

        // Get source IDs and read their data from the repository
        let source_ids = self
            .tokio_runtime
            .block_on(async { self.data_engine.source_ids().await });

        for source_id in &source_ids {
            let data_path = nemo_data::DataPath::from_source(source_id);
            if let Some(value) = self.data_engine.repository.get(&data_path) {
                let source_path = format!("data.{}", source_id);

                if let Ok(mut layout_manager) = self.layout_manager.try_write() {
                    let updates = layout_manager.on_data_changed(&source_path, &value);
                    if !updates.is_empty() {
                        layout_manager.apply_updates(updates);
                        any_updates = true;
                    }
                }
            }
        }

        // Propagate plugin-written data paths through bindings
        let dirty_paths: Vec<String> = {
            if let Ok(mut paths) = self.plugin_dirty_paths.try_write() {
                paths.drain().collect()
            } else {
                vec![]
            }
        };

        for path in &dirty_paths {
            if let Ok(data_path) = nemo_data::DataPath::parse(path) {
                if let Some(value) = self.data_engine.repository.get(&data_path) {
                    if let Ok(mut layout_manager) = self.layout_manager.try_write() {
                        let updates = layout_manager.on_data_changed(path, &value);
                        if !updates.is_empty() {
                            layout_manager.apply_updates(updates);
                            any_updates = true;
                        }
                    }
                }
            }
        }

        any_updates
    }

    // ── Router / navigation ────────────────────────────────────────────────

    /// Enqueues a navigation to `path` on `router` (the primary router when
    /// `None`) and wakes the UI poll loop. Never mutates router state or fires
    /// hooks directly — those happen in [`Self::apply_pending_navigations`].
    pub fn enqueue_navigation(&self, router: Option<String>, path: String) {
        self.push_nav_intent(NavIntent::Navigate { router, path });
    }

    fn push_nav_intent(&self, intent: NavIntent) {
        if let Ok(mut q) = self.nav_intents.lock() {
            q.push(intent);
        }
        self.data_dirty.store(true, Ordering::Release);
        self.data_notify.notify_one();
    }

    /// Records a launch-time starting-path override for a router (from
    /// `--route`). `spec` is `<path>` (primary router) or `<router-id>=<path>`.
    /// Consulted once, when the target router is first initialized, so it must
    /// be set before the first render.
    pub fn set_initial_route(&self, spec: &str) {
        let (router, path) = match spec.split_once('=') {
            Some((r, p)) => (Some(r.trim().to_string()), p.trim().to_string()),
            None => (None, spec.trim().to_string()),
        };
        if path.is_empty() {
            return;
        }
        if let Ok(mut ir) = self.initial_route.lock() {
            *ir = Some(InitialRoute { router, path });
        }
    }

    /// The launch-time starting path for `router_id`, if a `--route` override
    /// targets it (explicitly by id, or the primary router when unscoped).
    fn initial_path_for(&self, router_id: &str) -> Option<String> {
        let ir = self.initial_route.lock().ok().and_then(|g| g.clone())?;
        let applies = match &ir.router {
            Some(rid) => rid == router_id,
            None => self.primary_router_id().as_deref() == Some(router_id),
        };
        applies.then_some(ir.path)
    }

    /// Returns the current path for `router_id`, lazily initializing the router
    /// on first access — to a `--route` override if one targets it, else
    /// `default_path`. Called from the render pass.
    pub fn router_current_path(&self, router_id: &str, default_path: &str) -> String {
        {
            let states = self.router_states.read().expect("router_states poisoned");
            if let Some(st) = states.get(router_id) {
                if let Some(path) = st.history.get(st.index) {
                    return path.clone();
                }
            }
        }
        let init_path = self
            .initial_path_for(router_id)
            .unwrap_or_else(|| default_path.to_string());
        let mut states = self.router_states.write().expect("router_states poisoned");
        let st = states
            .entry(router_id.to_string())
            .or_insert_with(|| RouterState {
                history: vec![init_path.clone()],
                index: 0,
                params: HashMap::new(),
                projected: false,
            });
        st.history.get(st.index).cloned().unwrap_or(init_path)
    }

    /// Returns the current path for `router_id` without initializing it. Used
    /// for `<nav-link>` active-state comparison, which must not create routers.
    pub fn router_current_path_peek(&self, router_id: &str) -> Option<String> {
        let states = self.router_states.read().ok()?;
        let st = states.get(router_id)?;
        st.history.get(st.index).cloned()
    }

    /// Resolves the id of the primary router: the one flagged `primary="true"`,
    /// else the first `<router>` found in the component tree.
    pub fn primary_router_id(&self) -> Option<String> {
        let lm = self.layout_manager.read().ok()?;
        let mut first = None;
        for id in lm.component_ids() {
            if let Some(c) = lm.get_component(&id) {
                if c.component_type == "router" {
                    let is_primary = c
                        .properties
                        .get("primary")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_primary {
                        return Some(id);
                    }
                    if first.is_none() {
                        first = Some(id);
                    }
                }
            }
        }
        first
    }

    /// Gathers a router's `default` path and its `<route>` children (pattern +
    /// lifecycle handlers) in document order, by reading the component tree.
    fn router_info(&self, router_id: &str) -> Option<RouterInfo> {
        let lm = self.layout_manager.read().ok()?;
        let router = lm.get_component(router_id)?;
        if router.component_type != "router" {
            return None;
        }
        let default_path = router
            .properties
            .get("default")
            .and_then(|v| v.as_str())
            .unwrap_or("/")
            .to_string();
        let routes = router
            .children
            .iter()
            .filter_map(|cid| lm.get_component(cid))
            .filter(|c| c.component_type == "route")
            .map(|c| RouteInfo {
                pattern: c
                    .properties
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                on_enter: c.handlers.get("enter").cloned(),
                on_leave: c.handlers.get("leave").cloned(),
            })
            .collect();
        Some(RouterInfo {
            default_path,
            routes,
        })
    }

    /// Projects a router's current path + params into the `DataRepository` at
    /// `data.route.<id>.path` and `data.route.<id>.params.*`, and records them
    /// in the router state. Reused by both the render pass (via
    /// [`Self::sync_route_projection`]) and applied navigations.
    fn write_route_to_repo(&self, router_id: &str, path: &str, params: &HashMap<String, String>) {
        let repo = &self.data_engine.repository;
        if let Ok(dp) = nemo_data::DataPath::parse(&format!("data.route.{}.path", router_id)) {
            let _ = repo.set(&dp, Value::String(path.to_string()));
        }
        // Replace the whole params object in one set so stale keys from the
        // previous route are cleared (deleting would leave a Null tombstone
        // that breaks nested sets).
        let mut params_obj = Value::Object(Default::default());
        if let Value::Object(obj) = &mut params_obj {
            for (k, v) in params {
                obj.insert(k.clone(), Value::String(v.clone()));
            }
        }
        if let Ok(dp) = nemo_data::DataPath::parse(&format!("data.route.{}.params", router_id)) {
            let _ = repo.set(&dp, params_obj);
        }
        if let Ok(mut states) = self.router_states.write() {
            if let Some(st) = states.get_mut(router_id) {
                st.params = params.clone();
                st.projected = true;
            }
        }
    }

    /// Marks the projected route paths dirty so `apply_pending_data_updates`
    /// propagates them through bindings (mirrors the `set_data` path).
    fn mark_route_dirty(&self, router_id: &str, params: &HashMap<String, String>) {
        if let Ok(mut paths) = self.plugin_dirty_paths.write() {
            paths.insert(format!("data.route.{}.path", router_id));
            for k in params.keys() {
                paths.insert(format!("data.route.{}.params.{}", router_id, k));
            }
        }
        self.data_dirty.store(true, Ordering::Release);
    }

    /// Projects a router's current path + params from the render pass, but only
    /// when they have not yet been projected or the params changed — so it is a
    /// cheap no-op on steady-state re-renders and never loops.
    pub fn sync_route_projection(
        &self,
        router_id: &str,
        path: &str,
        params: &HashMap<String, String>,
    ) {
        let needs = {
            let states = self.router_states.read().expect("router_states poisoned");
            match states.get(router_id) {
                Some(st) => !st.projected || &st.params != params,
                None => true,
            }
        };
        if needs {
            self.write_route_to_repo(router_id, path, params);
        }
    }

    /// Applies all queued navigation intents. Runs on the UI thread from the
    /// poll loop, **outside** the extension lock: it updates router history,
    /// projects path+params into the repository, and fires `on-leave`/`on-enter`
    /// lifecycle hooks. Returns `true` if any navigation was applied (so the
    /// caller re-renders).
    pub fn apply_pending_navigations(&self) -> bool {
        let intents: Vec<NavIntent> = {
            let mut q = self.nav_intents.lock().expect("nav_intents poisoned");
            if q.is_empty() {
                return false;
            }
            std::mem::take(&mut *q)
        };

        let mut any = false;
        for intent in intents {
            if self.apply_one_navigation(intent) {
                any = true;
            }
        }
        any
    }

    /// Applies a single navigation intent. Returns `true` if the current path
    /// actually changed.
    fn apply_one_navigation(&self, intent: NavIntent) -> bool {
        let router_id = match intent
            .router()
            .map(String::from)
            .or_else(|| self.primary_router_id())
        {
            Some(id) => id,
            None => {
                tracing::warn!("navigate: no router found");
                return false;
            }
        };
        let info = match self.router_info(&router_id) {
            Some(info) => info,
            None => {
                tracing::warn!("navigate: unknown router '{}'", router_id);
                return false;
            }
        };

        // Update history/index under the write lock and capture old + new path.
        let (old_path, new_path) = {
            let mut states = self.router_states.write().expect("router_states poisoned");
            let st = states
                .entry(router_id.clone())
                .or_insert_with(|| RouterState {
                    history: vec![info.default_path.clone()],
                    index: 0,
                    params: HashMap::new(),
                    projected: false,
                });
            let old_path = st.history.get(st.index).cloned();
            let new_path = match &intent {
                NavIntent::Navigate { path, .. } => {
                    // Drop any forward history, then push unless it's a no-op.
                    st.history.truncate(st.index + 1);
                    if st.history.get(st.index) != Some(path) {
                        st.history.push(path.clone());
                        st.index = st.history.len() - 1;
                    }
                    path.clone()
                }
                NavIntent::Back { .. } => {
                    if st.index == 0 {
                        return false;
                    }
                    st.index -= 1;
                    st.history[st.index].clone()
                }
                NavIntent::Forward { .. } => {
                    if st.index + 1 >= st.history.len() {
                        return false;
                    }
                    st.index += 1;
                    st.history[st.index].clone()
                }
            };
            (old_path, new_path)
        };

        let changed = old_path.as_deref() != Some(new_path.as_str());

        // Match the new path to a route for its params + on-enter handler.
        let patterns: Vec<String> = info.routes.iter().map(|r| r.pattern.clone()).collect();
        let (new_idx, new_params) = crate::containers::router::resolve_route(&patterns, &new_path)
            .unwrap_or((usize::MAX, HashMap::new()));

        // Project path + params into the repository and flag for binding
        // propagation.
        self.write_route_to_repo(&router_id, &new_path, &new_params);
        self.mark_route_dirty(&router_id, &new_params);

        // Fire lifecycle hooks only on an actual path change, outside all locks.
        if changed {
            if let Some(old) = &old_path {
                let old_idx = crate::containers::router::resolve_route(&patterns, old)
                    .map(|(i, _)| i)
                    .unwrap_or(usize::MAX);
                if let Some(handler) = info.routes.get(old_idx).and_then(|r| r.on_leave.clone()) {
                    self.call_handler(&handler, &router_id, "leave");
                }
            }
            if let Some(handler) = info.routes.get(new_idx).and_then(|r| r.on_enter.clone()) {
                self.call_handler(&handler, &router_id, "enter");
            }
        }

        changed
    }

    /// Parses sink configuration and stores sink configs.
    fn setup_data_sinks(&self) -> Result<()> {
        let data_config = {
            let config = self.config.read().expect("config lock poisoned");
            config.get("data").cloned()
        };

        let data_config = match data_config {
            Some(dc) => dc,
            None => return Ok(()),
        };

        let sinks = match data_config.get("sink") {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let sink_obj = match sinks.as_object() {
            Some(obj) => obj.clone(),
            None => return Ok(()),
        };

        let mut configs = self
            .sink_configs
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to lock sink configs: {}", e))?;

        for (sink_name, sink_config) in &sink_obj {
            let sink_type = sink_config
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let target = sink_config
                .get("topic")
                .or_else(|| sink_config.get("channel"))
                .or_else(|| sink_config.get("subject"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let mut params = HashMap::new();
            if let Some(obj) = sink_config.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        params.insert(k.clone(), s.to_string());
                    } else if let Some(i) = v.as_i64() {
                        params.insert(k.clone(), i.to_string());
                    }
                }
            }

            info!(
                "Configured data sink '{}' (type: {}, target: {})",
                sink_name, sink_type, target
            );
            configs.insert(
                sink_name.clone(),
                SinkConfig {
                    sink_type,
                    target,
                    params,
                },
            );
        }

        Ok(())
    }

    /// Publishes data to a configured sink.
    #[allow(dead_code)]
    pub fn publish_to_sink(&self, sink_id: &str, payload: &str) -> Result<()> {
        let sink_config = {
            let configs = self
                .sink_configs
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to lock sink configs: {}", e))?;
            configs.get(sink_id).cloned()
        };

        let sink_config =
            sink_config.ok_or_else(|| anyhow::anyhow!("Sink '{}' not found", sink_id))?;

        let sink_name = sink_config
            .params
            .get("name")
            .cloned()
            .unwrap_or_else(|| sink_id.to_string());

        self.tokio_runtime.block_on(async {
            match sink_config.sink_type.as_str() {
                "mqtt" => {
                    if let Some(client_lock) = self.integration.mqtt(&sink_name).await {
                        let client = client_lock.read().await;
                        client
                            .publish(
                                &sink_config.target,
                                payload.as_bytes().to_vec(),
                                nemo_integration::QoS::AtLeastOnce,
                                false,
                            )
                            .await
                            .map_err(|e| anyhow::anyhow!("MQTT publish failed: {}", e))?;
                    } else {
                        tracing::warn!("No MQTT client registered for sink '{}'", sink_id);
                    }
                }
                "redis" => {
                    if let Some(client_lock) = self.integration.redis(&sink_name).await {
                        let client = client_lock.read().await;
                        client
                            .publish(&sink_config.target, payload)
                            .await
                            .map_err(|e| anyhow::anyhow!("Redis publish failed: {}", e))?;
                    } else {
                        tracing::warn!("No Redis client registered for sink '{}'", sink_id);
                    }
                }
                "nats" => {
                    if let Some(client_lock) = self.integration.nats(&sink_name).await {
                        let client = client_lock.read().await;
                        client
                            .publish(&sink_config.target, payload.as_bytes())
                            .await
                            .map_err(|e| anyhow::anyhow!("NATS publish failed: {}", e))?;
                    } else {
                        tracing::warn!("No NATS client registered for sink '{}'", sink_id);
                    }
                }
                other => {
                    tracing::warn!("Unknown sink type '{}' for sink '{}'", other, sink_id);
                }
            }
            Ok(())
        })
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

// ── Template expansion ────────────────────────────────────────────────────
//
// ## Overview
//
// Templates allow reusable component definitions in XML. A template is
// defined once, then instantiated by components that set `template = "name"`.
//
// ## Expansion Pipeline
//
// 1. `extract_templates()` — Collects all `templates { template "name" { .. } }`
//    blocks from the parsed config into a `HashMap<String, Value>`.
//
// 2. Plugin templates — Templates registered by native plugins via
//    `PluginRegistrar::register_template()` are merged in. XML-defined
//    templates override plugin templates on name collision.
//
// 3. `expand_children()` — Recursively walks all `component` children in the
//    layout tree. For each child that has a `template = "name"` key, calls
//    `expand_template()`.
//
// 4. `expand_template()` — The core expansion function:
//    a. Resolves the template name, detecting circular references via a stack.
//    b. Recursively expands template-of-template chains (a template can itself
//       reference another template).
//    c. Extracts and validates `vars` from the instance via `extract_vars()`.
//    d. Interpolates `${var_name}` placeholders in the template body via
//       `interpolate_variables()`.
//    e. Deep-merges instance properties onto the expanded template via
//       `deep_merge_values()`.
//    f. Handles slot injection — if the template contains a child with
//       `slot = true`, the instance's own children are injected there via
//       `find_and_inject_slot()`.
//    g. Scopes template-originated child IDs by prefixing with the instance ID
//       (e.g. template child `"inner"` becomes `"page_a_inner"`) to prevent
//       collisions when the same template is instantiated multiple times.
//    h. Strips consumed keys (`template`, `slot`, `vars`) from the output.
//
// ## Deep Merge Semantics (`deep_merge_values`)
//
// - Scalar keys: overlay (instance) wins over base (template).
// - `component` children: merged via `merge_component_children()` — base
//   children are preserved, overlay children with the same ID replace them,
//   new IDs are appended.
// - `binding` blocks: merged by `target` property — overlay bindings with
//   the same target replace base bindings, others are appended.
// - `template` and `vars` keys in the overlay are skipped (consumed).
//
// ## Slot Injection (`find_and_inject_slot`)
//
// A template can designate one child as a "slot" by adding `slot = true`.
// When the template is instantiated, the instance's own component children
// are injected into the slot child's `component` map. This allows templates
// to define a wrapper structure while letting each instance provide unique
// content.
//
// ## ID Scoping (`scope_template_children`)
//
// To prevent ID collisions, template-originated child IDs are prefixed with
// the parent instance ID: `"child"` → `"instance_child"`. Only IDs that
// originated from the template are scoped; instance-injected children (e.g.
// via slots) keep their original IDs. The scoping is recursive through
// template-owned subtrees.

type TemplateMap = HashMap<String, Value>;

/// Extracts template definitions from the parsed config.
///
/// XML `<templates><template name="name">...</template></templates>` parses as:
/// `config["templates"]["template"]["name"] = { ... }`
fn extract_templates(config: &Value) -> TemplateMap {
    let mut map = TemplateMap::new();
    if let Some(templates_block) = config.get("templates") {
        if let Some(template_entries) = templates_block.get("template") {
            if let Some(obj) = template_entries.as_object() {
                for (name, value) in obj {
                    map.insert(name.clone(), value.clone());
                }
            }
        }
    }
    map
}

/// Deep-merges two `Value::Object`s. Overlay wins for scalars.
/// Special handling for `component` children and `binding` blocks.
/// The `template` key from the overlay is skipped (consumed during expansion).
fn deep_merge_values(base: &Value, overlay: &Value) -> Value {
    let base_obj = match base.as_object() {
        Some(o) => o,
        None => return overlay.clone(),
    };
    let overlay_obj = match overlay.as_object() {
        Some(o) => o,
        None => return overlay.clone(),
    };

    let mut result = base_obj.clone();

    for (key, overlay_val) in overlay_obj {
        if key == "template" || key == "vars" {
            continue; // consumed during expansion
        }
        match key.as_str() {
            "component" => {
                let base_children = result.get("component").cloned().unwrap_or(Value::Null);
                let merged = merge_component_children(&base_children, overlay_val);
                result.insert(key.clone(), merged);
            }
            "binding" => {
                let base_bindings = result.get("binding").cloned().unwrap_or(Value::Null);
                let merged = merge_bindings(&base_bindings, overlay_val);
                result.insert(key.clone(), merged);
            }
            _ => {
                // Scalar / any other key: overlay wins
                result.insert(key.clone(), overlay_val.clone());
            }
        }
    }

    Value::Object(result)
}

/// Merges component children. For Object children (labeled blocks): base keys
/// first, overlay keys appended. Same-ID overlay children replace base children.
fn merge_component_children(base: &Value, overlay: &Value) -> Value {
    match (base.as_object(), overlay.as_object()) {
        (Some(base_obj), Some(overlay_obj)) => {
            let mut result = base_obj.clone();
            for (id, child) in overlay_obj {
                // Same-ID replaces, new IDs are appended
                result.insert(id.clone(), child.clone());
            }
            Value::Object(result)
        }
        (Some(_), None) if overlay.is_null() => base.clone(),
        (None, Some(_)) | (None, None) => overlay.clone(),
        _ => overlay.clone(),
    }
}

/// Merges binding blocks by `target` property. Normalizes to arrays,
/// instance wins for same target.
fn merge_bindings(base: &Value, overlay: &Value) -> Value {
    let base_arr = match base {
        Value::Array(arr) => arr.clone(),
        Value::Object(_) => vec![base.clone()],
        _ => Vec::new(),
    };
    let overlay_arr = match overlay {
        Value::Array(arr) => arr.clone(),
        Value::Object(_) => vec![overlay.clone()],
        _ => Vec::new(),
    };

    // Index overlay bindings by target
    let mut overlay_targets: HashMap<String, Value> = HashMap::new();
    for b in &overlay_arr {
        if let Some(target) = b.get("target").and_then(|v| v.as_str()) {
            overlay_targets.insert(target.to_string(), b.clone());
        }
    }

    let mut result: Vec<Value> = Vec::new();
    // Keep base bindings, replacing those with matching overlay targets
    for b in &base_arr {
        if let Some(target) = b.get("target").and_then(|v| v.as_str()) {
            if let Some(replacement) = overlay_targets.remove(target) {
                result.push(replacement);
            } else {
                result.push(b.clone());
            }
        } else {
            result.push(b.clone());
        }
    }
    // Append remaining overlay bindings (new targets)
    for (_, b) in overlay_targets {
        result.push(b);
    }

    if result.len() == 1 {
        result.into_iter().next().unwrap()
    } else {
        Value::Array(result)
    }
}

/// Removes specified keys from a `Value::Object`.
fn strip_keys(value: &Value, keys: &[&str]) -> Value {
    match value.as_object() {
        Some(obj) => {
            let mut result = obj.clone();
            for key in keys {
                result.shift_remove(*key);
            }
            Value::Object(result)
        }
        None => value.clone(),
    }
}

/// Extracts the `vars` block from a component instance as a `HashMap<String, String>`.
/// Returns an empty map if no `vars` key is present. Errors if vars contains non-string values.
fn extract_vars(instance: &Value) -> Result<HashMap<String, String>, String> {
    let obj = match instance.as_object() {
        Some(o) => o,
        None => return Ok(HashMap::new()),
    };

    let vars_val = match obj.get("vars") {
        Some(v) => v,
        None => return Ok(HashMap::new()),
    };

    let vars_obj = vars_val
        .as_object()
        .ok_or("'vars' block must be an object")?;

    let mut vars = HashMap::new();
    for (key, val) in vars_obj {
        match val.as_str() {
            Some(s) => {
                vars.insert(key.clone(), s.to_string());
            }
            None => {
                return Err(format!(
                    "Variable '{}' must be a string, got: {:?}",
                    key, val
                ));
            }
        }
    }

    Ok(vars)
}

/// Recursively walks a `Value` tree and replaces `${var_name}` patterns in strings
/// with values from the vars map. Errors on undefined variables.
fn interpolate_variables(
    value: &Value,
    vars: &HashMap<String, String>,
    template_name: &str,
) -> Result<Value, String> {
    match value {
        Value::String(s) => {
            let mut result = s.clone();
            // Find all ${...} patterns
            let mut start = 0;
            while let Some(begin) = result[start..].find("${") {
                let begin = start + begin;
                let after_open = begin + 2;
                match result[after_open..].find('}') {
                    Some(end) => {
                        let var_name = &result[after_open..after_open + end];
                        match vars.get(var_name) {
                            Some(replacement) => {
                                let pattern = format!("${{{}}}", var_name);
                                result = result.replacen(&pattern, replacement, 1);
                                // Continue from after the replacement
                                start = begin + replacement.len();
                            }
                            None => {
                                let available: Vec<&str> =
                                    vars.keys().map(|k| k.as_str()).collect();
                                return Err(format!(
                                    "Undefined variable '{}' in template '{}'. Available vars: {:?}",
                                    var_name, template_name, available
                                ));
                            }
                        }
                    }
                    None => break, // Unclosed ${, leave as-is
                }
            }
            Ok(Value::String(result))
        }
        Value::Object(obj) => {
            let mut result = indexmap::IndexMap::new();
            for (key, val) in obj {
                result.insert(
                    key.clone(),
                    interpolate_variables(val, vars, template_name)?,
                );
            }
            Ok(Value::Object(result))
        }
        Value::Array(arr) => {
            let mut result = Vec::new();
            for val in arr {
                result.push(interpolate_variables(val, vars, template_name)?);
            }
            Ok(Value::Array(result))
        }
        _ => Ok(value.clone()),
    }
}

/// Wraps a value in an object with a single "component" key.
fn obj_with_component(children: &Value) -> Value {
    let mut map = indexmap::IndexMap::new();
    map.insert("component".to_string(), children.clone());
    Value::Object(map)
}

/// Walks the template's `component` children looking for one with `slot = true`.
/// If found, appends `instance_children` into that child's own `component`
/// children and strips the `slot` key. If no slot found, returns None.
fn find_and_inject_slot(template_value: &Value, instance_children: &Value) -> Option<Value> {
    let obj = template_value.as_object()?;
    let components = obj.get("component")?.as_object()?;

    for (child_id, child_val) in components {
        if let Some(true) = child_val.get("slot").and_then(|v| v.as_bool()) {
            // Found the slot child — inject instance children into it
            let mut new_components = components.clone();
            let mut slot_child = child_val.as_object().cloned().unwrap_or_default();

            // Merge instance children into the slot child's component children
            let existing = slot_child.get("component").cloned().unwrap_or(Value::Null);
            let merged = if existing.is_null() {
                instance_children.clone()
            } else {
                merge_component_children(&existing, instance_children)
            };
            slot_child.insert("component".to_string(), merged);
            slot_child.shift_remove("slot"); // strip slot key

            new_components.insert(child_id.clone(), Value::Object(slot_child));

            let mut result = obj.clone();
            result.insert("component".to_string(), Value::Object(new_components));
            return Some(Value::Object(result));
        }

        // Recurse into this child to find a nested slot
        if child_val.get("component").is_some() {
            if let Some(injected_child) = find_and_inject_slot(child_val, instance_children) {
                let mut new_components = components.clone();
                new_components.insert(child_id.clone(), injected_child);
                let mut result = obj.clone();
                result.insert("component".to_string(), Value::Object(new_components));
                return Some(Value::Object(result));
            }
        }
    }

    None
}

/// Expands a single component instance that may reference a template.
/// `instance_id` is the labeled block name (e.g., "page_button") used to
/// prefix template-originated child IDs for uniqueness.
fn expand_template(
    instance: &Value,
    templates: &TemplateMap,
    expansion_stack: &mut Vec<String>,
    instance_id: Option<&str>,
) -> Result<Value, String> {
    let obj = match instance.as_object() {
        Some(o) => o,
        None => return Ok(instance.clone()),
    };

    // Check for template = "name"
    let template_name = match obj.get("template").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => {
            // No template reference — just recurse into children
            return expand_children(instance, templates, expansion_stack);
        }
    };

    // Circular reference check
    if expansion_stack.contains(&template_name) {
        return Err(format!(
            "Circular template reference detected: {} -> {}",
            expansion_stack.join(" -> "),
            template_name
        ));
    }

    // Look up the template
    let template_def = templates
        .get(&template_name)
        .ok_or_else(|| format!("Unknown template: '{}'", template_name))?
        .clone();

    // Collect all template-originated component IDs (including nested descendants)
    // so we can prefix them for uniqueness without touching instance-injected IDs.
    let mut template_owned_ids = std::collections::HashSet::new();
    collect_all_component_ids(&template_def, &mut template_owned_ids);
    let template_child_ids: Vec<String> = template_def
        .get("component")
        .and_then(|c| c.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Recursively expand the template itself (template-of-template)
    expansion_stack.push(template_name.clone());
    let expanded_template = expand_template(&template_def, templates, expansion_stack, None)?;
    expansion_stack.pop();

    // Interpolate template variables from instance vars block
    let vars = extract_vars(instance)?;
    let interpolated = if vars.is_empty() {
        expanded_template
    } else {
        interpolate_variables(&expanded_template, &vars, &template_name)?
    };

    // Extract instance children before merging
    let instance_children = obj.get("component").cloned();

    // Merge instance properties (without children) onto the template first
    let instance_without_children = strip_keys(instance, &["component"]);
    let merged = deep_merge_values(&interpolated, &instance_without_children);

    // Handle children: if template has a slot, inject instance children there.
    // Otherwise, merge children as siblings via deep_merge.
    let with_slots = match &instance_children {
        Some(children) if !children.is_null() => {
            match find_and_inject_slot(&merged, children) {
                Some(injected) => injected,
                None => {
                    // No slot found — merge children as siblings
                    deep_merge_values(&merged, &obj_with_component(children))
                }
            }
        }
        _ => merged,
    };

    // Strip consumed keys
    let stripped = strip_keys(&with_slots, &["template", "slot", "vars"]);

    // Prefix template-originated child IDs with the instance ID for uniqueness.
    // This prevents ID collisions when the same template is used by multiple
    // instances (e.g., all content pages having a child named "inner").
    let scoped = if let Some(parent_id) = instance_id {
        scope_template_children(
            &stripped,
            parent_id,
            &template_child_ids,
            &template_owned_ids,
        )
    } else {
        stripped
    };

    // Recurse into merged children
    expand_children(&scoped, templates, expansion_stack)
}

/// Renames template-originated child IDs by prefixing them with the parent
/// instance ID. Only children whose original ID is in `template_child_ids`
/// are renamed at the top level; instance children keep their original IDs.
/// Within template-originated subtrees, only IDs in `template_owned_ids` are
/// recursively scoped, so instance-injected children (e.g. via slots) are untouched.
fn scope_template_children(
    value: &Value,
    parent_id: &str,
    template_child_ids: &[String],
    template_owned_ids: &std::collections::HashSet<String>,
) -> Value {
    if template_child_ids.is_empty() {
        return value.clone();
    }

    let obj = match value.as_object() {
        Some(o) => o,
        None => return value.clone(),
    };

    let components = match obj.get("component").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return value.clone(),
    };

    let mut new_components = indexmap::IndexMap::new();
    for (id, child) in components {
        if template_child_ids.contains(id) {
            let new_id = format!("{}_{}", parent_id, id);
            let scoped_child = scope_owned_descendants(child, parent_id, template_owned_ids);
            new_components.insert(new_id, scoped_child);
        } else {
            new_components.insert(id.clone(), child.clone());
        }
    }

    let mut result = obj.clone();
    result.insert("component".to_string(), Value::Object(new_components));
    Value::Object(result)
}

/// Recursively collects all component IDs from a value tree.
fn collect_all_component_ids(value: &Value, ids: &mut std::collections::HashSet<String>) {
    if let Some(obj) = value.as_object() {
        if let Some(components) = obj.get("component").and_then(|c| c.as_object()) {
            for (id, child) in components {
                ids.insert(id.clone());
                collect_all_component_ids(child, ids);
            }
        }
    }
}

/// Recursively prefixes nested component IDs with `parent_id`, but only
/// those IDs that are in `owned_ids` (template-originated). Instance-injected
/// children (e.g. via slot) are left unchanged.
fn scope_owned_descendants(
    value: &Value,
    parent_id: &str,
    owned_ids: &std::collections::HashSet<String>,
) -> Value {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return value.clone(),
    };

    let components = match obj.get("component").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return value.clone(),
    };

    let mut new_components = indexmap::IndexMap::new();
    for (id, child) in components {
        if owned_ids.contains(id) {
            let new_id = format!("{}_{}", parent_id, id);
            let scoped_child = scope_owned_descendants(child, parent_id, owned_ids);
            new_components.insert(new_id, scoped_child);
        } else {
            new_components.insert(id.clone(), child.clone());
        }
    }

    let mut result = obj.clone();
    result.insert("component".to_string(), Value::Object(new_components));
    Value::Object(result)
}

/// Iterates over all `component` children and expands templates in each.
fn expand_children(
    value: &Value,
    templates: &TemplateMap,
    expansion_stack: &mut Vec<String>,
) -> Result<Value, String> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Ok(value.clone()),
    };

    let components = match obj.get("component") {
        Some(c) => c,
        None => return Ok(value.clone()),
    };

    let expanded_components = if let Some(comp_obj) = components.as_object() {
        let mut result = indexmap::IndexMap::new();
        for (id, child) in comp_obj {
            let expanded = expand_template(child, templates, expansion_stack, Some(id.as_str()))?;
            result.insert(id.clone(), expanded);
        }
        Value::Object(result)
    } else if let Some(comp_arr) = components.as_array() {
        let mut result = Vec::new();
        for child in comp_arr {
            let expanded = expand_template(child, templates, expansion_stack, None)?;
            result.push(expanded);
        }
        Value::Array(result)
    } else {
        components.clone()
    };

    let mut result = obj.clone();
    result.insert("component".to_string(), expanded_components);
    Ok(Value::Object(result))
}

// ── Single-file component (SFC) compilation ───────────────────────────────
//
// SFCs are a namespaced, file-scoped superset of `<templates>`. Imported
// `.nemo` files are parsed into `config["sfc"][tag] = { template, style?,
// script?, source_path }` by nemo-config. Here we (1) collect the registered
// tags, (2) merge each SFC template into the `TemplateMap`, and (3) rewrite any
// `<tag>` usage — in the layout and inside other SFC templates — into a
// `template = "tag"` instance so the existing expansion pipeline (deep-merge,
// slot injection, id-scoping) handles the rest with no downstream changes.

/// Collects the set of registered SFC tag names from `config["sfc"]`.
fn collect_sfc_tags(config: &Value) -> HashSet<String> {
    config
        .get("sfc")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Recursively rewrites SFC tag usages into template instances. Children are
/// rewritten first (bottom-up) so a node that is itself an SFC tag keeps its
/// already-rewritten children.
fn rewrite_sfc_tags(value: &Value, sfc_tags: &HashSet<String>) -> Value {
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            // Arrays of anonymous components can appear under `component`.
            if let Value::Array(arr) = value {
                return Value::Array(arr.iter().map(|v| rewrite_sfc_tags(v, sfc_tags)).collect());
            }
            return value.clone();
        }
    };

    let mut result = obj.clone();

    // Recurse into component children (object of id→node or array of nodes).
    if let Some(children) = obj.get("component") {
        result.insert(
            "component".to_string(),
            rewrite_sfc_tags(children, sfc_tags),
        );
    }
    // Object maps that are NOT the `component` key still need recursion when
    // their values look like component nodes (e.g. the `component` object's own
    // id→node entries reached via the array/object branch above). Handle the
    // id→node map case: values keyed by id under `component` are objects.
    if obj.get("component").is_none() && obj.get("type").is_none() {
        // A bare id→node map (as `component`'s object value) — rewrite each entry.
        let mut rewritten = indexmap::IndexMap::new();
        let mut changed = false;
        for (k, v) in obj {
            if v.as_object().and_then(|o| o.get("type")).is_some() {
                rewritten.insert(k.clone(), rewrite_sfc_tags(v, sfc_tags));
                changed = true;
            } else {
                rewritten.insert(k.clone(), v.clone());
            }
        }
        if changed {
            return Value::Object(rewritten);
        }
    }

    // If this node itself is an SFC tag, transform it into a template instance.
    if let Some(t) = result.get("type").and_then(|v| v.as_str()) {
        if sfc_tags.contains(t) {
            let tag = t.to_string();
            return sfc_node_to_instance(&Value::Object(result), &tag);
        }
    }

    Value::Object(result)
}

/// Converts an SFC tag node (`{ type: "tag", <attrs>, component: {…} }`) into a
/// template instance (`{ template: "tag", <attrs>, vars: {…}, component: {…} }`).
///
/// Scalar attributes are kept at the top level (so `deep_merge_values` overlays
/// them onto the template body) *and* folded into a `vars` map (so `${attr}`
/// interpolation works). The `type` key is dropped — the template body supplies
/// the real component type.
fn sfc_node_to_instance(node: &Value, tag: &str) -> Value {
    let obj = match node.as_object() {
        Some(o) => o,
        None => return node.clone(),
    };

    let mut inst = indexmap::IndexMap::new();
    inst.insert("template".to_string(), Value::String(tag.to_string()));

    let mut vars = indexmap::IndexMap::new();
    for (key, val) in obj {
        match key.as_str() {
            "type" => continue, // template body supplies the real type
            "component" => {
                inst.insert("component".to_string(), val.clone());
            }
            _ => {
                inst.insert(key.clone(), val.clone());
                if let Some(s) = scalar_to_string(val) {
                    vars.insert(key.clone(), Value::String(s));
                }
            }
        }
    }
    if !vars.is_empty() {
        inst.insert("vars".to_string(), Value::Object(vars));
    }

    Value::Object(inst)
}

/// Renders a scalar `Value` as the string used for `${}` interpolation. Returns
/// `None` for objects/arrays/null, which are not interpolable.
fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Rewrites bare `on_*` handler references (no `::`) in an SFC template body to
/// `sfc:<tag>::<fn>`, so template-authored handlers route to the SFC's own
/// script. Already-qualified refs (containing `::`) and instance-supplied
/// handlers (overlaid later via deep-merge) are left untouched.
fn rewrite_sfc_handlers(value: &Value, tag: &str) -> Value {
    match value {
        Value::Object(obj) => {
            let mut result = indexmap::IndexMap::new();
            for (key, val) in obj {
                let new_val = if key.starts_with("on_") {
                    match val.as_str() {
                        Some(f) if !f.contains("::") && !f.trim().is_empty() => {
                            Value::String(format!("sfc:{}::{}", tag, f))
                        }
                        _ => rewrite_sfc_handlers(val, tag),
                    }
                } else {
                    rewrite_sfc_handlers(val, tag)
                };
                result.insert(key.clone(), new_val);
            }
            Value::Object(result)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| rewrite_sfc_handlers(v, tag)).collect())
        }
        _ => value.clone(),
    }
}

// ── Layout parsing ───────────────────────────────────────────────────────

/// Parses layout configuration from a Value.
///
/// `extra_templates` are templates registered by native plugins, merged
/// with any templates defined in the XML config. Plugin templates are
/// added first so XML-defined templates can override them.
fn parse_layout_config(config: &Value, extra_templates: &TemplateMap) -> Option<LayoutConfig> {
    let layout = config.get("layout")?;
    let mut templates = extract_templates(config);

    // Merge plugin-registered templates (XML templates take precedence)
    for (name, value) in extra_templates {
        templates
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }

    // Compile single-file components (`.nemo` SFCs): each imported SFC becomes a
    // `TemplateMap` entry keyed by its tag, with template-authored `on_*`
    // handlers rewritten to `sfc:<tag>::<fn>` and any nested SFC tags rewritten
    // for composition. XML-defined `<templates>` take precedence on name clash.
    let sfc_tags = collect_sfc_tags(config);
    if !sfc_tags.is_empty() {
        if let Some(sfc_map) = config.get("sfc").and_then(|v| v.as_object()) {
            for (tag, def) in sfc_map {
                if let Some(body) = def.get("template") {
                    let body = rewrite_sfc_tags(body, &sfc_tags);
                    let body = rewrite_sfc_handlers(&body, tag);
                    templates.entry(tag.clone()).or_insert(body);
                }
            }
        }
    }

    // Rewrite SFC tag usages in the layout into template instances before
    // expansion, so the existing expand/slot/scope pipeline handles them.
    let layout_owned;
    let layout: &Value = if sfc_tags.is_empty() {
        layout
    } else {
        layout_owned = rewrite_sfc_tags(layout, &sfc_tags);
        &layout_owned
    };

    let expanded_layout = if templates.is_empty() {
        layout.clone()
    } else {
        let mut stack = Vec::new();
        expand_children(layout, &templates, &mut stack).unwrap_or_else(|e| {
            tracing::error!("Template expansion failed: {}", e);
            layout.clone()
        })
    };

    // Get layout type
    let layout_type = expanded_layout
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
    let root = parse_layout_node_as_root(&expanded_layout, &layout_type)?;

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

    let mut root = LayoutNode::new(root_type).with_id("__layout_root__");

    // Parse component children from the layout object
    if let Some(layout_obj) = layout.as_object() {
        // Components are parsed as:
        // layout.component = { "header": {...}, "content": {...} }
        // So we look for the "component" key which is an object of named components
        if let Some(components) = layout_obj.get("component") {
            if let Some(comp_obj) = components.as_object() {
                // Each key is a component ID, value is the component config
                for (component_id, component_config) in comp_obj {
                    if let Some(child) =
                        parse_component_from_value(component_config, Some(component_id))
                    {
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
    let component_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("panel");

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
                // Nested components - parsed as objects
                // e.g., component: { button: {...} }
                if let Some(comp_obj) = val.as_object() {
                    for (child_id, child_config) in comp_obj {
                        if let Some(child) =
                            parse_component_from_value(child_config, Some(child_id))
                        {
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
                if let Some(event_name) = key.strip_prefix("on_") {
                    if let Some(handler) = val.as_str() {
                        // Extract event name (e.g., "on_click" -> "click")
                        node = node.with_handler(event_name, handler);
                    }
                } else if let Some(target_prop) = key.strip_prefix("bind_") {
                    // Data binding: bind_text = "data.sensors.payload.temperature"
                    if let Some(source_path) = val.as_str() {
                        node.config
                            .bindings
                            .push(nemo_layout::BindingSpec::one_way(source_path, target_prop));
                    }
                } else if key == "binding" {
                    // Explicit binding block(s)
                    let binding_values = if let Some(arr) = val.as_array() {
                        arr.clone()
                    } else {
                        vec![val.clone()]
                    };
                    for binding_val in &binding_values {
                        if let Some(binding_obj) = binding_val.as_object() {
                            let source = binding_obj
                                .get("source")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let target = binding_obj
                                .get("target")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let mode = binding_obj
                                .get("mode")
                                .and_then(|v| v.as_str())
                                .unwrap_or("one_way");
                            let transform = binding_obj
                                .get("transform")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let mut spec = match mode {
                                "two_way" => nemo_layout::BindingSpec::two_way(&source, &target),
                                _ => nemo_layout::BindingSpec::one_way(&source, &target),
                            };
                            if let Some(t) = transform {
                                spec = spec.with_transform(t);
                            }
                            node.config.bindings.push(spec);
                        }
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

/// Runtime context providing API access to scripts and plugins.
pub struct RuntimeContext {
    config: Arc<RwLock<Value>>,
    layout_manager: Arc<RwLock<LayoutManager>>,
    event_bus: Arc<EventBus>,
    data_repository: Arc<DataRepository>,
    data_dirty: Arc<AtomicBool>,
    data_notify: Arc<tokio::sync::Notify>,
    plugin_dirty_paths: Arc<RwLock<HashSet<String>>>,
    nav_intents: Arc<Mutex<Vec<NavIntent>>>,
}

impl RuntimeContext {
    /// Creates a new runtime context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Arc<RwLock<Value>>,
        layout_manager: Arc<RwLock<LayoutManager>>,
        event_bus: Arc<EventBus>,
        data_repository: Arc<DataRepository>,
        data_dirty: Arc<AtomicBool>,
        data_notify: Arc<tokio::sync::Notify>,
        plugin_dirty_paths: Arc<RwLock<HashSet<String>>>,
        nav_intents: Arc<Mutex<Vec<NavIntent>>>,
    ) -> Self {
        Self {
            config,
            layout_manager,
            event_bus,
            data_repository,
            data_dirty,
            data_notify,
            plugin_dirty_paths,
            nav_intents,
        }
    }

    /// Enqueues a navigation intent and wakes the UI poll loop. Shared by the
    /// `navigate`/`back`/`forward` trait methods below.
    fn enqueue(&self, intent: NavIntent) {
        if let Ok(mut q) = self.nav_intents.lock() {
            q.push(intent);
        }
        self.data_dirty.store(true, Ordering::Release);
        self.data_notify.notify_one();
    }
}

impl PluginContext for RuntimeContext {
    fn get_data(&self, path: &str) -> Option<PluginValue> {
        // Read from the DataRepository under "data.<path>"
        let data_path = nemo_data::DataPath::parse(&format!("data.{}", path)).ok()?;
        self.data_repository
            .get(&data_path)
            .map(|v| value_to_plugin_value(&v))
    }

    fn set_data(&self, path: &str, value: PluginValue) -> Result<(), PluginError> {
        let full_path = format!("data.{}", path);
        let data_path = nemo_data::DataPath::parse(&full_path)
            .map_err(|e| PluginError::InvalidConfig(e.to_string()))?;
        let config_value = plugin_value_to_config_value(value);
        self.data_repository
            .set(&data_path, config_value)
            .map_err(|e| PluginError::InvalidConfig(e.to_string()))?;
        // Record this path so apply_pending_data_updates propagates it through bindings.
        if let Ok(mut paths) = self.plugin_dirty_paths.write() {
            paths.insert(full_path);
        }
        self.data_dirty.store(true, Ordering::Release);
        self.data_notify.notify_one();
        Ok(())
    }

    fn emit_event(&self, event_type: &str, payload: PluginValue) {
        let json_value = plugin_value_to_json(payload);
        self.event_bus.emit_simple(event_type, json_value);
    }

    fn get_config(&self, path: &str) -> Option<PluginValue> {
        if let Ok(config) = self.config.try_read() {
            get_nested_value(&config, path).map(value_to_plugin_value)
        } else {
            None
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => tracing::debug!(target: "plugin", "{}", message),
            LogLevel::Info => tracing::info!(target: "plugin", "{}", message),
            LogLevel::Warn => tracing::warn!(target: "plugin", "{}", message),
            LogLevel::Error => tracing::error!(target: "plugin", "{}", message),
        }
    }

    fn get_component_property(&self, component_id: &str, property: &str) -> Option<PluginValue> {
        if let Ok(layout_manager) = self.layout_manager.try_read() {
            layout_manager
                .get_component(component_id)
                .and_then(|component| component.properties.get(property))
                .map(value_to_plugin_value)
        } else {
            None
        }
    }

    fn set_component_property(
        &self,
        component_id: &str,
        property: &str,
        value: PluginValue,
    ) -> Result<(), PluginError> {
        if let Ok(mut layout_manager) = self.layout_manager.try_write() {
            let config_value = plugin_value_to_config_value(value);
            layout_manager
                .set_property(component_id, property, config_value)
                .map_err(|e| PluginError::ComponentFailed(e.to_string()))
        } else {
            Err(PluginError::ComponentFailed(
                "Layout manager is locked".to_string(),
            ))
        }
    }

    fn navigate(&self, router: Option<&str>, path: &str) -> Result<(), PluginError> {
        self.enqueue(NavIntent::Navigate {
            router: router.map(String::from),
            path: path.to_string(),
        });
        Ok(())
    }

    fn back(&self, router: Option<&str>) -> Result<(), PluginError> {
        self.enqueue(NavIntent::Back {
            router: router.map(String::from),
        });
        Ok(())
    }

    fn forward(&self, router: Option<&str>) -> Result<(), PluginError> {
        self.enqueue(NavIntent::Forward {
            router: router.map(String::from),
        });
        Ok(())
    }
}

/// Converts a nemo_config::Value to a PluginValue.
fn value_to_plugin_value(value: &Value) -> PluginValue {
    match value {
        Value::Null => PluginValue::Null,
        Value::Bool(b) => PluginValue::Bool(*b),
        Value::Integer(i) => PluginValue::Integer(*i),
        Value::Float(f) => PluginValue::Float(*f),
        Value::String(s) => PluginValue::String(s.clone()),
        Value::Array(arr) => PluginValue::Array(arr.iter().map(value_to_plugin_value).collect()),
        Value::Object(obj) => PluginValue::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), value_to_plugin_value(v)))
                .collect(),
        ),
    }
}

/// Converts a PluginValue to a nemo_config::Value.
fn plugin_value_to_config_value(value: PluginValue) -> Value {
    match value {
        PluginValue::Null => Value::Null,
        PluginValue::Bool(b) => Value::Bool(b),
        PluginValue::Integer(i) => Value::Integer(i),
        PluginValue::Float(f) => Value::Float(f),
        PluginValue::String(s) => Value::String(s),
        PluginValue::Array(arr) => {
            Value::Array(arr.into_iter().map(plugin_value_to_config_value).collect())
        }
        PluginValue::Object(obj) => {
            let map: indexmap::IndexMap<String, Value> = obj
                .into_iter()
                .map(|(k, v)| (k, plugin_value_to_config_value(v)))
                .collect();
            Value::Object(map)
        }
    }
}

/// Converts a PluginValue to a serde_json::Value for events.
fn plugin_value_to_json(value: PluginValue) -> serde_json::Value {
    match value {
        PluginValue::Null => serde_json::Value::Null,
        PluginValue::Bool(b) => serde_json::Value::Bool(b),
        PluginValue::Integer(i) => serde_json::json!(i),
        PluginValue::Float(f) => serde_json::json!(f),
        PluginValue::String(s) => serde_json::Value::String(s),
        PluginValue::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(plugin_value_to_json).collect())
        }
        PluginValue::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .into_iter()
                .map(|(k, v)| (k, plugin_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

#[cfg(test)]
mod test_helpers {
    use indexmap::IndexMap;
    use nemo_config::Value;

    /// Helper to build a `Value::Object` from key-value pairs.
    pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        Value::Object(map)
    }

    /// Shorthand for `Value::String`.
    pub fn s(val: &str) -> Value {
        Value::String(val.to_string())
    }
}

#[cfg(test)]
mod sfc_tests {
    use super::test_helpers::{obj, s};
    use super::*;

    #[test]
    fn test_rewrite_sfc_tag_to_instance() {
        let mut tags = HashSet::new();
        tags.insert("labeled-button".to_string());

        let node = obj(vec![("type", s("labeled-button")), ("label", s("Save"))]);
        let rewritten = rewrite_sfc_tags(&node, &tags);

        // Becomes a template instance; the `type` is dropped (template supplies it).
        assert_eq!(rewritten.get("template"), Some(&s("labeled-button")));
        assert_eq!(rewritten.get("type"), None);
        // Scalar attrs are kept for deep-merge overlay …
        assert_eq!(rewritten.get("label"), Some(&s("Save")));
        // … and folded into `vars` for `${label}` interpolation.
        assert_eq!(
            rewritten.get("vars").and_then(|v| v.get("label")),
            Some(&s("Save"))
        );
    }

    #[test]
    fn test_rewrite_sfc_handlers_prefixes_bare_refs() {
        let body = obj(vec![
            ("type", s("button")),
            ("on_click", s("handleClick")),
            ("on_hover", s("other::qualified")),
        ]);
        let rewritten = rewrite_sfc_handlers(&body, "labeled-button");
        // Bare handler routes to the SFC's own script …
        assert_eq!(
            rewritten.get("on_click"),
            Some(&s("sfc:labeled-button::handleClick"))
        );
        // … already-qualified refs are left untouched.
        assert_eq!(rewritten.get("on_hover"), Some(&s("other::qualified")));
    }

    /// A minimal `card` SFC (panel wrapping a slotted stack) plus an interpolating
    /// `labeled-button` SFC, exercised through the full `parse_layout_config`
    /// pipeline: tag rewrite → template merge → expand → slot inject → id scope.
    fn sfc_config() -> Value {
        let card_template = obj(vec![
            ("type", s("panel")),
            (
                "component",
                obj(vec![(
                    "inner",
                    obj(vec![
                        ("type", s("stack")),
                        ("direction", s("vertical")),
                        ("slot", Value::Bool(true)),
                    ]),
                )]),
            ),
        ]);
        let button_template = obj(vec![
            ("type", s("button")),
            ("label", s("${label}")),
            ("on_click", s("handleClick")),
        ]);

        obj(vec![
            (
                "sfc",
                obj(vec![
                    ("card", obj(vec![("template", card_template)])),
                    ("labeled-button", obj(vec![("template", button_template)])),
                ]),
            ),
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![
                            (
                                "__anon_1",
                                obj(vec![
                                    ("type", s("card")),
                                    (
                                        "component",
                                        obj(vec![(
                                            "lbl1",
                                            obj(vec![("type", s("label")), ("text", s("A"))]),
                                        )]),
                                    ),
                                ]),
                            ),
                            (
                                "__anon_2",
                                obj(vec![
                                    ("type", s("card")),
                                    (
                                        "component",
                                        obj(vec![(
                                            "lbl2",
                                            obj(vec![("type", s("label")), ("text", s("B"))]),
                                        )]),
                                    ),
                                ]),
                            ),
                            (
                                "b1",
                                obj(vec![("type", s("labeled-button")), ("label", s("Save"))]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ])
    }

    #[test]
    fn test_sfc_multi_instance_id_scoping_and_slots() {
        let config = sfc_config();
        let layout = parse_layout_config(&config, &TemplateMap::new()).expect("layout");
        let root = layout.root;

        // Two cards + one button.
        assert_eq!(root.children.len(), 3);

        let cards: Vec<&LayoutNode> = root
            .children
            .iter()
            .filter(|c| c.component_type == "panel")
            .collect();
        assert_eq!(cards.len(), 2, "both cards expanded to panels");

        // The slotted `inner` stack id is scoped per instance — no collision.
        let inner_ids: Vec<String> = cards
            .iter()
            .filter_map(|card| card.children.first())
            .filter_map(|inner| inner.id.clone())
            .collect();
        assert!(inner_ids.contains(&"__anon_1_inner".to_string()));
        assert!(inner_ids.contains(&"__anon_2_inner".to_string()));
        assert_ne!(inner_ids[0], inner_ids[1]);

        // Slot content (the instance's label) landed inside the inner stack.
        let texts: Vec<String> = cards
            .iter()
            .filter_map(|card| card.children.first())
            .flat_map(|inner| inner.children.iter())
            .filter_map(|lbl| lbl.config.properties.get("text").cloned())
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert!(texts.contains(&"A".to_string()));
        assert!(texts.contains(&"B".to_string()));
    }

    #[test]
    fn test_sfc_interpolation_and_scoped_handler() {
        let config = sfc_config();
        let layout = parse_layout_config(&config, &TemplateMap::new()).expect("layout");

        let button = layout
            .root
            .children
            .iter()
            .find(|c| c.component_type == "button")
            .expect("button expanded");

        // `${label}` interpolated from the instance attr.
        assert_eq!(
            button
                .config
                .properties
                .get("label")
                .and_then(|v| v.as_str()),
            Some("Save")
        );
        // Template-authored bare handler routed to the SFC's own script id.
        assert_eq!(
            button.handlers.get("click").map(|s| s.as_str()),
            Some("sfc:labeled-button::handleClick")
        );
    }
}

#[cfg(test)]
mod template_tests {
    use super::test_helpers::{obj, s};
    use super::*;

    #[test]
    fn test_extract_templates_empty() {
        let config = obj(vec![("layout", obj(vec![("type", s("stack"))]))]);
        let templates = extract_templates(&config);
        assert!(templates.is_empty());
    }

    #[test]
    fn test_extract_templates_basic() {
        let config = obj(vec![(
            "templates",
            obj(vec![(
                "template",
                obj(vec![
                    (
                        "nav_item",
                        obj(vec![("type", s("button")), ("variant", s("ghost"))]),
                    ),
                    ("page", obj(vec![("type", s("panel"))])),
                ]),
            )]),
        )]);

        let templates = extract_templates(&config);
        assert_eq!(templates.len(), 2);
        assert!(templates.contains_key("nav_item"));
        assert!(templates.contains_key("page"));
        assert_eq!(
            templates["nav_item"].get("type").and_then(|v| v.as_str()),
            Some("button")
        );
    }

    #[test]
    fn test_deep_merge_scalar_override() {
        let base = obj(vec![
            ("type", s("button")),
            ("variant", s("ghost")),
            ("size", s("sm")),
        ]);
        let overlay = obj(vec![("variant", s("primary")), ("label", s("Click"))]);
        let merged = deep_merge_values(&base, &overlay);
        assert_eq!(merged.get("type").and_then(|v| v.as_str()), Some("button"));
        assert_eq!(
            merged.get("variant").and_then(|v| v.as_str()),
            Some("primary")
        );
        assert_eq!(merged.get("size").and_then(|v| v.as_str()), Some("sm"));
        assert_eq!(merged.get("label").and_then(|v| v.as_str()), Some("Click"));
    }

    #[test]
    fn test_children_appended_no_slot() {
        let template = obj(vec![
            ("type", s("panel")),
            (
                "component",
                obj(vec![("child_a", obj(vec![("type", s("label"))]))]),
            ),
        ]);
        let instance = obj(vec![
            ("template", s("test")),
            (
                "component",
                obj(vec![("child_b", obj(vec![("type", s("button"))]))]),
            ),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("test".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        let comp = result.get("component").unwrap().as_object().unwrap();
        assert!(comp.contains_key("child_a"));
        assert!(comp.contains_key("child_b"));
    }

    #[test]
    fn test_slot_injection() {
        let template = obj(vec![
            ("type", s("panel")),
            (
                "component",
                obj(vec![(
                    "inner",
                    obj(vec![("type", s("stack")), ("slot", Value::Bool(true))]),
                )]),
            ),
        ]);
        let instance = obj(vec![
            ("template", s("page")),
            (
                "component",
                obj(vec![("my_child", obj(vec![("type", s("label"))]))]),
            ),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("page".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        // Instance children should be inside "inner", not at top level
        let top_comp = result.get("component").unwrap().as_object().unwrap();
        assert!(top_comp.contains_key("inner"));
        assert!(!top_comp.contains_key("my_child"));

        let inner = &top_comp["inner"];
        let inner_comp = inner.get("component").unwrap().as_object().unwrap();
        assert!(inner_comp.contains_key("my_child"));

        // slot key should be stripped
        assert!(inner.get("slot").is_none());
    }

    #[test]
    fn test_same_id_child_override() {
        let base_children = obj(vec![
            ("a", obj(vec![("type", s("label")), ("text", s("old"))])),
            ("b", obj(vec![("type", s("button"))])),
        ]);
        let overlay_children = obj(vec![(
            "a",
            obj(vec![("type", s("label")), ("text", s("new"))]),
        )]);

        let merged = merge_component_children(&base_children, &overlay_children);
        let comp = merged.as_object().unwrap();
        assert_eq!(comp.len(), 2);
        assert_eq!(comp["a"].get("text").and_then(|v| v.as_str()), Some("new"));
    }

    #[test]
    fn test_circular_reference_detected() {
        let template_a = obj(vec![("template", s("b")), ("type", s("panel"))]);
        let template_b = obj(vec![("template", s("a")), ("type", s("panel"))]);

        let mut templates = TemplateMap::new();
        templates.insert("a".to_string(), template_a);
        templates.insert("b".to_string(), template_b);

        let instance = obj(vec![("template", s("a"))]);
        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Circular"), "Error was: {}", err);
    }

    #[test]
    fn test_missing_template_error() {
        let templates = TemplateMap::new();
        let instance = obj(vec![("template", s("nonexistent"))]);
        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown template"));
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::test_helpers::{obj, s};
    use super::*;
    use indexmap::IndexMap;

    // ── get_nested_value ──────────────────────────────────────────────

    #[test]
    fn test_get_nested_value_simple() {
        let config = obj(vec![("app", obj(vec![("title", s("Hello"))]))]);
        assert_eq!(get_nested_value(&config, "app.title"), Some(&s("Hello")));
    }

    #[test]
    fn test_get_nested_value_deep() {
        let config = obj(vec![(
            "a",
            obj(vec![("b", obj(vec![("c", Value::Integer(42))]))]),
        )]);
        assert_eq!(
            get_nested_value(&config, "a.b.c"),
            Some(&Value::Integer(42))
        );
    }

    #[test]
    fn test_get_nested_value_missing() {
        let config = obj(vec![("app", obj(vec![("title", s("Hello"))]))]);
        assert_eq!(get_nested_value(&config, "app.missing"), None);
        assert_eq!(get_nested_value(&config, "nonexistent"), None);
        assert_eq!(get_nested_value(&config, "app.title.deep"), None);
    }

    #[test]
    fn test_get_nested_value_single_key() {
        let config = obj(vec![("key", Value::Bool(true))]);
        assert_eq!(get_nested_value(&config, "key"), Some(&Value::Bool(true)));
    }

    // ── create_data_source ────────────────────────────────────────────

    #[test]
    fn test_create_data_source_timer() {
        let config = obj(vec![
            ("type", s("timer")),
            ("interval", Value::Integer(5)),
            ("immediate", Value::Bool(false)),
        ]);
        let source = nemo_data::create_source("test_timer", "timer", &config);
        assert!(source.is_some());
        assert_eq!(source.unwrap().id(), "test_timer");
    }

    #[test]
    fn test_create_data_source_timer_defaults() {
        // Timer with no interval/immediate should use defaults
        let config = obj(vec![("type", s("timer"))]);
        let source = nemo_data::create_source("t", "timer", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_http() {
        let config = obj(vec![
            ("type", s("http")),
            ("url", s("https://example.com/api")),
            ("interval", Value::Integer(30)),
        ]);
        let source = nemo_data::create_source("api", "http", &config);
        assert!(source.is_some());
        assert_eq!(source.unwrap().id(), "api");
    }

    #[test]
    fn test_create_data_source_http_missing_url() {
        let config = obj(vec![("type", s("http"))]);
        let source = nemo_data::create_source("api", "http", &config);
        assert!(
            source.is_none(),
            "HTTP source without URL should return None"
        );
    }

    #[test]
    fn test_create_data_source_websocket() {
        let config = obj(vec![
            ("type", s("websocket")),
            ("url", s("ws://localhost:8080")),
        ]);
        let source = nemo_data::create_source("ws", "websocket", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_websocket_missing_url() {
        let config = obj(vec![("type", s("websocket"))]);
        assert!(nemo_data::create_source("ws", "websocket", &config).is_none());
    }

    #[test]
    fn test_create_data_source_mqtt() {
        let config = obj(vec![
            ("type", s("mqtt")),
            ("host", s("broker.local")),
            ("port", Value::Integer(1883)),
            ("topics", Value::Array(vec![s("sensor/+")])),
        ]);
        let source = nemo_data::create_source("mqtt", "mqtt", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_mqtt_defaults() {
        let config = obj(vec![("type", s("mqtt"))]);
        let source = nemo_data::create_source("mqtt", "mqtt", &config);
        assert!(source.is_some(), "MQTT should use default host/port");
    }

    #[test]
    fn test_create_data_source_redis() {
        let config = obj(vec![
            ("type", s("redis")),
            ("url", s("redis://127.0.0.1:6379")),
            ("channels", Value::Array(vec![s("events")])),
        ]);
        assert!(nemo_data::create_source("r", "redis", &config).is_some());
    }

    #[test]
    fn test_create_data_source_nats() {
        let config = obj(vec![
            ("type", s("nats")),
            ("url", s("nats://127.0.0.1:4222")),
            ("subjects", Value::Array(vec![s("updates.>")])),
        ]);
        assert!(nemo_data::create_source("n", "nats", &config).is_some());
    }

    #[test]
    fn test_create_data_source_file() {
        let config = obj(vec![
            ("type", s("file")),
            ("path", s("/tmp/data.json")),
            ("format", s("json")),
            ("watch", Value::Bool(true)),
        ]);
        assert!(nemo_data::create_source("f", "file", &config).is_some());
    }

    #[test]
    fn test_create_data_source_file_missing_path() {
        let config = obj(vec![("type", s("file"))]);
        assert!(nemo_data::create_source("f", "file", &config).is_none());
    }

    #[test]
    fn test_create_data_source_unknown_type() {
        let config = obj(vec![("type", s("unknown"))]);
        assert!(nemo_data::create_source("x", "unknown", &config).is_none());
    }

    // ── parse_layout_config ───────────────────────────────────────────

    #[test]
    fn test_parse_layout_config_stack() {
        let config = obj(vec![(
            "layout",
            obj(vec![
                ("type", s("stack")),
                (
                    "component",
                    obj(vec![(
                        "btn",
                        obj(vec![("type", s("button")), ("label", s("OK"))]),
                    )]),
                ),
            ]),
        )]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.children.len(), 1);
        assert_eq!(layout.root.children[0].component_type, "button");
    }

    #[test]
    fn test_parse_layout_config_dock() {
        let config = obj(vec![("layout", obj(vec![("type", s("dock"))]))]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.component_type, "dock");
    }

    #[test]
    fn test_parse_layout_config_missing() {
        let config = obj(vec![("app", obj(vec![]))]);
        assert!(parse_layout_config(&config, &TemplateMap::new()).is_none());
    }

    #[test]
    fn test_parse_layout_config_with_handlers() {
        let config = obj(vec![(
            "layout",
            obj(vec![
                ("type", s("stack")),
                (
                    "component",
                    obj(vec![(
                        "btn",
                        obj(vec![("type", s("button")), ("on_click", s("handle_click"))]),
                    )]),
                ),
            ]),
        )]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        let btn = &layout.root.children[0];
        assert_eq!(
            btn.handlers.get("click").map(|s| s.as_str()),
            Some("handle_click")
        );
    }

    #[test]
    fn test_parse_layout_config_with_bindings() {
        let config = obj(vec![(
            "layout",
            obj(vec![
                ("type", s("stack")),
                (
                    "component",
                    obj(vec![(
                        "lbl",
                        obj(vec![
                            ("type", s("label")),
                            ("bind_text", s("data.sensors.temperature")),
                        ]),
                    )]),
                ),
            ]),
        )]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        let lbl = &layout.root.children[0];
        assert_eq!(lbl.config.bindings.len(), 1);
        assert_eq!(lbl.config.bindings[0].source, "data.sensors.temperature");
        assert_eq!(lbl.config.bindings[0].target, "text");
    }

    // ── NemoRuntime basic construction ────────────────────────────────

    #[test]
    fn test_runtime_new_nonexistent_config() {
        let rt = NemoRuntime::new(Path::new("/nonexistent/config.xml")).unwrap();
        // Should succeed — config file is checked lazily in load_config
        assert!(rt.get_config("anything").is_none());
    }

    #[test]
    fn test_runtime_load_config_missing_file() {
        let rt = NemoRuntime::new(Path::new("/does/not/exist.xml")).unwrap();
        // load_config should succeed gracefully when file doesn't exist
        assert!(rt.load_config().is_ok());
    }

    #[test]
    fn test_runtime_get_config_empty() {
        let rt = NemoRuntime::new(Path::new("/tmp/empty.xml")).unwrap();
        assert!(rt.get_config("app.title").is_none());
    }

    // ── call_handler parsing ──────────────────────────────────────────

    #[test]
    fn test_call_handler_with_script_prefix() {
        // Just verify the parsing logic — handler execution will warn
        // about missing scripts, which is fine for this test
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        rt.load_config().unwrap();
        rt.initialize().unwrap();
        // Should not panic; the handler will log a warning
        rt.call_handler("my_script::on_click", "btn1", "click");
    }

    #[test]
    fn test_call_handler_without_script_prefix() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        rt.load_config().unwrap();
        rt.initialize().unwrap();
        // Should default to "handlers" script
        rt.call_handler("on_click", "btn1", "click");
    }

    // ── apply_pending_data_updates ────────────────────────────────────

    #[test]
    fn test_apply_pending_data_updates_when_clean() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        // data_dirty starts false, should return false
        assert!(!rt.apply_pending_data_updates());
    }

    #[test]
    fn test_apply_pending_data_updates_when_dirty_no_sources() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        rt.data_dirty.store(true, Ordering::Release);
        // Dirty but no sources registered — still returns false (no updates to apply)
        assert!(!rt.apply_pending_data_updates());
        // Dirty flag should be cleared
        assert!(!rt.data_dirty.load(Ordering::Acquire));
    }

    // ── RuntimeContext PluginContext impl ──────────────────────────────

    #[test]
    fn test_runtime_context_set_and_get_data() {
        let config = Arc::new(RwLock::new(Value::Null));
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(registry)));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty.clone(),
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );

        // set_data should store and mark dirty
        ctx.set_data("test.value", PluginValue::Integer(42))
            .unwrap();
        assert!(dirty.load(Ordering::Acquire));

        // get_data should retrieve it
        let val = ctx.get_data("test.value");
        assert_eq!(val, Some(PluginValue::Integer(42)));
    }

    #[test]
    fn test_runtime_context_get_data_missing() {
        let config = Arc::new(RwLock::new(Value::Null));
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(registry)));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty,
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );
        assert_eq!(ctx.get_data("nonexistent"), None);
    }

    #[test]
    fn test_runtime_context_get_config() {
        let mut map = IndexMap::new();
        let mut app_map = IndexMap::new();
        app_map.insert("title".to_string(), s("Test App"));
        map.insert("app".to_string(), Value::Object(app_map));
        let config = Arc::new(RwLock::new(Value::Object(map)));

        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(registry)));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty,
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );

        assert_eq!(
            ctx.get_config("app.title"),
            Some(PluginValue::String("Test App".to_string()))
        );
        assert_eq!(ctx.get_config("app.missing"), None);
    }

    #[test]
    fn test_runtime_context_component_property() {
        let config = Arc::new(RwLock::new(Value::Null));
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(Arc::clone(&registry))));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        // Apply a layout so there's a component to query
        {
            let mut lm = layout_manager.write().unwrap();
            let root = LayoutNode::new("stack").with_id("root").with_child(
                LayoutNode::new("label")
                    .with_id("lbl")
                    .with_prop("text", s("Hello")),
            );
            lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
                .unwrap();
        }

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty,
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );

        assert_eq!(
            ctx.get_component_property("lbl", "text"),
            Some(PluginValue::String("Hello".to_string()))
        );
        assert_eq!(ctx.get_component_property("lbl", "missing"), None);
        assert_eq!(ctx.get_component_property("no_such_id", "text"), None);

        // set_component_property
        ctx.set_component_property("lbl", "text", PluginValue::String("World".to_string()))
            .unwrap();
        assert_eq!(
            ctx.get_component_property("lbl", "text"),
            Some(PluginValue::String("World".to_string()))
        );
    }

    // ── Value conversion roundtrips ───────────────────────────────────

    #[test]
    fn test_value_to_plugin_value_roundtrip() {
        let original = Value::Object({
            let mut m = IndexMap::new();
            m.insert("name".to_string(), s("test"));
            m.insert("count".to_string(), Value::Integer(7));
            m.insert("active".to_string(), Value::Bool(true));
            m.insert("ratio".to_string(), Value::Float(1.23));
            m.insert(
                "items".to_string(),
                Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
            );
            m.insert("empty".to_string(), Value::Null);
            m
        });
        let plugin_val = value_to_plugin_value(&original);
        let back = plugin_value_to_config_value(plugin_val);
        assert_eq!(original, back);
    }

    #[test]
    fn test_plugin_value_to_json() {
        let pv = PluginValue::Object({
            let mut m = indexmap::IndexMap::new();
            m.insert("key".to_string(), PluginValue::String("val".to_string()));
            m.insert("num".to_string(), PluginValue::Integer(99));
            m
        });
        let json = plugin_value_to_json(pv);
        assert_eq!(json["key"], "val");
        assert_eq!(json["num"], 99);
    }
}

#[cfg(test)]
mod template_tests_continued {
    use super::test_helpers::{obj, s};
    use super::*;

    #[test]
    fn test_template_key_stripped() {
        let template = obj(vec![("type", s("button")), ("variant", s("ghost"))]);
        let instance = obj(vec![("template", s("btn")), ("label", s("Click"))]);

        let mut templates = TemplateMap::new();
        templates.insert("btn".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();
        assert!(result.get("template").is_none());
    }

    #[test]
    fn test_slot_key_stripped() {
        let template = obj(vec![
            ("type", s("panel")),
            (
                "component",
                obj(vec![(
                    "inner",
                    obj(vec![("type", s("stack")), ("slot", Value::Bool(true))]),
                )]),
            ),
        ]);

        let instance = obj(vec![
            ("template", s("t")),
            (
                "component",
                obj(vec![("child", obj(vec![("type", s("label"))]))]),
            ),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        let inner = result
            .get("component")
            .and_then(|c| c.get("inner"))
            .unwrap();
        assert!(inner.get("slot").is_none());
    }

    #[test]
    fn test_recursive_template_resolution() {
        // "outer" references "inner", which is a plain template
        let inner_template = obj(vec![("type", s("stack")), ("direction", s("vertical"))]);
        let outer_template = obj(vec![
            ("template", s("inner")),
            ("spacing", Value::Integer(12)),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("inner".to_string(), inner_template);
        templates.insert("outer".to_string(), outer_template);

        let instance = obj(vec![
            ("template", s("outer")),
            ("padding", Value::Integer(8)),
        ]);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();
        assert_eq!(result.get("type").and_then(|v| v.as_str()), Some("stack"));
        assert_eq!(
            result.get("direction").and_then(|v| v.as_str()),
            Some("vertical")
        );
        assert_eq!(result.get("spacing").and_then(|v| v.as_i64()), Some(12));
        assert_eq!(result.get("padding").and_then(|v| v.as_i64()), Some(8));
        assert!(result.get("template").is_none());
    }

    #[test]
    fn test_template_child_ids_scoped() {
        // Two pages using the same template should get unique inner child IDs
        let config = obj(vec![
            (
                "templates",
                obj(vec![(
                    "template",
                    obj(vec![(
                        "page",
                        obj(vec![
                            ("type", s("panel")),
                            ("visible", Value::Bool(false)),
                            (
                                "component",
                                obj(vec![(
                                    "inner",
                                    obj(vec![("type", s("stack")), ("slot", Value::Bool(true))]),
                                )]),
                            ),
                        ]),
                    )]),
                )]),
            ),
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![
                            (
                                "page_a",
                                obj(vec![
                                    ("template", s("page")),
                                    (
                                        "component",
                                        obj(vec![("child_a", obj(vec![("type", s("label"))]))]),
                                    ),
                                ]),
                            ),
                            (
                                "page_b",
                                obj(vec![
                                    ("template", s("page")),
                                    (
                                        "component",
                                        obj(vec![("child_b", obj(vec![("type", s("label"))]))]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        let layout_config =
            parse_layout_config(&config, &TemplateMap::new()).expect("Layout parse failed");
        let root = &layout_config.root;

        // page_a's inner child should be "page_a_inner"
        let page_a = &root.children[0];
        assert_eq!(page_a.children[0].effective_id(), "page_a_inner");

        // page_b's inner child should be "page_b_inner"
        let page_b = &root.children[1];
        assert_eq!(page_b.children[0].effective_id(), "page_b_inner");

        // Both should contain their respective injected children
        assert_eq!(page_a.children[0].children[0].effective_id(), "child_a");
        assert_eq!(page_b.children[0].children[0].effective_id(), "child_b");
    }

    #[test]
    fn test_template_handler_preserved() {
        // on_click from template should survive expansion
        let config = obj(vec![
            (
                "templates",
                obj(vec![(
                    "template",
                    obj(vec![(
                        "nav",
                        obj(vec![("type", s("button")), ("on_click", s("on_nav"))]),
                    )]),
                )]),
            ),
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![(
                            "nav_btn",
                            obj(vec![("template", s("nav")), ("label", s("Test"))]),
                        )]),
                    ),
                ]),
            ),
        ]);
        let layout_config =
            parse_layout_config(&config, &TemplateMap::new()).expect("Layout parse failed");

        let nav = &layout_config.root.children[0];
        assert_eq!(
            nav.handlers.get("click").map(|s| s.as_str()),
            Some("on_nav")
        );
    }

    #[test]
    fn test_template_integration() {
        // Build config Value directly to test template expansion
        let config = obj(vec![
            (
                "templates",
                obj(vec![(
                    "template",
                    obj(vec![
                        (
                            "nav",
                            obj(vec![
                                ("type", s("button")),
                                ("variant", s("ghost")),
                                ("size", s("sm")),
                                ("on_click", s("on_nav")),
                            ]),
                        ),
                        (
                            "page",
                            obj(vec![
                                ("type", s("panel")),
                                ("visible", Value::Bool(false)),
                                (
                                    "component",
                                    obj(vec![(
                                        "inner",
                                        obj(vec![
                                            ("type", s("stack")),
                                            ("direction", s("vertical")),
                                            ("slot", Value::Bool(true)),
                                        ]),
                                    )]),
                                ),
                            ]),
                        ),
                    ]),
                )]),
            ),
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![
                            (
                                "nav_btn",
                                obj(vec![("template", s("nav")), ("label", s("Button"))]),
                            ),
                            (
                                "page_btn",
                                obj(vec![
                                    ("template", s("page")),
                                    ("visible", Value::Bool(true)),
                                    (
                                        "component",
                                        obj(vec![(
                                            "title",
                                            obj(vec![
                                                ("type", s("label")),
                                                ("text", s("Button Page")),
                                            ]),
                                        )]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        let layout_config =
            parse_layout_config(&config, &TemplateMap::new()).expect("Layout parse failed");

        // nav_btn should be a ghost button with label
        let root = &layout_config.root;
        assert!(root.children.len() >= 2);

        let nav = &root.children[0];
        assert_eq!(nav.component_type, "button");
        assert_eq!(
            nav.config
                .properties
                .get("variant")
                .and_then(|v| v.as_str()),
            Some("ghost")
        );
        assert_eq!(
            nav.config.properties.get("label").and_then(|v| v.as_str()),
            Some("Button")
        );
        // template key should not leak through as a property
        assert!(!nav.config.properties.contains_key("template"));

        // page_btn should be a panel with visible=true
        let page = &root.children[1];
        assert_eq!(page.component_type, "panel");
        assert_eq!(
            page.config
                .properties
                .get("visible")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        // The inner stack should contain the title label (slot injection)
        assert!(!page.children.is_empty());
        let inner = &page.children[0];
        assert_eq!(inner.component_type, "stack");
        assert!(!inner.children.is_empty());
        let title = &inner.children[0];
        assert_eq!(title.component_type, "label");
        assert_eq!(
            title.config.properties.get("text").and_then(|v| v.as_str()),
            Some("Button Page")
        );
    }
}

#[cfg(test)]
mod template_vars_tests {
    use super::test_helpers::{obj, s};
    use super::*;

    #[test]
    fn test_basic_interpolation() {
        let template = obj(vec![
            ("type", s("label")),
            ("text", s("Status: ${ns}")),
            ("bind_text", s("data.${ns}.output")),
        ]);
        let instance = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("pid.motor1"))])),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        assert_eq!(
            result.get("text").and_then(|v| v.as_str()),
            Some("Status: pid.motor1")
        );
        assert_eq!(
            result.get("bind_text").and_then(|v| v.as_str()),
            Some("data.pid.motor1.output")
        );
    }

    #[test]
    fn test_multiple_instances_different_vars() {
        let template = obj(vec![
            ("type", s("label")),
            ("bind_text", s("data.${ns}.output")),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let instance1 = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("pid.motor1"))])),
        ]);
        let instance2 = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("pid.motor2"))])),
        ]);

        let mut stack = Vec::new();
        let result1 = expand_template(&instance1, &templates, &mut stack, None).unwrap();
        let result2 = expand_template(&instance2, &templates, &mut stack, None).unwrap();

        assert_eq!(
            result1.get("bind_text").and_then(|v| v.as_str()),
            Some("data.pid.motor1.output")
        );
        assert_eq!(
            result2.get("bind_text").and_then(|v| v.as_str()),
            Some("data.pid.motor2.output")
        );
    }

    #[test]
    fn test_undefined_variable_error() {
        let template = obj(vec![("type", s("label")), ("text", s("${undefined_var}"))]);
        let instance = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("foo"))])),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let err = expand_template(&instance, &templates, &mut stack, None).unwrap_err();
        assert!(err.contains("Undefined variable 'undefined_var'"));
        assert!(err.contains("ns"));
    }

    #[test]
    fn test_no_vars_passthrough() {
        // Without a vars block, ${...} patterns should pass through unchanged
        let template = obj(vec![("type", s("label")), ("text", s("${ns}.output"))]);
        let instance = obj(vec![("template", s("t"))]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        assert_eq!(
            result.get("text").and_then(|v| v.as_str()),
            Some("${ns}.output")
        );
    }

    #[test]
    fn test_instance_override_wins() {
        let template = obj(vec![("type", s("label")), ("text", s("${ns} default"))]);
        let instance = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("pid"))])),
            ("text", s("override")),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        // Instance property should override interpolated template value
        assert_eq!(
            result.get("text").and_then(|v| v.as_str()),
            Some("override")
        );
    }

    #[test]
    fn test_nested_template_own_vars() {
        // Inner template has its own vars; outer template's vars should not leak in
        let inner_template = obj(vec![("type", s("label")), ("text", s("inner: ${x}"))]);
        let outer_template = obj(vec![
            ("type", s("panel")),
            ("title", s("outer: ${y}")),
            (
                "component",
                obj(vec![(
                    "child",
                    obj(vec![
                        ("template", s("inner")),
                        ("vars", obj(vec![("x", s("hello"))])),
                    ]),
                )]),
            ),
        ]);
        let instance = obj(vec![
            ("template", s("outer")),
            ("vars", obj(vec![("y", s("world"))])),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("inner".to_string(), inner_template);
        templates.insert("outer".to_string(), outer_template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();

        assert_eq!(
            result.get("title").and_then(|v| v.as_str()),
            Some("outer: world")
        );

        let child = result
            .get("component")
            .and_then(|c| c.get("child"))
            .unwrap();
        assert_eq!(
            child.get("text").and_then(|v| v.as_str()),
            Some("inner: hello")
        );
    }

    #[test]
    fn test_non_string_var_error() {
        let template = obj(vec![("type", s("label"))]);
        let instance = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("x", Value::Integer(42))])),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let err = expand_template(&instance, &templates, &mut stack, None).unwrap_err();
        assert!(err.contains("must be a string"));
    }

    #[test]
    fn test_vars_key_stripped_from_output() {
        let template = obj(vec![("type", s("label"))]);
        let instance = obj(vec![
            ("template", s("t")),
            ("vars", obj(vec![("ns", s("pid"))])),
        ]);

        let mut templates = TemplateMap::new();
        templates.insert("t".to_string(), template);

        let mut stack = Vec::new();
        let result = expand_template(&instance, &templates, &mut stack, None).unwrap();
        assert!(result.get("vars").is_none());
        assert!(result.get("template").is_none());
    }
}

// ── Error path and edge case tests ───────────────────────────────────────
//
// These tests cover error conditions, malformed inputs, and edge cases
// that are not exercised by the happy-path tests above.

#[cfg(test)]
mod error_path_tests {
    use super::test_helpers::{obj, s};
    use super::*;

    // ── get_nested_value edge cases ──────────────────────────────────

    #[test]
    fn test_get_nested_value_empty_path() {
        let config = obj(vec![("key", s("val"))]);
        // Empty string splits to [""], so it looks for key ""
        assert_eq!(get_nested_value(&config, ""), None);
    }

    #[test]
    fn test_get_nested_value_consecutive_dots() {
        let config = obj(vec![("a", obj(vec![("b", s("val"))]))]);
        // "a..b" splits to ["a", "", "b"] — empty segment fails lookup
        assert_eq!(get_nested_value(&config, "a..b"), None);
    }

    #[test]
    fn test_get_nested_value_traverse_scalar() {
        let config = obj(vec![("a", Value::Integer(42))]);
        // Traversing through a scalar should return None
        assert_eq!(get_nested_value(&config, "a.b"), None);
    }

    #[test]
    fn test_get_nested_value_traverse_array() {
        let config = obj(vec![(
            "a",
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
        )]);
        // Arrays don't support .get(key) — should return None
        assert_eq!(get_nested_value(&config, "a.0"), None);
    }

    #[test]
    fn test_get_nested_value_traverse_null() {
        let config = obj(vec![("a", Value::Null)]);
        assert_eq!(get_nested_value(&config, "a.b"), None);
    }

    #[test]
    fn test_get_nested_value_traverse_bool() {
        let config = obj(vec![("flag", Value::Bool(true))]);
        assert_eq!(get_nested_value(&config, "flag.sub"), None);
    }

    // ── extract_vars error paths ─────────────────────────────────────

    #[test]
    fn test_extract_vars_non_object_vars_block() {
        let instance = obj(vec![("template", s("tmpl")), ("vars", s("not_an_object"))]);
        let result = extract_vars(&instance);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be an object"));
    }

    #[test]
    fn test_extract_vars_array_vars_block() {
        let instance = obj(vec![
            ("template", s("tmpl")),
            ("vars", Value::Array(vec![s("a"), s("b")])),
        ]);
        let result = extract_vars(&instance);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_vars_null_vars_block() {
        let instance = obj(vec![("template", s("tmpl")), ("vars", Value::Null)]);
        let result = extract_vars(&instance);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_vars_non_string_var_value() {
        let instance = obj(vec![
            ("template", s("tmpl")),
            ("vars", obj(vec![("count", Value::Integer(42))])),
        ]);
        let result = extract_vars(&instance);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be a string"));
    }

    #[test]
    fn test_extract_vars_non_object_instance() {
        // When the instance itself is not an object, returns empty map
        let result = extract_vars(&s("not_an_object"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_extract_vars_no_vars_key() {
        let instance = obj(vec![("template", s("tmpl"))]);
        let result = extract_vars(&instance);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ── interpolate_variables error paths ─────────────────────────────

    #[test]
    fn test_interpolate_unclosed_pattern() {
        let vars = HashMap::from([("ns".to_string(), "sensor1".to_string())]);
        let input = s("hello ${unclosed");
        let result = interpolate_variables(&input, &vars, "test").unwrap();
        // Unclosed ${ should be left as-is
        assert_eq!(result, s("hello ${unclosed"));
    }

    #[test]
    fn test_interpolate_undefined_variable() {
        let vars = HashMap::new();
        let input = s("${missing}");
        let result = interpolate_variables(&input, &vars, "my_template");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Undefined variable 'missing'"));
        assert!(err.contains("my_template"));
    }

    #[test]
    fn test_interpolate_partial_second_var_undefined() {
        let vars = HashMap::from([("a".to_string(), "val_a".to_string())]);
        let input = s("${a} and ${b}");
        let result = interpolate_variables(&input, &vars, "tmpl");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Undefined variable 'b'"));
    }

    #[test]
    fn test_interpolate_multiple_vars_all_defined() {
        let vars = HashMap::from([
            ("x".to_string(), "1".to_string()),
            ("y".to_string(), "2".to_string()),
        ]);
        let input = s("coords: ${x},${y}");
        let result = interpolate_variables(&input, &vars, "tmpl").unwrap();
        assert_eq!(result, s("coords: 1,2"));
    }

    #[test]
    fn test_interpolate_in_array() {
        let vars = HashMap::from([("ns".to_string(), "sensor1".to_string())]);
        let input = Value::Array(vec![s("data.${ns}.temp"), s("data.${ns}.humidity")]);
        let result = interpolate_variables(&input, &vars, "tmpl").unwrap();
        let Value::Array(items) = result else {
            panic!("Expected array, got {result:?}");
        };
        assert_eq!(items[0], s("data.sensor1.temp"));
        assert_eq!(items[1], s("data.sensor1.humidity"));
    }

    #[test]
    fn test_interpolate_undefined_in_array() {
        let vars = HashMap::new();
        let input = Value::Array(vec![s("${missing}")]);
        let result = interpolate_variables(&input, &vars, "tmpl");
        assert!(result.is_err());
    }

    #[test]
    fn test_interpolate_non_string_passthrough() {
        let vars = HashMap::new();
        // Non-string values should pass through without error
        assert_eq!(
            interpolate_variables(&Value::Integer(42), &vars, "t").unwrap(),
            Value::Integer(42)
        );
        assert_eq!(
            interpolate_variables(&Value::Bool(true), &vars, "t").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            interpolate_variables(&Value::Null, &vars, "t").unwrap(),
            Value::Null
        );
    }

    // ── expand_template error paths ──────────────────────────────────

    #[test]
    fn test_expand_template_non_object_instance() {
        let templates = TemplateMap::new();
        let mut stack = Vec::new();
        // Non-object instance returns the value unchanged
        let result = expand_template(&s("not_an_object"), &templates, &mut stack, None).unwrap();
        assert_eq!(result, s("not_an_object"));
    }

    #[test]
    fn test_expand_template_null_instance() {
        let templates = TemplateMap::new();
        let mut stack = Vec::new();
        let result = expand_template(&Value::Null, &templates, &mut stack, None).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_expand_template_unknown_template_name() {
        let templates = TemplateMap::new();
        let mut stack = Vec::new();
        let instance = obj(vec![("template", s("nonexistent"))]);
        let result = expand_template(&instance, &templates, &mut stack, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown template: 'nonexistent'"));
    }

    #[test]
    fn test_expand_template_with_invalid_vars() {
        let mut templates = TemplateMap::new();
        templates.insert(
            "tmpl".to_string(),
            obj(vec![("type", s("label")), ("text", s("${ns}"))]),
        );
        let mut stack = Vec::new();
        // vars block is not an object — should propagate error
        let instance = obj(vec![("template", s("tmpl")), ("vars", s("not_an_object"))]);
        let result = expand_template(&instance, &templates, &mut stack, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be an object"));
    }

    // ── parse_layout_config edge cases ───────────────────────────────

    #[test]
    fn test_parse_layout_config_unknown_type_defaults_to_stack() {
        let config = obj(vec![("layout", obj(vec![("type", s("foobar"))]))]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.component_type, "stack");
    }

    #[test]
    fn test_parse_layout_config_missing_type_defaults_to_stack() {
        let config = obj(vec![("layout", obj(vec![]))]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.component_type, "stack");
    }

    #[test]
    fn test_parse_layout_config_grid_type() {
        let config = obj(vec![("layout", obj(vec![("type", s("grid"))]))]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.component_type, "grid");
    }

    #[test]
    fn test_parse_layout_config_tiles_type() {
        let config = obj(vec![("layout", obj(vec![("type", s("tiles"))]))]);
        let layout = parse_layout_config(&config, &TemplateMap::new()).unwrap();
        assert_eq!(layout.root.component_type, "tiles");
    }

    #[test]
    fn test_parse_layout_config_no_layout_key() {
        let config = obj(vec![("app", obj(vec![("title", s("Test"))]))]);
        assert!(parse_layout_config(&config, &TemplateMap::new()).is_none());
    }

    #[test]
    fn test_parse_layout_config_template_expansion_failure_fallback() {
        // Create a config with a template reference that uses undefined vars
        // so expansion fails, but parse_layout_config falls back to raw layout
        let config = obj(vec![
            (
                "templates",
                obj(vec![(
                    "bad_tmpl",
                    obj(vec![("type", s("label")), ("text", s("${undefined}"))]),
                )]),
            ),
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![(
                            "widget",
                            obj(vec![
                                ("template", s("bad_tmpl")),
                                ("vars", obj(vec![])), // no vars defined
                            ]),
                        )]),
                    ),
                ]),
            ),
        ]);
        // Should not panic — falls back to raw layout on expansion failure
        let layout = parse_layout_config(&config, &TemplateMap::new());
        assert!(layout.is_some());
    }

    // ── parse_component_from_value edge cases ────────────────────────

    #[test]
    fn test_parse_component_non_object_returns_none() {
        assert!(parse_component_from_value(&Value::Null, None).is_none());
        assert!(parse_component_from_value(&s("string"), None).is_none());
        assert!(parse_component_from_value(&Value::Integer(1), None).is_none());
    }

    #[test]
    fn test_parse_component_missing_type_defaults_to_panel() {
        let val = obj(vec![("label", s("hello"))]);
        let node = parse_component_from_value(&val, Some("test")).unwrap();
        assert_eq!(node.component_type, "panel");
    }

    #[test]
    fn test_parse_component_binding_missing_source_target() {
        let binding = obj(vec![("mode", s("one_way"))]); // no source or target
        let val = obj(vec![("type", s("label")), ("binding", binding)]);
        let node = parse_component_from_value(&val, Some("lbl")).unwrap();
        // Should have one binding with empty source and target (from unwrap_or_default)
        assert_eq!(node.config.bindings.len(), 1);
        assert_eq!(node.config.bindings[0].source, "");
        assert_eq!(node.config.bindings[0].target, "");
    }

    #[test]
    fn test_anonymous_labels_survive_full_build_pipeline() {
        // End-to-end guard for the dev-dashboard "all labels show Median:" bug:
        // id-less labels in sibling stacks must each keep their own text after
        // parse → parse_layout_config → LayoutManager (which flattens the tree
        // into an id-keyed map). If anonymous ids collided they would collapse
        // to one entry here.
        let xml = r#"
        <nemo>
            <layout type="stack">
                <stack id="row1" direction="horizontal">
                    <label text="Mean: " />
                    <label id="stats_mean" text="--" />
                </stack>
                <stack id="row2" direction="horizontal">
                    <label text="Median: " />
                    <label id="stats_median" text="--" />
                </stack>
            </layout>
        </nemo>
        "#;

        let config = nemo_config::XmlParser::new().parse(xml).unwrap();
        let layout_config =
            parse_layout_config(&config, &TemplateMap::new()).expect("layout should parse");

        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let mut lm = LayoutManager::new(Arc::clone(&registry));
        lm.apply_layout(layout_config).unwrap();

        // Gather every label's text from the flat component map.
        let texts: Vec<String> = lm
            .component_ids()
            .into_iter()
            .filter_map(|id| lm.get_component(&id).cloned())
            .filter(|c| c.component_type == "label")
            .filter_map(|c| {
                c.properties
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(String::from)
            })
            .collect();

        // Both distinct prefix labels must be present — not two "Median: ".
        assert!(
            texts.iter().any(|t| t == "Mean: "),
            "missing 'Mean: ' label, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "Median: "),
            "missing 'Median: ' label, got {texts:?}"
        );
        assert_eq!(
            texts.iter().filter(|t| t.as_str() == "Median: ").count(),
            1,
            "'Median: ' should appear exactly once, got {texts:?}"
        );
    }

    #[test]
    fn test_on_load_handler_reads_config() {
        // `<script on-load="…">` must surface through NemoRuntime::on_load_handler
        // so App::new can call it once at startup.
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        {
            let mut cfg = rt.config.write().unwrap();
            *cfg = nemo_config::XmlParser::new()
                .parse(r#"<nemo><script src="./scripts" on-load="hydrate" /></nemo>"#)
                .unwrap();
        }
        assert_eq!(rt.on_load_handler(), Some("hydrate".to_string()));
    }

    #[test]
    fn test_on_load_handler_absent_is_none() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        {
            let mut cfg = rt.config.write().unwrap();
            *cfg = nemo_config::XmlParser::new()
                .parse(r#"<nemo><script src="./scripts" /></nemo>"#)
                .unwrap();
        }
        assert_eq!(rt.on_load_handler(), None);
    }

    #[test]
    fn test_parse_component_with_array_children() {
        let child1 = obj(vec![("type", s("button")), ("label", s("A"))]);
        let child2 = obj(vec![("type", s("button")), ("label", s("B"))]);
        let val = obj(vec![
            ("type", s("panel")),
            ("component", Value::Array(vec![child1, child2])),
        ]);
        let node = parse_component_from_value(&val, Some("parent")).unwrap();
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].component_type, "button");
        assert_eq!(node.children[1].component_type, "button");
    }

    // ── RuntimeContext error paths ────────────────────────────────────

    #[test]
    fn test_set_component_property_nonexistent_component() {
        let config = Arc::new(RwLock::new(Value::Null));
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(Arc::clone(&registry))));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        // Apply a layout with one component
        {
            let mut lm = layout_manager.write().unwrap();
            let root = LayoutNode::new("stack").with_id("root").with_child(
                LayoutNode::new("label")
                    .with_id("lbl")
                    .with_prop("text", s("Hi")),
            );
            lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
                .unwrap();
        }

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty,
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );

        // Setting property on a nonexistent component should return error
        let result =
            ctx.set_component_property("no_such_id", "text", PluginValue::String("test".into()));
        assert!(result.is_err());
    }

    // ── Router navigation (deferred apply) ─────────────────────────────

    /// End-to-end regression guard that a navigation applied through the
    /// deferred queue updates router state, projects path+params into the
    /// repository, and fires `on-leave`/`on-enter` hooks — the latter proving
    /// the apply point is *not* re-entrant with the extension write lock.
    #[test]
    fn test_apply_pending_navigation_updates_state_and_fires_hooks() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir(&scripts_dir).unwrap();
        {
            let mut f = std::fs::File::create(scripts_dir.join("handlers.rhai")).unwrap();
            writeln!(
                f,
                "fn record_enter(id, ev) {{ set_data(\"test.entered\", id); }}\n\
                 fn record_leave(id, ev) {{ set_data(\"test.left\", id); }}"
            )
            .unwrap();
        }
        let config_path = dir.path().join("app.xml");
        {
            let mut f = std::fs::File::create(&config_path).unwrap();
            write!(
                f,
                r#"<nemo>
  <app title="t"/>
  <script src="./scripts"/>
  <layout type="stack">
    <router id="main" default="/home">
      <route path="/home" on-leave="record_leave"></route>
      <route path="/users/:id" on-enter="record_enter"></route>
      <route path="*"></route>
    </router>
  </layout>
</nemo>"#
            )
            .unwrap();
        }

        let rt = NemoRuntime::new(&config_path).unwrap();
        rt.load_config().unwrap();
        rt.initialize().unwrap();

        // Lazily initialize the router to its default so the old path (/home)
        // is known and its on-leave hook can fire on the next navigation.
        assert_eq!(rt.router_current_path("main", "/home"), "/home");

        rt.enqueue_navigation(None, "/users/42".to_string());
        assert!(rt.apply_pending_navigations());

        // Current path advanced; path + params were projected into the repo.
        assert_eq!(rt.router_current_path("main", "/home"), "/users/42");
        let get = |p: &str| {
            rt.data_engine
                .repository
                .get(&nemo_data::DataPath::parse(p).unwrap())
        };
        assert_eq!(
            get("data.route.main.path").and_then(|v| v.as_str().map(String::from)),
            Some("/users/42".to_string())
        );
        assert_eq!(
            get("data.route.main.params.id").and_then(|v| v.as_str().map(String::from)),
            Some("42".to_string())
        );

        // Both lifecycle hooks fired (running Rhai via call_handler from the
        // apply point, which holds no extension lock).
        assert_eq!(
            get("data.test.left").and_then(|v| v.as_str().map(String::from)),
            Some("main".to_string())
        );
        assert_eq!(
            get("data.test.entered").and_then(|v| v.as_str().map(String::from)),
            Some("main".to_string())
        );

        // back() returns to the previous path.
        rt.push_nav_intent(NavIntent::Back { router: None });
        assert!(rt.apply_pending_navigations());
        assert_eq!(rt.router_current_path("main", "/home"), "/home");

        // Stale params from the /users/:id route were cleared on the way back.
        assert!(get("data.route.main.params.id").is_none());
    }

    /// `--route settings=/general` (explicit router id) overrides that router's
    /// starting path on lazy init, and leaves other routers on their default.
    #[test]
    fn test_initial_route_override_explicit_router() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        rt.set_initial_route("main=/table");
        // Explicit id matches without needing a component tree.
        assert_eq!(rt.router_current_path("main", "/button"), "/table");
        // A router the override does not name keeps its default.
        assert_eq!(rt.router_current_path("other", "/home"), "/home");
    }

    /// An unscoped `--route /settings` targets the primary router (resolved from
    /// the component tree).
    #[test]
    fn test_initial_route_override_primary_router() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.xml")).unwrap();
        {
            let mut lm = rt.layout_manager.write().unwrap();
            let root = LayoutNode::new("router")
                .with_id("main")
                .with_prop("default", s("/home"));
            lm.apply_layout(LayoutConfig::new(LayoutType::Stack, root))
                .unwrap();
        }
        rt.set_initial_route("/settings");
        assert_eq!(rt.router_current_path("main", "/home"), "/settings");
    }

    #[test]
    fn test_runtime_context_get_config_with_null_config() {
        let config = Arc::new(RwLock::new(Value::Null));
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        let layout_manager = Arc::new(RwLock::new(LayoutManager::new(registry)));
        let event_bus = Arc::new(EventBus::with_default_capacity());
        let repo = Arc::new(DataRepository::new());
        let dirty = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(tokio::sync::Notify::new());

        let ctx = RuntimeContext::new(
            config,
            layout_manager,
            event_bus,
            repo,
            dirty,
            notify,
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(Mutex::new(Vec::new())),
        );
        assert_eq!(ctx.get_config("any.path"), None);
    }

    // ── NemoRuntime with malformed config ────────────────────────────

    #[test]
    fn test_runtime_load_malformed_config() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad.xml");
        {
            let mut f = std::fs::File::create(&config_path).unwrap();
            writeln!(f, "<nemo><unclosed>not valid xml").unwrap();
        }
        let rt = NemoRuntime::new(&config_path).unwrap();
        let result = rt.load_config();
        assert!(result.is_err(), "Malformed XML should produce an error");
    }

    #[test]
    fn test_runtime_load_empty_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("empty.xml");
        {
            std::fs::File::create(&config_path).unwrap();
        }
        let rt = NemoRuntime::new(&config_path).unwrap();
        // Empty file should load without error
        assert!(rt.load_config().is_ok());
    }

    // ── create_data_source edge cases ────────────────────────────────

    #[test]
    fn test_create_data_source_empty_type() {
        let config = obj(vec![("type", s(""))]);
        assert!(nemo_data::create_source("x", "", &config).is_none());
    }

    #[test]
    fn test_create_data_source_type_case_sensitivity() {
        // "HTTP" (uppercase) should not match "http"
        let config = obj(vec![("type", s("HTTP")), ("url", s("https://example.com"))]);
        assert!(nemo_data::create_source("api", "HTTP", &config).is_none());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating leaf `Value` instances.
    fn arb_leaf_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Integer),
            (-1e10f64..1e10f64).prop_map(Value::Float),
            "[a-zA-Z0-9_]{0,15}".prop_map(Value::String),
        ]
    }

    /// Strategy for nested `Value` (up to 2 levels deep).
    fn arb_value() -> impl Strategy<Value = Value> {
        arb_leaf_value().prop_recursive(2, 12, 3, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..3).prop_map(Value::Array),
                prop::collection::vec(("[a-z]{1,6}".prop_map(String::from), inner), 0..3).prop_map(
                    |pairs| {
                        Value::Object(
                            pairs
                                .into_iter()
                                .collect::<indexmap::IndexMap<String, Value>>(),
                        )
                    }
                ),
            ]
        })
    }

    proptest! {
        #[test]
        fn value_to_plugin_value_roundtrip(val in arb_value()) {
            let plugin = value_to_plugin_value(&val);
            let back = plugin_value_to_config_value(plugin);
            prop_assert_eq!(&val, &back);
        }

        #[test]
        fn plugin_value_to_json_does_not_panic(val in arb_value()) {
            let plugin = value_to_plugin_value(&val);
            let _ = plugin_value_to_json(plugin);
        }

        #[test]
        fn get_nested_value_single_key(key in "[a-z]{1,8}", val in arb_leaf_value()) {
            let config = {
                let mut m = indexmap::IndexMap::new();
                m.insert(key.clone(), val.clone());
                Value::Object(m)
            };
            prop_assert_eq!(get_nested_value(&config, &key), Some(&val));
        }

        #[test]
        fn get_nested_value_two_level(
            k1 in "[a-z]{1,5}",
            k2 in "[a-z]{1,5}",
            val in arb_leaf_value(),
        ) {
            let inner = {
                let mut m = indexmap::IndexMap::new();
                m.insert(k2.clone(), val.clone());
                Value::Object(m)
            };
            let config = {
                let mut m = indexmap::IndexMap::new();
                m.insert(k1.clone(), inner);
                Value::Object(m)
            };
            let path = format!("{}.{}", k1, k2);
            prop_assert_eq!(get_nested_value(&config, &path), Some(&val));
        }

        #[test]
        fn interpolate_no_vars_passthrough(s_val in "[a-zA-Z0-9 ]{0,20}") {
            // Strings without ${} should pass through unchanged
            if !s_val.contains("${") {
                let vars = HashMap::new();
                let input = Value::String(s_val.clone());
                let result = interpolate_variables(&input, &vars, "test").unwrap();
                prop_assert_eq!(result, Value::String(s_val));
            }
        }

        #[test]
        fn deep_merge_non_object_overlay_wins(val in arb_leaf_value()) {
            // When overlay is not an object, overlay value is returned
            let base = Value::String("base".to_string());
            let merged = deep_merge_values(&base, &val);
            prop_assert_eq!(&merged, &val);
        }

        #[test]
        fn deep_merge_two_objects_contains_all_keys(
            pairs1 in prop::collection::vec(("[a-e]{1}".prop_map(String::from), arb_leaf_value()), 1..3),
            pairs2 in prop::collection::vec(("[f-j]{1}".prop_map(String::from), arb_leaf_value()), 1..3),
        ) {
            // When merging two objects with disjoint keys, all keys appear
            let obj1 = Value::Object(pairs1.iter().cloned().collect());
            let obj2 = Value::Object(pairs2.iter().cloned().collect());
            let merged = deep_merge_values(&obj1, &obj2);
            let Value::Object(m) = merged else {
                panic!("Expected object");
            };
            for (k, _) in &pairs1 {
                prop_assert!(m.contains_key(k), "Missing key {} from base", k);
            }
            // overlay keys (excluding template/vars) should be present
            for (k, _) in &pairs2 {
                if k != "template" && k != "vars" {
                    prop_assert!(m.contains_key(k), "Missing key {} from overlay", k);
                }
            }
        }

        #[test]
        fn extract_templates_from_no_templates_is_empty(val in arb_leaf_value()) {
            // Any value without a "templates" key should produce empty map
            let templates = extract_templates(&val);
            prop_assert!(templates.is_empty());
        }
    }
}
