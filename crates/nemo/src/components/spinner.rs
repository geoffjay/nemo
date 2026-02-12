use gpui::*;
use gpui_component::spinner::Spinner as GpuiSpinner;
use gpui_component::{Sizable, Size as ComponentSize};
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Spinner {
    source: BuiltComponent,
}

impl Spinner {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let mut spinner = GpuiSpinner::new();

        if let Some(size) = props.get("size").and_then(|v| v.as_str()) {
            spinner = match size {
                "xs" => spinner.with_size(ComponentSize::XSmall),
                "sm" => spinner.with_size(ComponentSize::Small),
                "lg" => spinner.with_size(ComponentSize::Large),
                _ => spinner,
            };
        }

        spinner
    }
}
