use serde::{Deserialize, Serialize};

/// Main application configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_name: "default".to_string(),
        }
    }
}
