use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct Tree {
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for Tree {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div().flex().flex_col().p_2().child(
            div()
                .text_sm()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Tree (placeholder)"),
        )
    }
}
