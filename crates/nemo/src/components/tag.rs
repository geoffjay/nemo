use gpui::*;
use gpui_component::tag::Tag as GpuiTag;
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Tag {
    source: BuiltComponent,
}

impl Tag {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let label = props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Tag")
            .to_string();
        let variant = props.get("variant").and_then(|v| v.as_str()).unwrap_or("");
        let outline = props
            .get("outline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut tag = match variant {
            "primary" => GpuiTag::primary(),
            "danger" => GpuiTag::danger(),
            "success" => GpuiTag::success(),
            "warning" => GpuiTag::warning(),
            "info" => GpuiTag::info(),
            _ => GpuiTag::secondary(),
        };

        if outline {
            tag = tag.outline();
        }

        // Wrap in a flex_none div so the tag doesn't stretch in flex parents
        div().flex_none().child(tag.child(label))
    }
}
