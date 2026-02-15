use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Stack {
    #[property(default = "vertical")]
    direction: String,
    #[property(default = 4)]
    spacing: i64,
    #[property]
    scroll: Option<bool>,
    #[source]
    source: nemo_layout::BuiltComponent,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let gap = px(self.spacing as f32);
        let is_horizontal = self.direction == "horizontal";
        let scroll = self.scroll == Some(true);

        let mut base = div()
            .id(SharedString::from(self.source.id.clone()))
            .flex()
            .flex_1()
            .min_h(px(0.))
            .gap(gap);
        base = if is_horizontal {
            let b = base.flex_row();
            if scroll {
                b.overflow_x_scroll()
            } else {
                b
            }
        } else {
            let b = base.flex_col();
            if scroll {
                b.overflow_y_scroll()
            } else {
                b
            }
        };

        base.children(self.children).into_any_element()
    }
}
