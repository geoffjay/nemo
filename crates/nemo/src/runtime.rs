//! Nemo runtime - manages all subsystems.

use anyhow::{Context, Result};
use nemo_config::{ConfigurationLoader, SchemaRegistry, Value};
use nemo_data::{DataFlowEngine, DataRepository};
use nemo_events::EventBus;
use nemo_extension::ExtensionManager;
use nemo_integration::IntegrationGateway;
use nemo_layout::{LayoutConfig, LayoutManager, LayoutNode, LayoutType};
use nemo_plugin_api::{LogLevel, PluginContext, PluginError, PluginValue};
use nemo_registry::{register_all_builtins, ComponentRegistry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::runtime::Runtime as TokioRuntime;
use tracing::{debug, info};

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
#[allow(dead_code)]
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
    /// Flag indicating data has changed and UI needs re-render.
    pub data_dirty: Arc<AtomicBool>,
    /// Notification signal for waking the UI when data changes.
    pub data_notify: Arc<tokio::sync::Notify>,
    /// Sink configurations for outbound data publishing.
    pub sink_configs: Arc<RwLock<HashMap<String, SinkConfig>>>,
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
        #[allow(clippy::arc_with_non_send_sync)]
        let extension_manager = Arc::new(RwLock::new(ExtensionManager::new()));
        #[allow(clippy::arc_with_non_send_sync)]
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
            data_dirty: Arc::new(AtomicBool::new(false)),
            data_notify: Arc::new(tokio::sync::Notify::new()),
            sink_configs: Arc::new(RwLock::new(HashMap::new())),
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
        let mut ext = self
            .extension_manager
            .write()
            .expect("extension_manager lock poisoned");
        ext.add_script_path(dir.join("scripts"));
        ext.add_plugin_path(dir.join("plugins"));
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
                    nemo_extension::ExtensionType::Plugin => {
                        if let Err(e) = ext.load_plugin(&manifest.path) {
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

    /// Loads scripts specified in configuration.
    fn load_scripts_from_config(&self) -> Result<()> {
        let scripts_config = {
            let config = self.config.read().expect("config lock poisoned");
            config.get("scripts").cloned()
        };

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

        // Register the runtime context with the extension manager for API access
        let context = Arc::new(RuntimeContext::new(
            Arc::clone(&self.config),
            Arc::clone(&self.layout_manager),
            Arc::clone(&self.event_bus),
            Arc::clone(&self.data_engine.repository),
            Arc::clone(&self.data_dirty),
            Arc::clone(&self.data_notify),
        ));

        {
            let mut ext = self
                .extension_manager
                .write()
                .expect("extension_manager lock poisoned");
            ext.register_context(context);
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
    #[allow(dead_code)]
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// Returns the component registry.
    #[allow(dead_code)]
    pub fn registry(&self) -> &Arc<ComponentRegistry> {
        &self.registry
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

        let ext = self
            .extension_manager
            .read()
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
        let layout_config = {
            let config = self.config.read().expect("config lock poisoned");
            parse_layout_config(&config)
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

                match create_data_source(source_name, source_type, source_config) {
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

                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
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
                });
            }
        }
    }

    /// Checks for pending data updates and propagates them through bindings.
    /// Returns true if any updates were applied (indicating the UI needs re-render).
    pub fn apply_pending_data_updates(&self) -> bool {
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

        any_updates
    }

    /// Parses sink configuration from HCL and stores sink configs.
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

/// Creates a DataSource from HCL configuration.
fn create_data_source(
    name: &str,
    source_type: &str,
    config: &Value,
) -> Option<Box<dyn nemo_data::DataSource>> {
    match source_type {
        "timer" => {
            let interval_secs = config
                .get("interval")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(1);
            let immediate = config
                .get("immediate")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let timer_config = nemo_data::TimerSourceConfig {
                id: name.to_string(),
                interval: std::time::Duration::from_secs(interval_secs as u64),
                immediate,
                payload: config.get("payload").cloned(),
            };
            Some(Box::new(nemo_data::TimerSource::new(timer_config)))
        }
        "http" => {
            let url = config.get("url").and_then(|v| v.as_str())?.to_string();
            let interval = config
                .get("interval")
                .and_then(|v| v.as_i64())
                .map(|secs| std::time::Duration::from_secs(secs as u64));

            let http_config = nemo_data::HttpSourceConfig {
                id: name.to_string(),
                url,
                interval,
                ..Default::default()
            };
            Some(Box::new(nemo_data::HttpSource::new(http_config)))
        }
        "websocket" => {
            let url = config.get("url").and_then(|v| v.as_str())?.to_string();
            let ws_config = nemo_data::WebSocketSourceConfig {
                id: name.to_string(),
                url,
                ..Default::default()
            };
            Some(Box::new(nemo_data::WebSocketSource::new(ws_config)))
        }
        "mqtt" => {
            let host = config
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("localhost")
                .to_string();
            let port = config.get("port").and_then(|v| v.as_i64()).unwrap_or(1883) as u16;
            let topics: Vec<String> = config
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let qos = config.get("qos").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
            let client_id = config
                .get("client_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mqtt_config = nemo_data::MqttSourceConfig {
                id: name.to_string(),
                host,
                port,
                topics,
                qos,
                client_id,
            };
            Some(Box::new(nemo_data::MqttSource::new(mqtt_config)))
        }
        "redis" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("redis://127.0.0.1:6379")
                .to_string();
            let channels: Vec<String> = config
                .get("channels")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let redis_config = nemo_data::RedisSourceConfig {
                id: name.to_string(),
                url,
                channels,
            };
            Some(Box::new(nemo_data::RedisSource::new(redis_config)))
        }
        "nats" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("nats://127.0.0.1:4222")
                .to_string();
            let subjects: Vec<String> = config
                .get("subjects")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let nats_config = nemo_data::NatsSourceConfig {
                id: name.to_string(),
                url,
                subjects,
            };
            Some(Box::new(nemo_data::NatsSource::new(nats_config)))
        }
        "file" => {
            let path = config.get("path").and_then(|v| v.as_str())?.to_string();
            let watch = config
                .get("watch")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let format_str = config
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("raw");
            let format = match format_str {
                "json" => nemo_data::FileFormat::Json,
                "lines" => nemo_data::FileFormat::Lines,
                _ => nemo_data::FileFormat::Raw,
            };

            let file_config = nemo_data::FileSourceConfig {
                id: name.to_string(),
                path: std::path::PathBuf::from(path),
                format,
                watch,
                ..Default::default()
            };
            Some(Box::new(nemo_data::FileSource::new(file_config)))
        }
        _ => None,
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

type TemplateMap = HashMap<String, Value>;

/// Extracts template definitions from the parsed config.
///
/// HCL `templates { template "name" { ... } }` parses as:
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
        if key == "template" {
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

    // Collect template child IDs so we can prefix them later for uniqueness
    let template_child_ids: Vec<String> = template_def
        .get("component")
        .and_then(|c| c.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Recursively expand the template itself (template-of-template)
    expansion_stack.push(template_name.clone());
    let expanded_template = expand_template(&template_def, templates, expansion_stack, None)?;
    expansion_stack.pop();

    // Extract instance children before merging
    let instance_children = obj.get("component").cloned();

    // Merge instance properties (without children) onto the template first
    let instance_without_children = strip_keys(instance, &["component"]);
    let merged = deep_merge_values(&expanded_template, &instance_without_children);

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
    let stripped = strip_keys(&with_slots, &["template", "slot"]);

    // Prefix template-originated child IDs with the instance ID for uniqueness.
    // This prevents ID collisions when the same template is used by multiple
    // instances (e.g., all content pages having a child named "inner").
    let scoped = if let Some(parent_id) = instance_id {
        scope_template_children(&stripped, parent_id, &template_child_ids)
    } else {
        stripped
    };

    // Recurse into merged children
    expand_children(&scoped, templates, expansion_stack)
}

/// Renames template-originated child IDs by prefixing them with the parent
/// instance ID. Only children whose original ID is in `template_child_ids`
/// are renamed; instance children keep their original IDs.
fn scope_template_children(value: &Value, parent_id: &str, template_child_ids: &[String]) -> Value {
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
            new_components.insert(new_id, child.clone());
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

// ── Layout parsing ───────────────────────────────────────────────────────

/// Parses layout configuration from a Value.
fn parse_layout_config(config: &Value) -> Option<LayoutConfig> {
    let layout = config.get("layout")?;
    let templates = extract_templates(config);

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
                // Nested components - HCL labeled blocks are parsed as objects
                // e.g., component "button" { ... } becomes component: { button: {...} }
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
}

impl RuntimeContext {
    /// Creates a new runtime context.
    pub fn new(
        config: Arc<RwLock<Value>>,
        layout_manager: Arc<RwLock<LayoutManager>>,
        event_bus: Arc<EventBus>,
        data_repository: Arc<DataRepository>,
        data_dirty: Arc<AtomicBool>,
        data_notify: Arc<tokio::sync::Notify>,
    ) -> Self {
        Self {
            config,
            layout_manager,
            event_bus,
            data_repository,
            data_dirty,
            data_notify,
        }
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
        let data_path = nemo_data::DataPath::parse(&format!("data.{}", path))
            .map_err(|e| PluginError::InvalidConfig(e.to_string()))?;
        let config_value = plugin_value_to_config_value(value);
        self.data_repository
            .set(&data_path, config_value)
            .map_err(|e| PluginError::InvalidConfig(e.to_string()))?;
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
mod template_tests {
    use super::*;
    use indexmap::IndexMap;

    /// Helper to build a Value::Object from key-value pairs.
    fn obj(pairs: Vec<(&str, Value)>) -> Value {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        Value::Object(map)
    }

    fn s(val: &str) -> Value {
        Value::String(val.to_string())
    }

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
    use super::*;
    use indexmap::IndexMap;

    /// Helper to build a Value::Object from key-value pairs.
    fn obj(pairs: Vec<(&str, Value)>) -> Value {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        Value::Object(map)
    }

    fn s(val: &str) -> Value {
        Value::String(val.to_string())
    }

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
        let source = create_data_source("test_timer", "timer", &config);
        assert!(source.is_some());
        assert_eq!(source.unwrap().id(), "test_timer");
    }

    #[test]
    fn test_create_data_source_timer_defaults() {
        // Timer with no interval/immediate should use defaults
        let config = obj(vec![("type", s("timer"))]);
        let source = create_data_source("t", "timer", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_http() {
        let config = obj(vec![
            ("type", s("http")),
            ("url", s("https://example.com/api")),
            ("interval", Value::Integer(30)),
        ]);
        let source = create_data_source("api", "http", &config);
        assert!(source.is_some());
        assert_eq!(source.unwrap().id(), "api");
    }

    #[test]
    fn test_create_data_source_http_missing_url() {
        let config = obj(vec![("type", s("http"))]);
        let source = create_data_source("api", "http", &config);
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
        let source = create_data_source("ws", "websocket", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_websocket_missing_url() {
        let config = obj(vec![("type", s("websocket"))]);
        assert!(create_data_source("ws", "websocket", &config).is_none());
    }

    #[test]
    fn test_create_data_source_mqtt() {
        let config = obj(vec![
            ("type", s("mqtt")),
            ("host", s("broker.local")),
            ("port", Value::Integer(1883)),
            ("topics", Value::Array(vec![s("sensor/+")])),
        ]);
        let source = create_data_source("mqtt", "mqtt", &config);
        assert!(source.is_some());
    }

    #[test]
    fn test_create_data_source_mqtt_defaults() {
        let config = obj(vec![("type", s("mqtt"))]);
        let source = create_data_source("mqtt", "mqtt", &config);
        assert!(source.is_some(), "MQTT should use default host/port");
    }

    #[test]
    fn test_create_data_source_redis() {
        let config = obj(vec![
            ("type", s("redis")),
            ("url", s("redis://127.0.0.1:6379")),
            ("channels", Value::Array(vec![s("events")])),
        ]);
        assert!(create_data_source("r", "redis", &config).is_some());
    }

    #[test]
    fn test_create_data_source_nats() {
        let config = obj(vec![
            ("type", s("nats")),
            ("url", s("nats://127.0.0.1:4222")),
            ("subjects", Value::Array(vec![s("updates.>")])),
        ]);
        assert!(create_data_source("n", "nats", &config).is_some());
    }

    #[test]
    fn test_create_data_source_file() {
        let config = obj(vec![
            ("type", s("file")),
            ("path", s("/tmp/data.json")),
            ("format", s("json")),
            ("watch", Value::Bool(true)),
        ]);
        assert!(create_data_source("f", "file", &config).is_some());
    }

    #[test]
    fn test_create_data_source_file_missing_path() {
        let config = obj(vec![("type", s("file"))]);
        assert!(create_data_source("f", "file", &config).is_none());
    }

    #[test]
    fn test_create_data_source_unknown_type() {
        let config = obj(vec![("type", s("unknown"))]);
        assert!(create_data_source("x", "unknown", &config).is_none());
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
        let layout = parse_layout_config(&config).unwrap();
        assert_eq!(layout.root.children.len(), 1);
        assert_eq!(layout.root.children[0].component_type, "button");
    }

    #[test]
    fn test_parse_layout_config_dock() {
        let config = obj(vec![("layout", obj(vec![("type", s("dock"))]))]);
        let layout = parse_layout_config(&config).unwrap();
        assert_eq!(layout.root.component_type, "dock");
    }

    #[test]
    fn test_parse_layout_config_missing() {
        let config = obj(vec![("app", obj(vec![]))]);
        assert!(parse_layout_config(&config).is_none());
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
        let layout = parse_layout_config(&config).unwrap();
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
        let layout = parse_layout_config(&config).unwrap();
        let lbl = &layout.root.children[0];
        assert_eq!(lbl.config.bindings.len(), 1);
        assert_eq!(lbl.config.bindings[0].source, "data.sensors.temperature");
        assert_eq!(lbl.config.bindings[0].target, "text");
    }

    // ── NemoRuntime basic construction ────────────────────────────────

    #[test]
    fn test_runtime_new_nonexistent_config() {
        let rt = NemoRuntime::new(Path::new("/nonexistent/config.hcl")).unwrap();
        // Should succeed — config file is checked lazily in load_config
        assert!(rt.get_config("anything").is_none());
    }

    #[test]
    fn test_runtime_load_config_missing_file() {
        let rt = NemoRuntime::new(Path::new("/does/not/exist.hcl")).unwrap();
        // load_config should succeed gracefully when file doesn't exist
        assert!(rt.load_config().is_ok());
    }

    #[test]
    fn test_runtime_get_config_empty() {
        let rt = NemoRuntime::new(Path::new("/tmp/empty.hcl")).unwrap();
        assert!(rt.get_config("app.title").is_none());
    }

    // ── call_handler parsing ──────────────────────────────────────────

    #[test]
    fn test_call_handler_with_script_prefix() {
        // Just verify the parsing logic — handler execution will warn
        // about missing scripts, which is fine for this test
        let rt = NemoRuntime::new(Path::new("/tmp/test.hcl")).unwrap();
        rt.load_config().unwrap();
        rt.initialize().unwrap();
        // Should not panic; the handler will log a warning
        rt.call_handler("my_script::on_click", "btn1", "click");
    }

    #[test]
    fn test_call_handler_without_script_prefix() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.hcl")).unwrap();
        rt.load_config().unwrap();
        rt.initialize().unwrap();
        // Should default to "handlers" script
        rt.call_handler("on_click", "btn1", "click");
    }

    // ── apply_pending_data_updates ────────────────────────────────────

    #[test]
    fn test_apply_pending_data_updates_when_clean() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.hcl")).unwrap();
        // data_dirty starts false, should return false
        assert!(!rt.apply_pending_data_updates());
    }

    #[test]
    fn test_apply_pending_data_updates_when_dirty_no_sources() {
        let rt = NemoRuntime::new(Path::new("/tmp/test.hcl")).unwrap();
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

        let ctx = RuntimeContext::new(config, layout_manager, event_bus, repo, dirty, notify);
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

        let ctx = RuntimeContext::new(config, layout_manager, event_bus, repo, dirty, notify);

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

        let ctx = RuntimeContext::new(config, layout_manager, event_bus, repo, dirty, notify);

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
            m.insert("ratio".to_string(), Value::Float(3.14));
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
            let mut m = HashMap::new();
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
    use super::*;
    use indexmap::IndexMap;

    fn obj(pairs: Vec<(&str, Value)>) -> Value {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        Value::Object(map)
    }

    fn s(val: &str) -> Value {
        Value::String(val.to_string())
    }

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
        let schema_registry = std::sync::Arc::new(SchemaRegistry::new());
        let loader = ConfigurationLoader::new(schema_registry);

        let hcl = r#"
templates {
  template "page" {
    type    = "panel"
    visible = false

    component "inner" {
      type = "stack"
      slot = true
    }
  }
}

layout {
  type = "stack"

  component "page_a" {
    template = "page"

    component "child_a" {
      type = "label"
    }
  }

  component "page_b" {
    template = "page"

    component "child_b" {
      type = "label"
    }
  }
}
"#;

        let config = loader.load_string(hcl, "test").expect("HCL parse failed");
        let layout_config = parse_layout_config(&config).expect("Layout parse failed");
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
        let schema_registry = std::sync::Arc::new(SchemaRegistry::new());
        let loader = ConfigurationLoader::new(schema_registry);

        let hcl = r#"
templates {
  template "nav" {
    type     = "button"
    on_click = "on_nav"
  }
}

layout {
  type = "stack"

  component "nav_btn" {
    template = "nav"
    label    = "Test"
  }
}
"#;

        let config = loader.load_string(hcl, "test").expect("HCL parse failed");
        let layout_config = parse_layout_config(&config).expect("Layout parse failed");

        let nav = &layout_config.root.children[0];
        assert_eq!(
            nav.handlers.get("click").map(|s| s.as_str()),
            Some("on_nav")
        );
    }

    #[test]
    fn test_template_integration() {
        // Parse real HCL through the config loader
        let schema_registry = std::sync::Arc::new(SchemaRegistry::new());
        let loader = ConfigurationLoader::new(schema_registry);

        let hcl = r#"
templates {
  template "nav" {
    type         = "button"
    variant      = "ghost"
    size         = "sm"
    on_click     = "on_nav"
  }

  template "page" {
    type    = "panel"
    visible = false

    component "inner" {
      type      = "stack"
      direction = "vertical"
      slot      = true
    }
  }
}

layout {
  type = "stack"

  component "nav_btn" {
    template = "nav"
    label    = "Button"
  }

  component "page_btn" {
    template = "page"
    visible  = true

    component "title" {
      type = "label"
      text = "Button Page"
    }
  }
}
"#;

        let config = loader.load_string(hcl, "test").expect("HCL parse failed");
        let layout_config = parse_layout_config(&config).expect("Layout parse failed");

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
        assert!(nav.config.properties.get("template").is_none());

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
