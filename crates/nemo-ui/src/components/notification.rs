use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

/// A toast notification component.
///
/// # XML Configuration
///
/// ```xml
/// <notification id="saved" message="Changes saved successfully." kind="success" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `message` | string | Notification text |
/// | `kind` | string | Notification type: `"info"`, `"success"`, `"warning"`, or `"error"` |
#[derive(IntoElement, NemoComponent)]
pub struct Notification {
    #[property(default = "")]
    message: String,
    #[property(name = "kind", default = "info")]
    notification_type: String,
}

impl RenderOnce for Notification {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let (bg, border_color) = match self.notification_type.as_str() {
            "error" => (colors.danger_hover, colors.danger),
            "warning" => (colors.warning_hover, colors.warning),
            "success" => (colors.success_hover, colors.success),
            _ => (colors.info_hover, colors.info), // info
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
