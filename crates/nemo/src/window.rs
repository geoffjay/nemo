use gpui::*;
use gpui_component::TitleBar;

pub fn get_window_options(
    cx: &mut App,
    _title: String,
    _width: u32,
    _height: u32,
) -> WindowOptions {
    // Default window size when restored from maximized state
    let restored_size = size(px(1200.), px(800.));
    let restored_bounds = Bounds::centered(None, restored_size, cx);

    // TODO: read width and height from config and use those if they're provided,
    // otherwise use maxmimized bounds
    //
    // Create the main window
    // let window_options = WindowOptions {
    //     window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
    //         None,
    //         size(px(width as f32), px(height as f32)),
    //         cx,
    //     ))),
    //     ..Default::default()
    // };

    WindowOptions {
        window_bounds: Some(WindowBounds::Maximized(restored_bounds)),
        titlebar: Some(TitleBar::title_bar_options()),
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}
