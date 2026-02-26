use gpui::*;
use gpui_component::tooltip::Tooltip as GpuiTooltip;
use nemo_layout::BuiltComponent;

/// A tooltip wrapper component that shows a popup on hover.
///
/// # XML Configuration
///
/// ```xml
/// <tooltip id="help-tip" content="Click to submit the form">
///   <button label="Submit" />
/// </tooltip>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `content` | string | Text displayed in the tooltip popup |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Tooltip {
    source: BuiltComponent,
    children: Vec<AnyElement>,
}

impl Tooltip {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            children: Vec::new(),
        }
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }
}

impl RenderOnce for Tooltip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let content = self
            .source
            .properties
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let id = ElementId::Name(SharedString::from(self.source.id.clone()));

        div()
            .id(id)
            .children(self.children)
            .tooltip(move |window, cx| {
                GpuiTooltip::new(SharedString::from(content.clone())).build(window, cx)
            })
    }
}
