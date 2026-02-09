use gpui::*;
use gpui_component::ActiveTheme;
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.visible == Some(false) {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .rounded_md()
            .bg(cx.theme().colors.secondary)
            .children(self.children)
            .into_any_element()
    }
}
