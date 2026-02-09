use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use gpui::*;
use gpui_component::Theme;
use gpui_component::ThemeConfig;
use gpui_component::ThemeConfigColors;
use gpui_component::ThemeMode;
use gpui_component::ThemeSet;
use tracing::info;

const THEME_SOURCES: &[&str] = &[
    include_str!("./catppuccin.json"),
    include_str!("./kanagawa.json"),
    include_str!("./tokyo-night.json"),
    include_str!("./gruvbox.json"),
];

/// All individual theme variants keyed by exact variant name (e.g. "Kanagawa Wave").
pub static THEMES: LazyLock<HashMap<SharedString, ThemeConfig>> = LazyLock::new(|| {
    let mut themes = HashMap::new();
    for source in THEME_SOURCES {
        let theme_set: ThemeSet = serde_json::from_str(source).unwrap();
        for theme in theme_set.themes {
            themes.insert(theme.name.clone(), theme);
        }
    }
    themes
});

/// Theme variants grouped by set name (lowercased), e.g. "kanagawa" -> [Wave, Lotus, Dragon].
pub static THEME_SETS: LazyLock<HashMap<String, Vec<ThemeConfig>>> = LazyLock::new(|| {
    let mut sets: HashMap<String, Vec<ThemeConfig>> = HashMap::new();
    for source in THEME_SOURCES {
        let theme_set: ThemeSet = serde_json::from_str(source).unwrap();
        let set_name = theme_set.name.to_lowercase();
        let entry = sets.entry(set_name).or_default();
        for theme in theme_set.themes {
            entry.push(theme);
        }
    }
    sets
});

/// Resolve a theme config by name and mode.
///
/// First tries exact variant name match in `THEMES` (case-insensitive),
/// then tries set name match in `THEME_SETS` (picks first variant matching requested mode).
pub fn resolve_theme(name: &str, mode: ThemeMode) -> Option<ThemeConfig> {
    let name_lower = name.to_lowercase();

    // Try exact variant name match (case-insensitive)
    for (key, config) in THEMES.iter() {
        if key.to_lowercase() == name_lower {
            return Some(config.clone());
        }
    }

    // Try set name match — pick the variant matching the requested mode
    if let Some(variants) = THEME_SETS.get(&name_lower) {
        // First try to find a variant matching the requested mode
        if let Some(config) = variants.iter().find(|t| t.mode == mode) {
            return Some(config.clone());
        }
        // Fall back to first available variant
        return variants.first().cloned();
    }

    None
}

/// Resolve a light/dark theme pair for system mode.
///
/// Returns (light_variant, dark_variant). If the set only has one mode,
/// duplicates it for both.
pub fn resolve_theme_pair(name: &str) -> Option<(ThemeConfig, ThemeConfig)> {
    let name_lower = name.to_lowercase();

    // Try set name first
    if let Some(variants) = THEME_SETS.get(&name_lower) {
        let light = variants.iter().find(|t| t.mode == ThemeMode::Light);
        let dark = variants.iter().find(|t| t.mode == ThemeMode::Dark);

        match (light, dark) {
            (Some(l), Some(d)) => return Some((l.clone(), d.clone())),
            (Some(l), None) => return Some((l.clone(), l.clone())),
            (None, Some(d)) => return Some((d.clone(), d.clone())),
            (None, None) => {
                if let Some(first) = variants.first() {
                    return Some((first.clone(), first.clone()));
                }
            }
        }
    }

    // Try exact variant name — duplicate for both modes
    for (key, config) in THEMES.iter() {
        if key.to_lowercase() == name_lower {
            return Some((config.clone(), config.clone()));
        }
    }

    None
}

/// Merge override colors into a base ThemeConfigColors using JSON serialization.
///
/// Only non-null keys from `overrides` replace values in `base`.
pub fn merge_theme_config_colors(
    base: &ThemeConfigColors,
    overrides: &ThemeConfigColors,
) -> ThemeConfigColors {
    let mut base_json = serde_json::to_value(base).unwrap();
    let overrides_json = serde_json::to_value(overrides).unwrap();

    if let (Some(base_obj), Some(overrides_obj)) =
        (base_json.as_object_mut(), overrides_json.as_object())
    {
        for (key, value) in overrides_obj {
            if !value.is_null() {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }

    serde_json::from_value(base_json).unwrap()
}

/// Main entry point: resolve and apply a named theme with optional mode and color overrides.
///
/// - `name`: Theme name (set name like "kanagawa" or exact variant like "Kanagawa Wave")
/// - `mode_str`: "light", "dark", or "system"
/// - `overrides`: Optional color overrides to merge into the resolved theme
/// - `cx`: GPUI App context
pub fn apply_configured_theme(
    name: &str,
    mode_str: &str,
    overrides: Option<&ThemeConfigColors>,
    cx: &mut App,
) {
    match mode_str {
        "system" => {
            if let Some((mut light, mut dark)) = resolve_theme_pair(name) {
                if let Some(ov) = overrides {
                    light.colors = merge_theme_config_colors(&light.colors, ov);
                    dark.colors = merge_theme_config_colors(&dark.colors, ov);
                }

                let light = Rc::new(light);
                let dark = Rc::new(dark);

                // Detect OS appearance before taking mutable borrow on Theme
                let os_mode = match cx.window_appearance() {
                    WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
                    _ => ThemeMode::Light,
                };

                let theme = Theme::global_mut(cx);
                // Apply both variants so system mode switching works
                theme.apply_config(&light);
                theme.apply_config(&dark);
                theme.mode = os_mode;
                // Re-apply the active variant's colors
                if os_mode == ThemeMode::Dark {
                    theme.apply_config(&dark);
                } else {
                    theme.apply_config(&light);
                }

                info!("Applied theme '{}' in system mode (detected: {:?})", name, os_mode);
            }
        }
        mode_str => {
            let mode = if mode_str == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };

            if let Some(mut config) = resolve_theme(name, mode) {
                if let Some(ov) = overrides {
                    config.colors = merge_theme_config_colors(&config.colors, ov);
                }

                let config = Rc::new(config);
                let theme = Theme::global_mut(cx);
                theme.mode = mode;
                theme.apply_config(&config);

                info!("Applied theme '{}' in {:?} mode", name, mode);
            }
        }
    }
}

/// Get a sorted list of all available theme names
pub fn get_theme_names() -> Vec<String> {
    let mut names: Vec<String> = THEMES.keys().map(|k| k.to_string()).collect();
    names.sort();
    names
}

/// Apply a theme by exact variant name
pub fn apply_theme(name: &str, cx: &mut App) {
    if let Some(theme_config) = THEMES.get(name) {
        let theme_config = Rc::new(theme_config.clone());
        let theme = Theme::global_mut(cx);
        theme.mode = theme_config.mode;
        theme.apply_config(&theme_config);
    }
}

/// Apply a theme by color mode
pub fn change_color_mode(mode: ThemeMode, _win: &mut Window, cx: &mut App) {
    let theme_name = match mode {
        ThemeMode::Light => "Kanagawa Lotus",
        ThemeMode::Dark => "Kanagawa Wave",
    };

    apply_theme(theme_name, cx);
}
