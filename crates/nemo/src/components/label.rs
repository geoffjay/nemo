use gpui::*;
use gpui_component::label::Label as GpuiLabel;
use nemo_macros::NemoComponent;

use crate::theme::tokens::{radius_for, FontSize, TokenStyled};

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
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property(default = "")]
    text: String,
    #[property(default = "md")]
    size: String,
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = GpuiLabel::new(SharedString::from(self.text));

        let label = match self.size.as_str() {
            "xs" => label.text_t(FontSize::Xs),
            "sm" => label.text_t(FontSize::Sm),
            "lg" => label.text_t(FontSize::Lg),
            "xl" => label.text_t(FontSize::Xl),
            _ => label,
        };

        match self
            .source
            .properties
            .get("rounded")
            .and_then(|v| v.as_str())
        {
            Some(name) => match radius_for(name, cx) {
                Some(r) => label.rounded(r),
                None => label,
            },
            None => label,
        }
    }
}
