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
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::RwLock;
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
    /// Sink configurations for outbound data publishing.
    pub sink_configs: Arc<std::sync::RwLock<HashMap<String, SinkConfig>>>,
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
            sink_configs: Arc::new(std::sync::RwLock::new(HashMap::new())),
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
            let loaded = self
                .config_loader
                .load(&self.config_path)
                .map_err(|e| anyhow::anyhow!("Failed to load config file: {}", e))?;

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

        // Set up data sources from configuration
        self.setup_data_sources()?;

        // Set up data sinks from configuration
        self.setup_data_sinks()?;

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
                                            tracing::warn!(
                                                "Failed to load script {:?}: {}",
                                                path,
                                                e
                                            )
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
                            self.tokio_runtime.block_on(async {
                                let mut ext = self.extension_manager.write().await;
                                match ext.load_script(&script_path) {
                                    Ok(id) => info!("Loaded script: {}", id),
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to load script {:?}: {}",
                                            script_path,
                                            e
                                        )
                                    }
                                }
                            });
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
        ));

        self.tokio_runtime.block_on(async {
            let mut ext = self.extension_manager.write().await;
            ext.register_context(context);
        });

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
        self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            get_nested_value(&config, path).cloned()
        })
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

        self.tokio_runtime.block_on(async {
            let ext = self.extension_manager.read().await;
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

    /// Parses data source configuration and registers sources with the DataFlowEngine.
    fn setup_data_sources(&self) -> Result<()> {
        let data_config = self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            config.get("data").cloned()
        });

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
        let data_config = self.tokio_runtime.block_on(async {
            let config = self.config.read().await;
            config.get("data").cloned()
        });

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
}

impl RuntimeContext {
    /// Creates a new runtime context.
    pub fn new(
        config: Arc<RwLock<Value>>,
        layout_manager: Arc<RwLock<LayoutManager>>,
        event_bus: Arc<EventBus>,
        data_repository: Arc<DataRepository>,
        data_dirty: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            layout_manager,
            event_bus,
            data_repository,
            data_dirty,
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
