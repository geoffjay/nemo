use gpui::*;
use gpui_component::avatar::Avatar as GpuiAvatar;
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Avatar {
    source: BuiltComponent,
}

impl Avatar {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let mut avatar = GpuiAvatar::new();

        if let Some(name) = props.get("name").and_then(|v| v.as_str()) {
            avatar = avatar.name(SharedString::from(name.to_string()));
        }

        avatar
    }
}
