use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct Table {
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x45475a))
            .p_2()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6c7086))
                    .child("Table (placeholder)"),
            )
    }
}
