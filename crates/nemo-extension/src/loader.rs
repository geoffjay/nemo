//! Extension discovery and loading.

use crate::error::ExtensionError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Type of extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionType {
    /// RHAI script extension.
    Script,
    /// Native plugin extension.
    Plugin,
}

/// Extension manifest describing an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Description.
    pub description: Option<String>,
    /// Extension type.
    pub extension_type: ExtensionType,
    /// Path to the extension file.
    pub path: PathBuf,
    /// Entry point (for scripts, the main function; for plugins, the symbol).
    pub entry_point: Option<String>,
    /// Dependencies on other extensions.
    pub dependencies: Vec<String>,
}

impl ExtensionManifest {
    /// Creates a new script manifest.
    pub fn script(id: impl Into<String>, name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: "0.1.0".to_string(),
            description: None,
            extension_type: ExtensionType::Script,
            path,
            entry_point: None,
            dependencies: Vec::new(),
        }
    }

    /// Creates a new plugin manifest.
    pub fn plugin(id: impl Into<String>, name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: "0.1.0".to_string(),
            description: None,
            extension_type: ExtensionType::Plugin,
            path,
            entry_point: None,
            dependencies: Vec::new(),
        }
    }
}

/// Discovers and loads extensions from configured paths.
pub struct ExtensionLoader {
    /// Script search paths.
    script_paths: Vec<PathBuf>,
    /// Plugin search paths.
    plugin_paths: Vec<PathBuf>,
}

impl ExtensionLoader {
    /// Creates a new extension loader.
    pub fn new() -> Self {
        Self {
            script_paths: Vec::new(),
            plugin_paths: Vec::new(),
        }
    }

    /// Adds a script search path.
    pub fn add_script_path(&mut self, path: impl Into<PathBuf>) {
        self.script_paths.push(path.into());
    }

    /// Adds a plugin search path.
    pub fn add_plugin_path(&mut self, path: impl Into<PathBuf>) {
        self.plugin_paths.push(path.into());
    }

    /// Discovers all extensions in configured paths.
    pub fn discover(&self) -> Result<Vec<ExtensionManifest>, ExtensionError> {
        let mut manifests = Vec::new();

        // Discover scripts
        for script_path in &self.script_paths {
            if script_path.is_dir() {
                manifests.extend(self.discover_scripts(script_path)?);
            }
        }

        // Discover plugins
        for plugin_path in &self.plugin_paths {
            if plugin_path.is_dir() {
                manifests.extend(self.discover_plugins(plugin_path)?);
            }
        }

        Ok(manifests)
    }

    /// Discovers scripts in a directory.
    fn discover_scripts(&self, dir: &Path) -> Result<Vec<ExtensionManifest>, ExtensionError> {
        let mut manifests = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "rhai") {
                let id = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                let name = id
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_uppercase().next().unwrap_or(c)
                        } else if c == '_' || c == '-' {
                            ' '
                        } else {
                            c
                        }
                    })
                    .collect::<String>();

                manifests.push(ExtensionManifest::script(&id, name, path));
            }
        }

        Ok(manifests)
    }

    /// Discovers plugins in a directory.
    fn discover_plugins(&self, dir: &Path) -> Result<Vec<ExtensionManifest>, ExtensionError> {
        let mut manifests = Vec::new();

        #[cfg(target_os = "macos")]
        let extension = "dylib";
        #[cfg(target_os = "linux")]
        let extension = "so";
        #[cfg(target_os = "windows")]
        let extension = "dll";

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == extension) {
                let id = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                // Strip lib prefix on Unix
                #[cfg(not(target_os = "windows"))]
                let id = id.strip_prefix("lib").unwrap_or(&id).to_string();

                let name = id
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_uppercase().next().unwrap_or(c)
                        } else if c == '_' || c == '-' {
                            ' '
                        } else {
                            c
                        }
                    })
                    .collect::<String>();

                manifests.push(ExtensionManifest::plugin(&id, name, path));
            }
        }

        Ok(manifests)
    }

    /// Loads a manifest from a JSON file.
    pub fn load_manifest(&self, path: &Path) -> Result<ExtensionManifest, ExtensionError> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| ExtensionError::InvalidManifest {
            id: path.to_string_lossy().to_string(),
            reason: e.to_string(),
        })
    }
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = ExtensionLoader::new();
        assert!(loader.script_paths.is_empty());
        assert!(loader.plugin_paths.is_empty());
    }

    #[test]
    fn test_script_manifest() {
        let manifest = ExtensionManifest::script("test", "Test Script", PathBuf::from("test.rhai"));
        assert_eq!(manifest.id, "test");
        assert_eq!(manifest.extension_type, ExtensionType::Script);
    }

    #[test]
    fn test_plugin_manifest() {
        let manifest = ExtensionManifest::plugin("test", "Test Plugin", PathBuf::from("libtest.so"));
        assert_eq!(manifest.id, "test");
        assert_eq!(manifest.extension_type, ExtensionType::Plugin);
    }
}
