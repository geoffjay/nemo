use gpui::*;
use gpui_component::badge::Badge as GpuiBadge;
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Badge {
    source: BuiltComponent,
    children: Vec<AnyElement>,
}

impl Badge {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            children: Vec::new(),
        }
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let count = props.get("count").and_then(|v| v.as_i64());
        let dot = props.get("dot").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut badge = GpuiBadge::new();

        if dot {
            badge = badge.dot();
        } else if let Some(c) = count {
            badge = badge.count(c as usize);
        }

        badge.children(self.children)
    }
}
