//! WASM plugin host for Nemo.
//!
//! Loads WebAssembly Component Model plugins using wasmtime, providing
//! sandboxed execution with host-API access via WIT bindings.

pub mod convert;
pub mod host_impl;

use host_impl::HostState;
use nemo_plugin_api::{PluginContext, PluginManifest as NativePluginManifest};
use semver::Version;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, info, warn};
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;

wasmtime::component::bindgen!({
    path: "wit/nemo-plugin.wit",
    world: "nemo-plugin",
});

/// Errors from the WASM plugin subsystem.
#[derive(Debug, Error)]
pub enum WasmError {
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),

    #[error("Failed to load WASM plugin from '{path}': {reason}")]
    Load { path: String, reason: String },

    #[error("WASM plugin '{id}' is already loaded")]
    AlreadyLoaded { id: String },

    #[error("WASM plugin not found: {id}")]
    NotFound { id: String },

    #[error("No plugin context registered")]
    NoContext,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A loaded WASM plugin instance.
pub struct WasmPlugin {
    pub id: String,
    pub manifest: NativePluginManifest,
    store: Store<HostState>,
    bindings: NemoPlugin,
    tick_interval_ms: u64,
    last_tick: Instant,
}

/// Host for managing WASM plugin instances.
pub struct WasmHost {
    engine: Engine,
    linker: Linker<HostState>,
    plugins: HashMap<String, WasmPlugin>,
    context: Option<Arc<dyn PluginContext>>,
}

impl WasmHost {
    /// Creates a new WASM host with a shared engine and pre-configured linker.
    pub fn new() -> Result<Self, WasmError> {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::<HostState>::new(&engine);

        // Add WASI imports
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;

        // Add our host-api imports
        NemoPlugin::add_to_linker(&mut linker, |state| state)?;

        Ok(Self {
            engine,
            linker,
            plugins: HashMap::new(),
            context: None,
        })
    }

    /// Sets the plugin context used by all subsequently loaded plugins.
    pub fn set_context(&mut self, context: Arc<dyn PluginContext>) {
        self.context = Some(context);
    }

    /// Loads a WASM component plugin from the given path.
    pub fn load(&mut self, path: &Path) -> Result<String, WasmError> {
        let context = self.context.clone().ok_or(WasmError::NoContext)?;

        let bytes = std::fs::read(path).map_err(|e| WasmError::Load {
            path: path.to_string_lossy().to_string(),
            reason: e.to_string(),
        })?;

        let component =
            Component::from_binary(&self.engine, &bytes).map_err(|e| WasmError::Load {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        let wasi_ctx = WasiCtxBuilder::new().build();
        let host_state = HostState::new(context, wasi_ctx);
        let mut store = Store::new(&self.engine, host_state);

        let bindings =
            NemoPlugin::instantiate(&mut store, &component, &self.linker).map_err(|e| {
                WasmError::Load {
                    path: path.to_string_lossy().to_string(),
                    reason: e.to_string(),
                }
            })?;

        // Get the manifest from the plugin
        let wit_manifest = bindings
            .call_get_manifest(&mut store)
            .map_err(|e| WasmError::Load {
                path: path.to_string_lossy().to_string(),
                reason: format!("get_manifest failed: {}", e),
            })?;

        let id = wit_manifest.id.clone();

        if self.plugins.contains_key(&id) {
            return Err(WasmError::AlreadyLoaded { id });
        }

        let manifest = NativePluginManifest::new(
            &wit_manifest.id,
            &wit_manifest.name,
            Version::parse(&wit_manifest.version).unwrap_or_else(|_| Version::new(0, 1, 0)),
        )
        .with_description(&wit_manifest.description);

        info!(
            "Loaded WASM plugin: {} v{}",
            manifest.name, manifest.version
        );

        // Call init
        bindings
            .call_init(&mut store)
            .map_err(|e| WasmError::Load {
                path: path.to_string_lossy().to_string(),
                reason: format!("init failed: {}", e),
            })?;

        debug!("WASM plugin '{}' initialized", id);

        // Call tick once to get the initial interval
        let tick_interval_ms = bindings
            .call_tick(&mut store)
            .map_err(|e| WasmError::Load {
                path: path.to_string_lossy().to_string(),
                reason: format!("initial tick failed: {}", e),
            })?;

        debug!("WASM plugin '{}' tick interval: {}ms", id, tick_interval_ms);

        let plugin = WasmPlugin {
            id: id.clone(),
            manifest,
            store,
            bindings,
            tick_interval_ms,
            last_tick: Instant::now(),
        };

        self.plugins.insert(id.clone(), plugin);
        Ok(id)
    }

    /// Unloads a plugin by ID.
    pub fn unload(&mut self, id: &str) -> Result<(), WasmError> {
        self.plugins
            .remove(id)
            .ok_or_else(|| WasmError::NotFound { id: id.to_string() })?;
        info!("Unloaded WASM plugin: {}", id);
        Ok(())
    }

    /// Gets a reference to a loaded plugin.
    pub fn get(&self, id: &str) -> Option<&WasmPlugin> {
        self.plugins.get(id)
    }

    /// Lists all loaded plugin IDs.
    pub fn list(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Returns the number of loaded plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Ticks all plugins that have an elapsed interval.
    /// Each plugin's `tick()` returns the next interval in ms (0 = stop ticking).
    pub fn tick_all(&mut self) {
        let ids: Vec<String> = self.plugins.keys().cloned().collect();

        for id in ids {
            if let Some(plugin) = self.plugins.get_mut(&id) {
                if plugin.tick_interval_ms == 0 {
                    continue;
                }

                let elapsed = plugin.last_tick.elapsed().as_millis() as u64;
                if elapsed < plugin.tick_interval_ms {
                    continue;
                }

                match plugin.bindings.call_tick(&mut plugin.store) {
                    Ok(next_interval) => {
                        plugin.tick_interval_ms = next_interval;
                        plugin.last_tick = Instant::now();
                    }
                    Err(e) => {
                        warn!("WASM plugin '{}' tick failed: {}", id, e);
                        plugin.tick_interval_ms = 0; // stop ticking on error
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_host_creation() {
        let host = WasmHost::new().expect("WasmHost should create successfully");
        assert_eq!(host.count(), 0);
        assert!(host.list().is_empty());
    }

    #[test]
    fn test_load_without_context() {
        let mut host = WasmHost::new().unwrap();
        let result = host.load(Path::new("nonexistent.wasm"));
        assert!(matches!(result, Err(WasmError::NoContext)));
    }

    #[test]
    fn test_unload_not_found() {
        let mut host = WasmHost::new().unwrap();
        let result = host.unload("nonexistent");
        assert!(matches!(result, Err(WasmError::NotFound { .. })));
    }
}
