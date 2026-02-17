//! Native plugin loading and management.
//!
//! This module handles loading, initializing, and managing native (cdylib)
//! plugins via `libloading`. Plugins expose two `extern "C"` symbols:
//!
//! - `nemo_plugin_manifest` — returns a [`PluginManifest`] describing the
//!   plugin's identity and capabilities.
//! - `nemo_plugin_entry` — called with a [`PluginRegistrar`] to register
//!   components, data sources, transforms, actions, and templates.
//!
//! # Safety
//!
//! All plugin loading is `unsafe` because dynamic library loading cannot
//! statically verify ABI compatibility. See [`PluginHost::load`] for the
//! full list of invariants that must hold.
//!
//! Currently, there is **no ABI version check, manifest validation, or
//! plugin signing**. The host trusts that the plugin binary was compiled
//! with a compatible toolchain and crate version.

use crate::error::ExtensionError;
use libloading::{Library, Symbol};
use nemo_plugin_api::{
    ActionSchema, ComponentSchema, DataSourceSchema, PluginContext, PluginManifest,
    PluginRegistrar, PluginValue, TransformSchema,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// A loaded native plugin.
pub struct LoadedPlugin {
    /// Plugin ID.
    pub id: String,
    /// Plugin manifest.
    pub manifest: PluginManifest,
    /// The loaded library (kept alive for symbol resolution).
    #[allow(dead_code)]
    library: Library,
}

impl LoadedPlugin {
    /// Returns the plugin ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the plugin manifest.
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

impl std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("id", &self.id)
            .field("manifest", &self.manifest)
            .finish_non_exhaustive()
    }
}

