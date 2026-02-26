use gpui::*;
use nemo_macros::NemoComponent;

/// An image display component.
///
/// # XML Configuration
///
/// ```xml
/// <image id="logo" src="https://example.com/logo.png" alt="Company Logo" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `src` | string | URL or file path of the image |
/// | `alt` | string | Alternative text shown when image cannot load |
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
