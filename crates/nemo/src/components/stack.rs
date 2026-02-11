use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

use super::{apply_shadow, resolve_color};

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
    #[property]
    shadow: Option<String>,
    #[property]
    scroll: Option<bool>,
    #[source]
    source: nemo_layout::BuiltComponent,
    #[children]
    children: Vec<AnyElement>,
}

impl Stack {
    fn apply_shadow_stateful(
        base: Stateful<Div>,
        shadow: Option<&str>,
    ) -> Stateful<Div> {
        match shadow {
            Some("sm") => base.shadow_sm(),
            Some("md") => base.shadow_md(),
            Some("lg") => base.shadow_lg(),
            Some("xl") => base.shadow_xl(),
            Some("2xl") => base.shadow_2xl(),
            _ => base,
        }
    }
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = px(self.spacing as f32);
        let is_horizontal = self.direction == "horizontal";
        let scroll = self.scroll == Some(true);

        if scroll {
            let mut base = div()
                .id(SharedString::from(self.source.id.clone()))
                .flex()
                .flex_1()
                .min_h(px(0.))
                .gap(gap);
            base = if is_horizontal {
                base.flex_row().overflow_x_scroll()
            } else {
                base.flex_col().overflow_y_scroll()
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
            base = Self::apply_shadow_stateful(base, self.shadow.as_deref());
            base.children(self.children).into_any_element()
        } else {
            let mut base = div().flex().flex_1().min_h(px(0.)).gap(gap);
            base = if is_horizontal {
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
            base = apply_shadow(base, self.shadow.as_deref());
            base.children(self.children).into_any_element()
        }
    }
}
