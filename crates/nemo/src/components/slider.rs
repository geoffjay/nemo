use gpui::*;
use gpui_component::slider::{Slider as GpuiSlider, SliderState};
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Slider {
    source: BuiltComponent,
    slider_state: Option<Entity<SliderState>>,
}

impl Slider {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            slider_state: None,
        }
    }

    pub fn slider_state(mut self, state: Entity<SliderState>) -> Self {
        self.slider_state = Some(state);
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.slider_state else {
            return div().child("Slider: missing state").into_any_element();
        };

        GpuiSlider::new(&state).into_any_element()
    }
}
