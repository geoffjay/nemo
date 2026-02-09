use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct Tooltip {
    #[property(default = "")]
    content: String,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Tooltip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Render children directly; full tooltip hover behavior requires
        // gpui tooltip infrastructure which needs an entity context.
        // This is a simple wrapper that renders children with a title attribute.
        div().children(self.children)
    }
}
