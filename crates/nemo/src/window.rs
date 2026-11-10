use gpui::*;
use gpui_kit::component::TitleBar;

/// Effective window appearance for a launch, derived from the config's
/// `app.window.mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowMode {
    /// Normal windowed/maximized application with client-drawn titlebar.
    #[default]
    Normal,
    /// Fullscreen: fills the display, no OS window decorations. The app's
    /// own header/footer chrome is kept.
    Fullscreen,
    /// Kiosk: fullscreen plus the app's header/footer chrome suppressed.
    Kiosk,
}

impl WindowMode {
    /// Whether the mode opens the window fullscreen (fills the display, no
    /// OS decorations). Both [`WindowMode::Fullscreen`] and
    /// [`WindowMode::Kiosk`] do.
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, Self::Fullscreen | Self::Kiosk)
    }

    /// Whether the mode suppresses the app's own header/footer chrome.
    /// Only [`WindowMode::Kiosk`] does — fullscreen keeps them.
    pub fn suppresses_chrome(&self) -> bool {
        matches!(self, Self::Kiosk)
    }

    /// Resolves `app.window.mode` (already snake-keyed) into a mode. Any
    /// unrecognized value falls back to `Normal` so typos never brick a launch.
    pub fn from_config_value(value: Option<&str>) -> Self {
        match value.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("fullscreen") => Self::Fullscreen,
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

/// Window options for a launch. `mode` selects the initial presentation.
///
/// Fullscreen modes open *windowed* (centered, at the configured size) and the
/// actual fullscreen toggle happens after the window exists — see
/// `build_app_window`. Passing `WindowBounds::Fullscreen` at open is a race on
/// macOS: gpui dispatches `toggleFullScreen:` before AppKit has the window on
/// screen, and the request is silently dropped (the window stays windowed).
///
/// For the same reason `titlebar` is `None` in the fullscreen modes: with a
/// titlebar present, gpui's macOS backend always applies the
/// `Closable|Miniaturizable` style mask, which keeps the traffic-light buttons
/// visible. Without one, the window opens `Titled|FullSizeContentView` with a
/// transparent titlebar and no buttons.
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

    let fullscreen = mode.is_fullscreen();

    // Fullscreen modes open windowed (see the doc comment above); the toggle
    // is applied post-open, where the configured size is the restore size.
    let window_bounds = if fullscreen || width.is_some() || height.is_some() {
        Some(WindowBounds::Windowed(restored_bounds))
    } else {
        Some(WindowBounds::Maximized(restored_bounds))
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
        titlebar: (!fullscreen).then(TitleBar::title_bar_options),
        window_decorations: Some(WindowDecorations::Client),
        // Resizable must stay true even in the fullscreen modes: AppKit's
        // `toggleFullScreen:` is unreliable for windows without the
        // `Resizable` style mask, and a presented fullscreen window can't be
        // user-resized anyway.
        is_resizable: true,
        is_minimizable: !fullscreen,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    // Import specific items (not `use super::*`) — a glob import in a nemo-bin
    // test module blows the macro-recursion limit.
    use super::WindowMode;

    #[test]
    fn window_mode_parses_case_insensitively() {
        assert_eq!(
            WindowMode::from_config_value(Some("kiosk")),
            WindowMode::Kiosk
        );
        assert_eq!(
            WindowMode::from_config_value(Some("KIOSK")),
            WindowMode::Kiosk
        );
        assert_eq!(
            WindowMode::from_config_value(Some("fullscreen")),
            WindowMode::Fullscreen
        );
        assert_eq!(
            WindowMode::from_config_value(Some("FullScreen")),
            WindowMode::Fullscreen
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

    #[test]
    fn window_mode_classifies_fullscreen_and_chrome() {
        assert!(WindowMode::Kiosk.is_fullscreen());
        assert!(WindowMode::Fullscreen.is_fullscreen());
        assert!(!WindowMode::Normal.is_fullscreen());

        assert!(WindowMode::Kiosk.suppresses_chrome());
        assert!(!WindowMode::Fullscreen.suppresses_chrome());
        assert!(!WindowMode::Normal.suppresses_chrome());
    }
}
