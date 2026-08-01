use serde::{Deserialize, Serialize};

/// Main application configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme_name: String,
    /// Color mode for the global theme: "dark", "light", or "system".
    pub theme_mode: Option<String>,
    pub font_family: Option<String>,
    /// Global corner-roundness for the whole UI. A named preset
    /// (`none`/`square`/`sharp`/`default`/`round`) or a raw pixel base radius
    /// (e.g. `"3"`). Sets the gpui-component `Theme.radius`, which every widget
    /// and all nemo-drawn chrome scale from. `None` keeps the theme default.
    pub roundness: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_name: "default".to_string(),
            theme_mode: None,
            font_family: None,
            roundness: None,
        }
    }
}
