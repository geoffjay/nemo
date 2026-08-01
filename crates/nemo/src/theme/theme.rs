use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::LazyLock;
use std::sync::RwLock;

use gpui::*;
use gpui_component::Theme;
use gpui_component::ThemeConfig;
use gpui_component::ThemeConfigColors;
use gpui_component::ThemeMode;
use gpui_component::ThemeSet;
use tracing::info;
use tracing::warn;

const THEME_SOURCES: &[&str] = &[
    include_str!("./catppuccin.json"),
    include_str!("./catppuccin-macchiato.json"),
    include_str!("./kanagawa.json"),
    include_str!("./kanagawa-dragon.json"),
    include_str!("./tokyo-night.json"),
    include_str!("./gruvbox.json"),
    include_str!("./nord.json"),
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

/// Project-defined theme sets registered from the loaded `app.xml`'s `<themes>`
/// block. Consulted **before** the baked-in `THEMES`/`THEME_SETS` statics, so a
/// project can add brand-new themes or fully replace a shipped one by reusing
/// its set name. Uses the same gpui-component `ThemeSet` schema as the bundled
/// JSON files.
static PROJECT_THEME_SETS: LazyLock<RwLock<Vec<ThemeSet>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Register project theme sets from external JSON files.
///
/// `srcs` are paths (relative to `base_dir`, or absolute) to files using the
/// same gpui-component `ThemeSet` schema as the bundled themes. The overlay is
/// **cleared first** so a project reload re-registers cleanly. Unlike the
/// baked-in themes (which `.unwrap()`), project input is untrusted: a missing or
/// malformed file is logged and skipped rather than panicking.
pub fn register_project_theme_sets(base_dir: &Path, srcs: &[String]) {
    let mut overlay = PROJECT_THEME_SETS
        .write()
        .expect("project theme sets lock poisoned");
    overlay.clear();

    for src in srcs {
        let path = if Path::new(src).is_absolute() {
            PathBuf::from(src)
        } else {
            base_dir.join(src)
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Failed to read project theme file {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        match serde_json::from_str::<ThemeSet>(&content) {
            Ok(set) => {
                info!(
                    "Registered project theme set '{}' ({} variant(s)) from {}",
                    set.name,
                    set.themes.len(),
                    path.display()
                );
                overlay.push(set);
            }
            Err(e) => warn!(
                "Failed to parse project theme file {}: {}",
                path.display(),
                e
            ),
        }
    }
}

/// Pair a set's variants into (light, dark), duplicating when only one mode
/// exists. Returns `None` for an empty variant list.
fn pair_from_variants(variants: &[ThemeConfig]) -> Option<(ThemeConfig, ThemeConfig)> {
    let light = variants.iter().find(|t| t.mode == ThemeMode::Light);
    let dark = variants.iter().find(|t| t.mode == ThemeMode::Dark);
    match (light, dark) {
        (Some(l), Some(d)) => Some((l.clone(), d.clone())),
        (Some(l), None) => Some((l.clone(), l.clone())),
        (None, Some(d)) => Some((d.clone(), d.clone())),
        (None, None) => variants.first().map(|f| (f.clone(), f.clone())),
    }
}

/// Resolve a light/dark pair from the project overlay by set name, then by exact
/// variant name (duplicated for both modes). `None` if no overlay match.
fn resolve_pair_from_overlay(name_lower: &str) -> Option<(ThemeConfig, ThemeConfig)> {
    let sets = PROJECT_THEME_SETS
        .read()
        .expect("project theme sets lock poisoned");

    for set in sets.iter() {
        if set.name.to_lowercase() == name_lower {
            if let Some(pair) = pair_from_variants(&set.themes) {
                return Some(pair);
            }
        }
    }

    for set in sets.iter() {
        for variant in &set.themes {
            if variant.name.to_lowercase() == name_lower {
                return Some((variant.clone(), variant.clone()));
            }
        }
    }

    None
}

/// Resolve a theme config by name and mode.
///
/// First tries exact variant name match in `THEMES` (case-insensitive),
/// then tries set name match in `THEME_SETS` (picks first variant matching requested mode).
/// The project overlay is consulted before either.
#[allow(dead_code)]
pub fn resolve_theme(name: &str, mode: ThemeMode) -> Option<ThemeConfig> {
    let name_lower = name.to_lowercase();

    // Project overlay wins over baked-in themes.
    if let Some((light, dark)) = resolve_pair_from_overlay(&name_lower) {
        return Some(if mode == ThemeMode::Dark { dark } else { light });
    }

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

    // Project overlay wins over baked-in themes.
    if let Some(pair) = resolve_pair_from_overlay(&name_lower) {
        return Some(pair);
    }

    // Try set name first
    if let Some(variants) = THEME_SETS.get(&name_lower) {
        if let Some(pair) = pair_from_variants(variants) {
            return Some(pair);
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

                info!(
                    "Applied theme '{}' in system mode (detected: {:?})",
                    name, os_mode
                );
            }
        }
        mode_str => {
            let mode = if mode_str == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };

            if let Some((mut light, mut dark)) = resolve_theme_pair(name) {
                if let Some(ov) = overrides {
                    light.colors = merge_theme_config_colors(&light.colors, ov);
                    dark.colors = merge_theme_config_colors(&dark.colors, ov);
                }

                let light = Rc::new(light);
                let dark = Rc::new(dark);

                let theme = Theme::global_mut(cx);
                // Apply both variants so mode toggling works
                theme.apply_config(&light);
                theme.apply_config(&dark);
                theme.mode = mode;
                // Re-apply the active variant's colors
                if mode == ThemeMode::Dark {
                    theme.apply_config(&dark);
                } else {
                    theme.apply_config(&light);
                }

                info!("Applied theme '{}' in {:?} mode", name, mode);
            }
        }
    }
}

/// Get a sorted list of all available theme names
#[allow(dead_code)]
pub fn get_theme_names() -> Vec<String> {
    let mut names: Vec<String> = THEMES.keys().map(|k| k.to_string()).collect();
    names.sort();
    names
}

/// Get a sorted, de-duplicated list of theme *set* display names.
///
/// Returns the original-cased set names (e.g. "Kanagawa", "Tokyo Night") suitable
/// for display in a selector. `apply_configured_theme` matches set names
/// case-insensitively, so these values can be passed to it directly.
pub fn get_theme_set_names() -> Vec<String> {
    let mut names: Vec<String> = THEME_SOURCES
        .iter()
        .filter_map(|source| serde_json::from_str::<ThemeSet>(source).ok())
        .map(|set| set.name.to_string())
        .collect();

    // Include project-defined theme sets so they appear in the settings picker.
    if let Ok(sets) = PROJECT_THEME_SETS.read() {
        names.extend(sets.iter().map(|set| set.name.to_string()));
    }

    names.sort();
    names.dedup();
    names
}

/// Apply a theme by exact variant name
#[allow(dead_code)]
pub fn apply_theme(name: &str, cx: &mut App) {
    if let Some(theme_config) = THEMES.get(name) {
        let theme_config = Rc::new(theme_config.clone());
        let theme = Theme::global_mut(cx);
        theme.mode = theme_config.mode;
        theme.apply_config(&theme_config);
    }
}

/// Apply the global `roundness` config to the live theme.
///
/// `value` is a named preset (`none`/`square`/`sharp`/`default`/`round`) or a
/// raw pixel base radius. It sets the gpui-component `Theme.radius` (the base
/// every widget and all nemo-drawn chrome scale from) and `radius_lg`
/// (proportionally, matching the `lg` token step). Unrecognized values are
/// ignored, leaving the theme default. Call *after* any theme application so it
/// is not overwritten — `apply_config` only touches radius when the theme JSON
/// declares one, which the shipped themes do not.
pub fn apply_roundness(value: &str, cx: &mut App) {
    let Some(base) = nemo_tokens::resolve_roundness(value) else {
        warn!("ignoring unrecognized roundness value: {value:?}");
        return;
    };
    let theme = Theme::global_mut(cx);
    theme.radius = px(base);
    // Scale the large radius proportionally to the base (lg == 8/6 of md).
    let lg = nemo_tokens::radius_scaled("lg", base).unwrap_or(base);
    theme.radius_lg = px(lg);
}

/// Toggle the color mode using the already-stored light/dark theme configs.
pub fn change_color_mode(mode: ThemeMode, _win: &mut Window, cx: &mut App) {
    let theme = Theme::global_mut(cx);
    let config = if mode == ThemeMode::Dark {
        theme.dark_theme.clone()
    } else {
        theme.light_theme.clone()
    };
    theme.mode = mode;
    theme.apply_config(&config);
}

#[cfg(test)]
mod tests {
    // NOTE: import specific items, not `use super::*` — that glob re-imports the
    // `gpui::*` prelude re-exported at the top of this module, whose sheer size
    // pushes the `#[test]` expansion past the crate's macro-recursion ceiling.
    use super::{
        get_theme_set_names, merge_theme_config_colors, register_project_theme_sets,
        resolve_theme_pair,
    };
    use gpui_component::{ThemeConfigColors, ThemeMode};

    const SAMPLE_SET: &str = r##"{
        "name": "Sample",
        "themes": [
            { "name": "Sample Dark", "mode": "dark", "colors": { "background": "#111111", "primary.background": "#222222" } },
            { "name": "Sample Light", "mode": "light", "colors": { "background": "#eeeeee" } }
        ]
    }"##;

    // A single test to avoid races on the global `PROJECT_THEME_SETS` overlay
    // (Cargo runs tests in the same binary concurrently).
    #[test]
    fn project_theme_overlay_lifecycle() {
        let dir = std::env::temp_dir().join(format!("nemo-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sample.json"), SAMPLE_SET).unwrap();

        register_project_theme_sets(&dir, &["sample.json".to_string()]);

        // Resolves by set name into a light/dark pair.
        let (light, dark) = resolve_theme_pair("sample").expect("overlay set should resolve");
        assert_eq!(light.mode, ThemeMode::Light);
        assert_eq!(dark.mode, ThemeMode::Dark);

        // Appears in the settings picker list.
        assert!(get_theme_set_names().iter().any(|n| n == "Sample"));

        // Overrides merge over the resolved base colors.
        let overrides: ThemeConfigColors =
            serde_json::from_str(r##"{ "primary.background": "#ff7a45" }"##).unwrap();
        let merged = merge_theme_config_colors(&dark.colors, &overrides);
        assert_eq!(merged.primary.as_deref(), Some("#ff7a45"));
        // A non-overridden color is preserved from the base.
        assert_eq!(merged.background.as_deref(), Some("#111111"));

        // A bad path is skipped gracefully (no panic) and clears the overlay.
        register_project_theme_sets(&dir, &["missing.json".to_string()]);
        assert!(resolve_theme_pair("sample").is_none());
        assert!(!get_theme_set_names().iter().any(|n| n == "Sample"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
