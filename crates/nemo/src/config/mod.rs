use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod app;
pub mod recent;

use app::AppConfig;

/// Main application configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NemoConfig {
    pub project_dir: PathBuf,

    /// Application settings
    pub app: AppConfig,
}

impl Default for NemoConfig {
    fn default() -> Self {
        Self {
            project_dir: dirs::home_dir().unwrap_or_default(),
            app: AppConfig::default(),
        }
    }
}

impl NemoConfig {
    /// Load configuration from an explicit path, or fall back to the default location.
    /// If the file doesn't exist at either location, returns defaults.
    pub fn load_from(explicit_path: Option<&PathBuf>) -> Self {
        let path = explicit_path.cloned().or_else(Self::config_path);

        path.and_then(|p| {
            if p.exists() {
                std::fs::read_to_string(&p).ok()
            } else {
                None
            }
        })
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
    }

    /// Load configuration from the default config file location.
    pub fn load() -> Self {
        Self::load_from(None)
    }

    /// Save the configuration to the default config file location
    pub fn save(&self) -> Result<(), std::io::Error> {
        let Some(config_path) = Self::config_path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            ));
        };

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&config_path, toml_str)
    }

    /// Get the default config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("nemo").join("config.toml"))
    }

    /// Load configuration from a specific TOML string (for testing)
    #[cfg(test)]
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load configuration from a specific file path (for testing)
    #[cfg(test)]
    pub fn load_from_path(path: &PathBuf) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| toml::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save configuration to a specific file path (for testing)
    #[cfg(test)]
    pub fn save_to_path(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(path, toml_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    mod default_tests {
        use super::*;

        #[test]
        fn test_default_values() {
            let config = NemoConfig::default();

            // root defaults
            assert_eq!(
                config.project_dir,
                dirs::home_dir().unwrap()
            );
        }
    }

    mod serialization_tests {
        use super::*;

        #[test]
        fn test_serialization_roundtrip() {
            let config = NemoConfig {
                project_dir: "/home/user/projects".into(),
                ..Default::default()
            };

            let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
            let parsed: NemoConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

            assert_eq!(parsed.project_dir, PathBuf::from("/home/user/projects"));
        }

        #[test]
        fn test_deserialize_full_config() {
            let toml_str = r#"
                project_dir = "/home/user/projects"
            "#;

            let config = NemoConfig::from_toml(toml_str).expect("Failed to deserialize");

            assert_eq!(config.project_dir, PathBuf::from("/home/user/projects"));
        }

        #[test]
        fn test_deserialize_partial_config_uses_defaults() {
            let toml_str = r#"
            "#;

            let config = NemoConfig::from_toml(toml_str).expect("Failed to deserialize");

            // root should be default
            assert_eq!(config.project_dir, dirs::home_dir().unwrap());
        }

        #[test]
        fn test_deserialize_empty_uses_all_defaults() {
            let toml_str = "";
            let config = NemoConfig::from_toml(toml_str).expect("Failed to deserialize");

            assert_eq!(
                config.project_dir,
                dirs::home_dir().unwrap()
            );
        }
    }

    mod file_io_tests {
        use super::*;

        #[test]
        fn test_save_and_load_from_path() {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let config_path = temp_dir.path().join("config.toml");

            let config = NemoConfig {
                project_dir: temp_dir.path().to_path_buf(),
                ..Default::default()
            };

            config
                .save_to_path(&config_path)
                .expect("Failed to save config");

            let loaded = NemoConfig::load_from_path(&config_path);

            assert_eq!(
                loaded.project_dir,
                PathBuf::from(temp_dir.path().to_str().unwrap())
            );
        }

        #[test]
        fn test_load_from_nonexistent_path_returns_defaults() {
            let config_path = PathBuf::from("/nonexistent/path/config.toml");
            let config = NemoConfig::load_from_path(&config_path);

            assert_eq!(
                config.project_dir,
                dirs::home_dir().unwrap()
            );
        }

        #[test]
        fn test_save_creates_parent_directories() {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let config_path = temp_dir
                .path()
                .join("nested")
                .join("dir")
                .join("config.toml");

            let config = NemoConfig::default();
            config
                .save_to_path(&config_path)
                .expect("Failed to save config");

            assert!(config_path.exists());
        }
    }

    mod toml_format_tests {
        use super::*;

        #[test]
        fn test_generated_toml_has_expected_sections() {
            let config = NemoConfig::default();
            let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

            assert!(toml_str.contains("[app]"));
        }

        #[test]
        fn test_generated_toml_has_expected_keys() {
            let config = NemoConfig::default();
            let _toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        }
    }
}
