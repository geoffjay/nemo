//! Nemo Extension Manager - RHAI scripting and native plugin loading.
//!
//! This crate provides the extension system for Nemo applications, including:
//! - RHAI script loading and execution
//! - Native plugin loading via libloading
//! - Extension discovery and registration
//! - Sandboxed script execution

pub mod error;
pub mod loader;
pub mod plugin;
pub mod registry;
pub mod rhai_engine;

pub use error::ExtensionError;
pub use loader::{ExtensionLoader, ExtensionManifest, ExtensionType};
pub use plugin::{LoadedPlugin, PluginHost, PluginInitResult};
pub use registry::ExtensionRegistry;
pub use rhai_engine::{RhaiConfig, RhaiEngine, RhaiFeatures};

use nemo_plugin_api::{PluginContext, PluginValue};
use std::collections::HashMap;
use std::sync::Arc;

/// The extension manager coordinates all extension operations.
pub struct ExtensionManager {
    /// Extension registry.
    pub registry: ExtensionRegistry,
    /// RHAI engine for script execution.
    pub rhai_engine: RhaiEngine,
    /// Plugin host for native plugins.
    pub plugin_host: PluginHost,
    /// WASM plugin host.
    #[cfg(feature = "wasm")]
    pub wasm_host: nemo_wasm::WasmHost,
    /// Extension loader.
    loader: ExtensionLoader,
    /// Templates registered by native plugins (name → PluginValue tree).
    plugin_templates: HashMap<String, PluginValue>,
}

impl ExtensionManager {
    /// Creates a new extension manager.
    pub fn new() -> Self {
        Self {
            registry: ExtensionRegistry::new(),
            rhai_engine: RhaiEngine::new(RhaiConfig::default()),
            plugin_host: PluginHost::new(),
            #[cfg(feature = "wasm")]
            wasm_host: nemo_wasm::WasmHost::new().expect("Failed to create WasmHost"),
            loader: ExtensionLoader::new(),
            plugin_templates: HashMap::new(),
        }
    }

    /// Creates an extension manager with custom configuration.
    pub fn with_config(rhai_config: RhaiConfig) -> Self {
        Self {
            registry: ExtensionRegistry::new(),
            rhai_engine: RhaiEngine::new(rhai_config),
            plugin_host: PluginHost::new(),
            #[cfg(feature = "wasm")]
            wasm_host: nemo_wasm::WasmHost::new().expect("Failed to create WasmHost"),
            loader: ExtensionLoader::new(),
            plugin_templates: HashMap::new(),
        }
    }

