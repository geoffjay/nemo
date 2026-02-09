use gpui::*;
use gpui_component::TitleBar;

pub fn get_window_options(cx: &mut App, width: Option<u32>, height: Option<u32>) -> WindowOptions {
    let effective_width = width.unwrap_or(1200) as f32;
    let effective_height = height.unwrap_or(800) as f32;
    let restored_size = size(px(effective_width), px(effective_height));
    let restored_bounds = Bounds::centered(None, restored_size, cx);

    let window_bounds = if width.is_some() || height.is_some() {
        Some(WindowBounds::Windowed(restored_bounds))
    } else {
        Some(WindowBounds::Maximized(restored_bounds))
    };

    WindowOptions {
        window_bounds,
        titlebar: Some(TitleBar::title_bar_options()),
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}
