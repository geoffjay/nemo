use gpui::*;
use nemo_macros::NemoComponent;

/// A multi-line text block component.
///
/// # XML Configuration
///
/// ```xml
/// <text id="description" content="Some paragraph text here." />
/// <!-- equivalently, the element body is the content -->
/// <text id="description">Some paragraph text here.</text>
/// <!-- inline bodies are whitespace-normalized (trim + collapse internal
///      runs to single spaces); opt out to keep the body verbatim -->
/// <text xml:space="preserve">preformatted   body</text>
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
