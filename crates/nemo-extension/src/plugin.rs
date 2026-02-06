//! Native plugin loading and management.

use crate::error::ExtensionError;
use libloading::{Library, Symbol};
use nemo_plugin_api::PluginManifest;
use std::collections::HashMap;
use std::path::Path;

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

    /// Loads a plugin from a path.
    pub fn load(&mut self, path: &Path) -> Result<String, ExtensionError> {
        // Safety: Loading dynamic libraries is inherently unsafe.
        // We trust that the plugin follows the expected ABI.
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
