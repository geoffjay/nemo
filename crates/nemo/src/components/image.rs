use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Image {
    #[property(default = "")]
    src: String,
    #[property]
    alt: Option<String>,
}

impl RenderOnce for Image {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if self.src.is_empty() {
            return div()
                .child(self.alt.unwrap_or_else(|| "No image".to_string()))
                .into_any_element();
        }
        div()
            .child(img(SharedString::from(self.src)))
            .into_any_element()
    }
}
