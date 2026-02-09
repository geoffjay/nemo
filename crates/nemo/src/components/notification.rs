use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Notification {
    #[property(default = "")]
    message: String,
    #[property(name = "kind", default = "info")]
    notification_type: String,
}

impl RenderOnce for Notification {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, border_color) = match self.notification_type.as_str() {
            "error" => (rgb(0x45262e), rgb(0xf38ba8)),
            "warning" => (rgb(0x45392e), rgb(0xfab387)),
            "success" => (rgb(0x2e4536), rgb(0xa6e3a1)),
            _ => (rgb(0x2e3545), rgb(0x89b4fa)), // info
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_4()
            .py_3()
            .rounded_md()
            .bg(bg)
            .border_l_4()
            .border_color(border_color)
            .child(self.message)
    }
}
