use gpui::*;
use gpui_kit::component::marker::{
    Marker as GpuiMarker, MarkerAlignment, MarkerContent, MarkerIcon, MarkerLoadingStyle,
    MarkerVariant,
};
use gpui_kit::component::Icon as GpuiIcon;
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;
use super::resolve_color;

/// A compact, composable row for status text, timeline boundaries, unread
/// labels, and system notices (gpui-kit `Marker`). Accepts an optional icon
/// name, text (which the shimmer loading effect animates), and arbitrary XML
/// children for rich composition.
///
/// # XML Configuration
///
/// ```xml
/// <marker id="status" icon="circle-check" text="Synced" text-color="theme.success" />
/// <marker id="boundary" variant="separator" text="Today" />
/// <marker id="unread" variant="border" icon="info" text="3 unread messages" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `text` | string | Label text; animated by the shimmer loading style |
/// | `icon` | string | Icon name shown in the compact leading slot |
/// | `variant` | string | Visual treatment: `plain` (default), `separator`, or `border` |
/// | `alignment` | string | Child placement: `start` (default), `center`, or `end` |
/// | `loading` | bool | Show the loading effect while keeping variant and layout |
/// | `loading-style` | string | Loading effect: `spinner` (default) or `shimmer` |
/// | `text-color` | string | Label color as `theme.<role>` or hex |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Marker {
    source: BuiltComponent,
    children: Vec<AnyElement>,
}

impl Marker {
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

impl RenderOnce for Marker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let text = props.get("text").and_then(|v| v.as_str());
        let icon = props.get("icon").and_then(|v| v.as_str());
        let variant = props.get("variant").and_then(|v| v.as_str());
        let alignment = props.get("alignment").and_then(|v| v.as_str());
        let loading = props
            .get("loading")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let loading_style = props.get("loading_style").and_then(|v| v.as_str());
        let text_color = props.get("text_color").and_then(|v| v.as_str());

        let mut marker = GpuiMarker::new()
            .with_variant(match variant {
                Some("separator") => MarkerVariant::Separator,
                Some("border") => MarkerVariant::Border,
                _ => MarkerVariant::Plain,
            })
            .with_loading_style(match loading_style {
                Some("shimmer") => MarkerLoadingStyle::Shimmer,
                _ => MarkerLoadingStyle::Spinner,
            })
            .loading(loading);

        if let Some(alignment) = alignment {
            marker = marker.alignment(match alignment {
                "center" => MarkerAlignment::Center,
                "end" => MarkerAlignment::End,
                _ => MarkerAlignment::Start,
            });
        }
        if let Some(color) = text_color.and_then(|c| resolve_color(c, cx)) {
            marker = marker.text_color(color);
        }
        // An explicit icon slot suppresses the automatic spinner under
        // spinner-style loading, mirroring upstream behavior.
        if let Some(name) = icon {
            marker = marker.icon(MarkerIcon::new().child(GpuiIcon::new(map_icon_name(name))));
        }
        if let Some(text) = text {
            marker = marker.content(MarkerContent::new().text(text));
        }

        marker.children(self.children)
    }
}
