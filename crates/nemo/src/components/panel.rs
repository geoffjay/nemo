use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

use super::resolve_color;

#[derive(IntoElement, NemoComponent)]
pub struct Panel {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    visible: Option<bool>,
    #[property]
    padding: Option<i64>,
    #[property]
    border: Option<i64>,
    #[property]
    border_color: Option<String>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.visible == Some(false) {
            return div().into_any_element();
        }

        let mut base = div()
            .flex()
            .flex_col()
            .rounded_md()
            .bg(cx.theme().colors.secondary);

        if let Some(p) = self.padding {
            base = base.p(px(p as f32));
        }

        if let Some(width) = self.border {
            base = base.border(px(width as f32));
            let color = self
                .border_color
                .as_deref()
                .and_then(|v| resolve_color(v, cx))
                .unwrap_or(cx.theme().colors.border);
            base = base.border_color(color);
        }

        base.children(self.children).into_any_element()
    }
}

