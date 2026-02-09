use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Panel {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    visible: Option<bool>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if self.visible == Some(false) {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .rounded_md()
            .bg(rgb(0x313244))
            .children(self.children)
            .into_any_element()
    }
}
