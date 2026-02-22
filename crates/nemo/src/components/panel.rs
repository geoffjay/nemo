use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Panel {
    #[source]
    #[allow(dead_code)]
    source: nemo_layout::BuiltComponent,
    #[property]
    visible: Option<bool>,
    #[property]
    padding: Option<i64>,
    #[property]
    border: Option<i64>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.visible == Some(false) {
            return div().into_any_element();
        }

        let mut el = div()
            .flex()
            .flex_col()
            .rounded_md()
            .bg(cx.theme().colors.secondary);

        if let Some(p) = self.padding {
            el = el.p(px(p as f32));
        }

        if let Some(b) = self.border {
            if b > 0 {
                el = el
                    .border(px(b as f32))
                    .border_color(cx.theme().colors.border);
            }
        }

        el.children(self.children).into_any_element()
    }
}