/// Host for managing native plugins.
pub struct PluginHost {
    /// Loaded plugins by ID.
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginHost {
    /// Creates a new plugin host.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Loads a native plugin from a dynamic library at the given path.
    ///
    /// # Safety Considerations
    ///
    /// Loading dynamic libraries is inherently unsafe. This function uses
    /// `libloading::Library::new` which calls `dlopen` (Unix) or
    /// `LoadLibrary` (Windows). The following invariants must hold:
    ///
    /// - **ABI compatibility**: The plugin must be compiled with the exact
    ///   same Rust compiler version and the same `nemo-plugin-api` crate
    ///   version as the host. Different versions may produce incompatible
    ///   type layouts (e.g. `PluginManifest`, `PluginRegistrar` vtable),
    ///   leading to undefined behaviour.
    /// - **Symbol correctness**: The library must export a
    ///   `nemo_plugin_manifest` symbol with signature
    ///   `extern "C" fn() -> PluginManifest`. If the symbol exists but has
    ///   a different signature, calling it is undefined behaviour.
    /// - **No constructor side-effects**: The library's load-time
    ///   constructors (`__attribute__((constructor))` or `DllMain`) must
    ///   not panic or perform operations that assume a specific runtime
    ///   state.
    /// - **Library lifetime**: The `Library` handle is stored in
    ///   `LoadedPlugin` and dropped on `unload()`. Any function pointers
    ///   or vtables obtained from the library become dangling after unload.
    ///
    /// ## What is NOT verified
    ///
    /// - ABI version compatibility (no version check between host and plugin)
    /// - Plugin code integrity (no signing or sandboxing)
    /// - Thread safety of plugin code
    ///
    /// ## Duplicate prevention
    ///
    /// If a plugin with the same `id` (from its manifest) is already loaded,
    /// this function returns `ExtensionError::AlreadyLoaded` without loading
    /// a second copy.
    pub fn load(&mut self, path: &Path) -> Result<String, ExtensionError> {
        // SAFETY: We trust that the plugin binary follows the expected Nemo
        // plugin ABI (same compiler version, same nemo-plugin-api version).
        // No ABI version checking is currently performed.
        unsafe {
            let library = Library::new(path).map_err(|e| ExtensionError::LoadError {
                id: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

            // Get the manifest function
            let manifest_fn: Symbol<unsafe extern "C" fn() -> PluginManifest> = library
                .get(b"nemo_plugin_manifest")
                .map_err(|e| ExtensionError::LoadError {
                    id: path.to_string_lossy().to_string(),
                    reason: format!("Missing nemo_plugin_manifest symbol: {}", e),
                })?;

            let manifest = manifest_fn();
            let id = manifest.id.clone();

            if self.plugins.contains_key(&id) {
                return Err(ExtensionError::AlreadyLoaded { id });
            }

            let plugin = LoadedPlugin {
                id: id.clone(),
                manifest,
                library,
            };

            self.plugins.insert(id.clone(), plugin);
            Ok(id)
        }
    }

    /// Unloads a plugin by ID.
    pub fn unload(&mut self, id: &str) -> Result<(), ExtensionError> {
        self.plugins
            .remove(id)
            .ok_or_else(|| ExtensionError::NotFound { id: id.to_string() })?;
        Ok(())
    }

    /// Gets a loaded plugin by ID.
    pub fn get(&self, id: &str) -> Option<&LoadedPlugin> {
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

    /// Checks if a plugin is loaded.
    pub fn is_loaded(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
}

/// Results collected from initializing a plugin via its entry point.
#[derive(Debug, Default)]
pub struct PluginInitResult {
    /// Templates registered by the plugin (name → PluginValue tree).
    pub templates: Vec<(String, PluginValue)>,
    /// Components registered by the plugin.
    pub components: Vec<(String, ComponentSchema)>,
    /// Data sources registered by the plugin.
    pub data_sources: Vec<(String, DataSourceSchema)>,
    /// Transforms registered by the plugin.
    pub transforms: Vec<(String, TransformSchema)>,
    /// Actions registered by the plugin.
    pub actions: Vec<(String, ActionSchema)>,
}

/// Concrete implementation of `PluginRegistrar` used during plugin initialization.
struct PluginRegistrarImpl {
    context: Arc<dyn PluginContext>,
    result: PluginInitResult,
}

impl PluginRegistrarImpl {
    fn new(context: Arc<dyn PluginContext>) -> Self {
        Self {
            context,
            result: PluginInitResult::default(),
        }
    }
}

impl PluginRegistrar for PluginRegistrarImpl {
    fn register_component(&mut self, name: &str, schema: ComponentSchema) {
        self.result.components.push((name.to_string(), schema));
    }

    fn register_data_source(&mut self, name: &str, schema: DataSourceSchema) {
        self.result.data_sources.push((name.to_string(), schema));
    }

    fn register_transform(&mut self, name: &str, schema: TransformSchema) {
        self.result.transforms.push((name.to_string(), schema));
    }

    fn register_action(&mut self, name: &str, schema: ActionSchema) {
        self.result.actions.push((name.to_string(), schema));
    }

    fn register_template(&mut self, name: &str, template: PluginValue) {
        self.result.templates.push((name.to_string(), template));
    }

    fn context(&self) -> &dyn PluginContext {
        self.context.as_ref()
    }

    fn context_arc(&self) -> Arc<dyn PluginContext> {
        Arc::clone(&self.context)
    }
}

impl PluginHost {
    /// Initializes a loaded plugin by calling its `nemo_plugin_entry` symbol.
    ///
    /// The plugin's entry function receives a [`PluginRegistrar`] that collects
    /// all registrations (components, data sources, transforms, actions,
    /// templates) into a [`PluginInitResult`].
    ///
    /// # Safety Considerations
    ///
    /// Same ABI invariants as [`load()`](Self::load). Additionally:
    ///
    /// - The entry function must not panic (undefined behaviour across FFI).
    /// - The entry function must not store the `PluginRegistrar` reference
    ///   beyond its own scope.
    /// - The `context` Arc is provided to the plugin and may be retained for
    ///   the lifetime of the library. It must remain valid until `unload()`.
    pub fn init_plugin(
        &self,
        id: &str,
        context: Arc<dyn PluginContext>,
    ) -> Result<PluginInitResult, ExtensionError> {
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| ExtensionError::NotFound { id: id.to_string() })?;

        // SAFETY: Same ABI trust as load(). The entry function must not panic
        // or store the registrar reference beyond this call.
        unsafe {
            let entry_fn: Symbol<nemo_plugin_api::PluginEntryFn> = plugin
                .library
                .get(b"nemo_plugin_entry")
                .map_err(|e| ExtensionError::PluginInitError {
                    plugin_id: id.to_string(),
                    reason: format!("Missing nemo_plugin_entry symbol: {}", e),
                })?;

            let mut registrar = PluginRegistrarImpl::new(context);
            entry_fn(&mut registrar);

            Ok(registrar.result)
        }
    }

    /// Initializes all loaded plugins. Returns a vec of (plugin_id, result).
    pub fn init_all_plugins(
        &self,
        context: Arc<dyn PluginContext>,
    ) -> Vec<(String, Result<PluginInitResult, ExtensionError>)> {
        let ids: Vec<String> = self.plugins.keys().cloned().collect();
        ids.into_iter()
            .map(|id| {
                let result = self.init_plugin(&id, Arc::clone(&context));
                (id, result)
            })
            .collect()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_host_creation() {
        let host = PluginHost::new();
        assert_eq!(host.count(), 0);
        assert!(host.list().is_empty());
    }

    #[test]
    fn test_plugin_not_found() {
        let mut host = PluginHost::new();
        let result = host.unload("nonexistent");
        assert!(matches!(result, Err(ExtensionError::NotFound { .. })));
    }
}
