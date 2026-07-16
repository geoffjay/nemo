use serde::{Deserialize, Serialize};

/// Main application configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme_name: String,
    /// Color mode for the global theme: "dark", "light", or "system".
    pub theme_mode: Option<String>,
    pub font_family: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_name: "default".to_string(),
            theme_mode: None,
            font_family: None,
        }
    }
}
