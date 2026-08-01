use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

use crate::theme::tokens::{radius_of, Space, TokenStyled};

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
    #[source]
    source: nemo_layout::BuiltComponent,
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

        let mut el = div()
            .flex()
            .items_center()
            .gap_t(Space::Sm)
            .px_t(Space::Lg)
            .py_t(Space::Md)
            .bg(bg)
            .border_l_4()
            .border_color(border_color);

        el = match self
            .source
            .properties
            .get("rounded")
            .and_then(|v| v.as_str())
        {
            Some("none") => el,
            Some(name) => el.rounded(radius_of(name, cx)),
            _ => el.rounded(radius_of("md", cx)),
        };

        el.child(self.message)
    }
}
