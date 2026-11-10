use gpui::*;
use gpui_kit::component::TitleBar;

/// Effective window appearance for a launch, derived from the config's
/// `app.window.mode` (currently only `"kiosk"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowMode {
    /// Normal windowed/maximized application with client-drawn titlebar.
    #[default]
    Normal,
    /// Kiosk: fullscreen bounds, no resizing, header/footer suppressed.
    Kiosk,
}

impl WindowMode {
    /// Resolves `app.window.mode` (already snake-keyed) into a mode. Any
    /// unrecognized value falls back to `Normal` so typos never brick a launch.
    pub fn from_config_value(value: Option<&str>) -> Self {
        match value.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("kiosk") => Self::Kiosk,
            _ => Self::default(),
        }
    }

    /// [`Self::from_config_value`] over an owned runtime config value
    /// (`get_config` returns `Option<Value>`, not a borrowed string).
    pub fn from_config(value: Option<nemo_config::Value>) -> Self {
        Self::from_config_value(value.as_ref().and_then(|v| v.as_str()))
    }
}

pub fn get_window_options(
    cx: &mut App,
    width: Option<u32>,
    height: Option<u32>,
    min_width: Option<u32>,
    min_height: Option<u32>,
    mode: WindowMode,
) -> WindowOptions {
    let effective_width = width.unwrap_or(1200) as f32;
    let effective_height = height.unwrap_or(800) as f32;
    let restored_size = size(px(effective_width), px(effective_height));
    let restored_bounds = Bounds::centered(None, restored_size, cx);

    let window_bounds = match mode {
        // Kiosk opens fullscreen; the configured size becomes the restore size.
        WindowMode::Kiosk => Some(WindowBounds::Fullscreen(restored_bounds)),
        WindowMode::Normal if width.is_some() || height.is_some() => {
            Some(WindowBounds::Windowed(restored_bounds))
        }
        WindowMode::Normal => Some(WindowBounds::Maximized(restored_bounds)),
    };

    let window_min_size = if min_width.is_some() || min_height.is_some() {
        Some(size(
            px(min_width.unwrap_or(0) as f32),
            px(min_height.unwrap_or(0) as f32),
        ))
    } else {
        None
    };

    WindowOptions {
        window_bounds,
        window_min_size,
        titlebar: Some(TitleBar::title_bar_options()),
        window_decorations: Some(WindowDecorations::Client),
        is_resizable: mode == WindowMode::Normal,
        is_minimizable: mode == WindowMode::Normal,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    // Import specific items (not `use super::*`) — a glob import in a nemo-bin
    // test module blows the macro-recursion limit.
    use super::WindowMode;

    #[test]
    fn window_mode_parses_kiosk_case_insensitively() {
        assert_eq!(
            WindowMode::from_config_value(Some("kiosk")),
            WindowMode::Kiosk
        );
        assert_eq!(
            WindowMode::from_config_value(Some("KIOSK")),
            WindowMode::Kiosk
        );
        assert_eq!(
            WindowMode::from_config_value(Some("normal")),
            WindowMode::Normal
        );
        assert_eq!(
            WindowMode::from_config_value(Some("nonsense")),
            WindowMode::Normal
        );
        assert_eq!(WindowMode::from_config_value(None), WindowMode::Normal);
    }
}
