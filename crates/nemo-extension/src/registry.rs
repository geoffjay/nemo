//! Extension registry for tracking loaded extensions.

use std::collections::HashMap;
use std::path::PathBuf;

/// Entry for a loaded script.
#[derive(Debug, Clone)]
pub struct ScriptEntry {
    /// Script ID.
    pub id: String,
    /// Path to the script file.
    pub path: PathBuf,
    /// Whether the script is enabled.
    pub enabled: bool,
}

/// Entry for a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    /// Plugin ID.
    pub id: String,
    /// Path to the plugin file.
    pub path: PathBuf,
    /// Whether the plugin is enabled.
    pub enabled: bool,
}

/// Registry tracking all loaded extensions.
pub struct ExtensionRegistry {
    /// Loaded scripts by ID.
    scripts: HashMap<String, ScriptEntry>,
    /// Loaded plugins by ID.
    plugins: HashMap<String, PluginEntry>,
}

impl ExtensionRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    /// Registers a script.
    pub fn register_script(&mut self, id: String, path: PathBuf) {
        let entry = ScriptEntry {
            id: id.clone(),
            path,
            enabled: true,
        };
        self.scripts.insert(id, entry);
    }

    /// Unregisters a script.
    pub fn unregister_script(&mut self, id: &str) -> Option<ScriptEntry> {
        self.scripts.remove(id)
    }

    /// Gets a script entry.
    pub fn get_script(&self, id: &str) -> Option<&ScriptEntry> {
        self.scripts.get(id)
    }

    /// Gets the path for a script.
    pub fn get_script_path(&self, id: &str) -> Option<PathBuf> {
        self.scripts.get(id).map(|e| e.path.clone())
    }

    /// Lists all script IDs.
    pub fn list_scripts(&self) -> Vec<String> {
        self.scripts.keys().cloned().collect()
    }

    /// Enables or disables a script.
    pub fn set_script_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(entry) = self.scripts.get_mut(id) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Registers a plugin.
    pub fn register_plugin(&mut self, id: String, path: PathBuf) {
        let entry = PluginEntry {
            id: id.clone(),
            path,
            enabled: true,
        };
        self.plugins.insert(id, entry);
    }

    /// Unregisters a plugin.
    pub fn unregister_plugin(&mut self, id: &str) -> Option<PluginEntry> {
        self.plugins.remove(id)
    }

    /// Gets a plugin entry.
    pub fn get_plugin(&self, id: &str) -> Option<&PluginEntry> {
        self.plugins.get(id)
    }

    /// Gets the path for a plugin.
    pub fn get_plugin_path(&self, id: &str) -> Option<PathBuf> {
        self.plugins.get(id).map(|e| e.path.clone())
    }

    /// Lists all plugin IDs.
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Enables or disables a plugin.
    pub fn set_plugin_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(entry) = self.plugins.get_mut(id) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Returns total count of extensions (scripts + plugins).
    pub fn total_count(&self) -> usize {
        self.scripts.len() + self.plugins.len()
    }

    /// Returns count of scripts.
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    /// Returns count of plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Clears all registered extensions.
    pub fn clear(&mut self) {
        self.scripts.clear();
        self.plugins.clear();
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ExtensionRegistry::new();
        assert_eq!(registry.total_count(), 0);
    }

    #[test]
    fn test_register_script() {
        let mut registry = ExtensionRegistry::new();
        registry.register_script("test".to_string(), PathBuf::from("test.rhai"));

        assert_eq!(registry.script_count(), 1);
        assert!(registry.get_script("test").is_some());
        assert!(registry.list_scripts().contains(&"test".to_string()));
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = ExtensionRegistry::new();
        registry.register_plugin("test".to_string(), PathBuf::from("libtest.so"));

        assert_eq!(registry.plugin_count(), 1);
        assert!(registry.get_plugin("test").is_some());
        assert!(registry.list_plugins().contains(&"test".to_string()));
    }

    #[test]
    fn test_unregister() {
        let mut registry = ExtensionRegistry::new();
        registry.register_script("test".to_string(), PathBuf::from("test.rhai"));

        let removed = registry.unregister_script("test");
        assert!(removed.is_some());
        assert_eq!(registry.script_count(), 0);
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = ExtensionRegistry::new();
        registry.register_script("test".to_string(), PathBuf::from("test.rhai"));

        assert!(registry.get_script("test").unwrap().enabled);

        registry.set_script_enabled("test", false);
        assert!(!registry.get_script("test").unwrap().enabled);
    }
}
