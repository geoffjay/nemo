use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Modal {
    #[property(default = "")]
    title: String,
    #[property]
    open: Option<bool>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Modal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.open.unwrap_or(false) {
            return div().into_any_element();
        }

        let bg = cx.theme().colors.background;
        let border = cx.theme().colors.border;

        // Overlay backdrop
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000088))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_6()
                    .rounded_lg()
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .min_w(px(300.0))
                    .max_w(px(600.0))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child(self.title),
                    )
                    .children(self.children),
            )
            .into_any_element()
    }
}
