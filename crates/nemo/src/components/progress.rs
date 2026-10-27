use gpui::*;
use gpui_kit::component::progress::Progress as GpuiProgress;
use nemo_layout::BuiltComponent;
use nemo_macros::NemoComponent;

/// A progress bar component.
///
/// # XML Configuration
///
/// ```xml
/// <progress id="upload" value="65" max="100" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `value` | float | Current progress value (0-100) |
/// | `max` | float | Maximum value (default: 100) |
#[derive(IntoElement, NemoComponent)]
pub struct Progress {
    #[property]
    value: Option<f64>,
    #[property(default = 100.0)]
    max: f64,
    #[source]
    source: BuiltComponent,
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let value = self.value.unwrap_or(0.0);
        let percentage = if self.max > 0.0 {
            (value / self.max * 100.0) as f32
        } else {
            0.0
        };
        // Unique element id per instance (XML `id`) — required for gpui state
        // (the animated value) and a11y node ids, which hash the element path.
        GpuiProgress::new(SharedString::from(self.source.id.clone()))
            .value(percentage)
            .accessibility_label(SharedString::from(format!("{}%", percentage.round())))
    }
}
