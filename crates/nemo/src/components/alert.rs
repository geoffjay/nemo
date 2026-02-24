use gpui::*;
use gpui_component::alert::{Alert as GpuiAlert, AlertVariant};
use nemo_layout::BuiltComponent;

/// An alert/status message component.
///
/// # XML Configuration
///
/// ```xml
/// <alert id="warn" message="Disk space is running low." variant="warning" title="Warning" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `message` | string | Alert body text |
/// | `variant` | string | Alert type: `"info"`, `"success"`, `"warning"`, or `"error"` |
/// | `title` | string | Optional title displayed above the message |
#[derive(IntoElement)]
pub struct Alert {
    source: BuiltComponent,
}

impl Alert {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let message = props
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let variant = props.get("variant").and_then(|v| v.as_str()).unwrap_or("");
        let title = props.get("title").and_then(|v| v.as_str());

        let mut alert = GpuiAlert::new(
            SharedString::from(self.source.id.clone()),
            SharedString::from(message),
        );

        alert = match variant {
            "info" => alert.with_variant(AlertVariant::Info),
            "success" => alert.with_variant(AlertVariant::Success),
            "warning" => alert.with_variant(AlertVariant::Warning),
            "error" => alert.with_variant(AlertVariant::Error),
            _ => alert,
        };

        if let Some(t) = title {
            alert = alert.title(SharedString::from(t.to_string()));
        }

        alert
    }
}
