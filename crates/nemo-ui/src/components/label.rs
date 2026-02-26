use gpui::*;
use gpui_component::label::Label as GpuiLabel;
use nemo_macros::NemoComponent;

/// A text display component.
///
/// # XML Configuration
///
/// ```xml
/// <label id="title" text="Hello World" size="lg" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `text` | string | The text content to display |
/// | `size` | string | Text size: `"sm"`, `"md"` (default), `"lg"`, or `"xl"` |
#[derive(IntoElement, NemoComponent)]
pub struct Label {
    #[property(default = "")]
    text: String,
    #[property(default = "md")]
    size: String,
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let label = GpuiLabel::new(SharedString::from(self.text));

        match self.size.as_str() {
            "xs" => label.text_xs(),
            "sm" => label.text_sm(),
            "lg" => label.text_lg(),
            "xl" => label.text_xl(),
            _ => label,
        }
    }
}
