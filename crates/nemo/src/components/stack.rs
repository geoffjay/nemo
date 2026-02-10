use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

use super::resolve_color;

#[derive(IntoElement, NemoComponent)]
pub struct Stack {
    #[property(default = "vertical")]
    direction: String,
    #[property(default = 4)]
    spacing: i64,
    #[property]
    padding: Option<i64>,
    #[property]
    border: Option<i64>,
    #[property]
    border_color: Option<String>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut base = div().flex().flex_1().gap(px(self.spacing as f32));
        base = if self.direction == "horizontal" {
            base.flex_row()
        } else {
            base.flex_col()
        };
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
        base.children(self.children)
    }
}
