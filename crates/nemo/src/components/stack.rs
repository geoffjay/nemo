use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Stack {
    #[property(default = "vertical")]
    direction: String,
    #[property(default = 4)]
    spacing: i64,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut base = div().flex().gap(px(self.spacing as f32));
        base = if self.direction == "horizontal" {
            base.flex_row()
        } else {
            base.flex_col()
        };
        base.children(self.children)
    }
}
