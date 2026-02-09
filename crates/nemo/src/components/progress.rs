use gpui::*;
use gpui_component::progress::Progress as GpuiProgress;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Progress {
    #[property]
    value: Option<f64>,
    #[property(default = 100.0)]
    max: f64,
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let value = self.value.unwrap_or(0.0);
        let percentage = if self.max > 0.0 {
            (value / self.max * 100.0) as f32
        } else {
            0.0
        };
        GpuiProgress::new().value(percentage)
    }
}
