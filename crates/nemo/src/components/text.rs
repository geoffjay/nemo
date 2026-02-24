use gpui::*;
use nemo_macros::NemoComponent;

/// A multi-line text block component.
///
/// # XML Configuration
///
/// ```xml
/// <text id="description" content="Some paragraph text here." />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `content` | string | The text content to display |
/// | `bind-content` | string | Data binding expression for dynamic content |
#[derive(IntoElement, NemoComponent)]
pub struct Text {
    #[property(name = "content", default = "")]
    content: String,
}

impl RenderOnce for Text {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().child(self.content)
    }
}