    /// Adds a script search path.
    pub fn add_script_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.loader.add_script_path(path);
    }

    /// Adds a plugin search path.
    pub fn add_plugin_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.loader.add_plugin_path(path);
    }

    /// Adds a WASM plugin search path.
    #[cfg(feature = "wasm")]
    pub fn add_wasm_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.loader.add_wasm_path(path);
    }

    /// Discovers all extensions in configured paths.
    pub fn discover(&self) -> Result<Vec<ExtensionManifest>, ExtensionError> {
        self.loader.discover()
    }

    /// Loads a script by path.
    pub fn load_script(&mut self, path: &std::path::Path) -> Result<String, ExtensionError> {
        let source = std::fs::read_to_string(path).map_err(|e| ExtensionError::LoadError {
            id: path.to_string_lossy().to_string(),
            reason: e.to_string(),
        })?;

        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        self.rhai_engine.load_script(&id, &source)?;
        self.registry
            .register_script(id.clone(), path.to_path_buf());

        Ok(id)
    }

    /// Loads a plugin by path.
    pub fn load_plugin(&mut self, path: &std::path::Path) -> Result<String, ExtensionError> {
        let id = self.plugin_host.load(path)?;
        self.registry
            .register_plugin(id.clone(), path.to_path_buf());
        Ok(id)
    }

    /// Calls a script function.
    pub fn call_script<T: Clone + Send + Sync + 'static>(
        &self,
        script_id: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, ExtensionError> {
        self.rhai_engine.call(script_id, function, args)
    }

    /// Evaluates a RHAI expression.
    pub fn eval<T: Clone + Send + Sync + 'static>(&self, expr: &str) -> Result<T, ExtensionError> {
        self.rhai_engine.eval(expr)
    }

    /// Reloads a script.
    pub fn reload_script(&mut self, id: &str) -> Result<(), ExtensionError> {
        if let Some(path) = self.registry.get_script_path(id) {
            let source = std::fs::read_to_string(&path).map_err(|e| ExtensionError::LoadError {
                id: id.to_string(),
                reason: e.to_string(),
            })?;
            self.rhai_engine.reload_script(id, &source)
        } else {
            Err(ExtensionError::NotFound { id: id.to_string() })
        }
    }

    /// Unloads a plugin.
    pub fn unload_plugin(&mut self, id: &str) -> Result<(), ExtensionError> {
        self.plugin_host.unload(id)?;
        self.registry.unregister_plugin(id);
        Ok(())
    }

    /// Lists all loaded scripts.
    pub fn list_scripts(&self) -> Vec<String> {
        self.registry.list_scripts()
    }

    /// Lists all loaded plugins.
    pub fn list_plugins(&self) -> Vec<String> {
        self.registry.list_plugins()
    }

    /// Loads a WASM plugin by path.
    #[cfg(feature = "wasm")]
    pub fn load_wasm_plugin(&mut self, path: &std::path::Path) -> Result<String, ExtensionError> {
        let id = self.wasm_host.load(path)?;
        self.registry
            .register_wasm_plugin(id.clone(), path.to_path_buf());
        Ok(id)
    }

    /// Unloads a WASM plugin.
    #[cfg(feature = "wasm")]
    pub fn unload_wasm_plugin(&mut self, id: &str) -> Result<(), ExtensionError> {
        self.wasm_host.unload(id)?;
        self.registry.unregister_wasm_plugin(id);
        Ok(())
    }

    /// Lists all loaded WASM plugins.
    #[cfg(feature = "wasm")]
    pub fn list_wasm_plugins(&self) -> Vec<String> {
        self.registry.list_wasm_plugins()
    }

    /// Ticks all WASM plugins that have an elapsed interval.
    #[cfg(feature = "wasm")]
    pub fn tick_wasm_plugins(&mut self) {
        self.wasm_host.tick_all();
    }

    /// Initializes all loaded native plugins by calling their entry points.
    ///
    /// Must be called after `register_context()` so the `PluginContext` is available.
    /// Collects templates registered by plugins for later use in layout expansion.
    pub fn init_plugins(&mut self, context: Arc<dyn PluginContext>) {
        let results = self.plugin_host.init_all_plugins(context);

        for (id, result) in results {
            match result {
                Ok(init_result) => {
                    for (name, template) in init_result.templates {
                        tracing::info!("Plugin '{}' registered template '{}'", id, name);
                        self.plugin_templates.insert(name, template);
                    }
                    tracing::info!("Plugin '{}' initialized successfully", id);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize plugin '{}': {}", id, e);
                }
            }
        }
    }

    /// Returns templates registered by native plugins.
    pub fn plugin_templates(&self) -> &HashMap<String, PluginValue> {
        &self.plugin_templates
    }

    /// Registers the extension context API with the RHAI engine.
    pub fn register_context(&mut self, context: Arc<dyn PluginContext>) {
        #[cfg(feature = "wasm")]
        self.wasm_host.set_context(Arc::clone(&context));
        self.rhai_engine.register_context(context);
    }
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_manager_creation() {
        let manager = ExtensionManager::new();
        assert!(manager.list_scripts().is_empty());
        assert!(manager.list_plugins().is_empty());
    }

    #[test]
    fn test_eval_expression() {
        let manager = ExtensionManager::new();
        let result: i64 = manager.eval("40 + 2").unwrap();
        assert_eq!(result, 42);
    }
}
