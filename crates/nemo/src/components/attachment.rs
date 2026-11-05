use gpui::*;
use gpui_kit::component::attachment::{
    Attachment as GpuiAttachment, AttachmentActions as GpuiAttachmentActions,
    AttachmentContent as GpuiAttachmentContent, AttachmentDescription,
    AttachmentMedia as GpuiAttachmentMedia, AttachmentStatus, AttachmentTitle,
};
use gpui_kit::component::{Icon as GpuiIcon, Sizable, Size as ComponentSize};
use nemo_layout::BuiltComponent;
use std::sync::Arc;

use super::icon::map_icon_name;
use crate::runtime::NemoRuntime;

/// A file or media attachment chip/card (gpui-kit `Attachment`). A card is
/// composed from three slots given as children — `<attachment-media>` for the
/// preview, `<attachment-content>` for the metadata, and
/// `<attachment-actions>` for controls — plus lifecycle `status` styling and
/// built-in remove/retry/click handlers.
///
/// # XML Configuration
///
/// ```xml
/// <attachment id="report" status="complete" size="md">
///   <attachment-media icon="file-text" />
///   <attachment-content title="quarterly-report.pdf" description="PDF · 2.4 MB" />
///   <attachment-actions>
///     <button id="att-remove" label="Remove" variant="ghost" size="xs" on-click="on_remove" />
///   </attachment-actions>
/// </attachment>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `status` | string | Lifecycle: `pending` (dashed border), `uploading`, `processing`, `failed` (destructive border), or `complete` (default) |
/// | `axis` | string | Layout: `horizontal` (default chip) or `vertical` (preview above metadata) |
/// | `size` | string | Chip scale: `xs`, `sm`, `md` (default), or `lg` |
/// | `progress` | float | Upload percent 0–100; determinate ring + bottom bar while `uploading` |
/// | `tooltip` | string | Hover text, e.g. the failure reason |
/// | `on-click` | string | Handler named by right-clicking / activating the whole card |
/// | `on-remove` | string | Handler for the built-in corner remove control |
/// | `on-retry` | string | Handler for the retry control shown while `failed` |
///
/// The built-in `on-remove` / `on-retry` controls appear only when those
/// handlers are present; slots are optional, so an icon-only or
/// metadata-only attachment is valid.
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Attachment {
    source: BuiltComponent,
    media: Option<AttachmentMediaData>,
    content: Option<AttachmentContentData>,
    action_children: Vec<AnyElement>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Attachment {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            media: None,
            content: None,
            action_children: Vec::new(),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn media(mut self, media: AttachmentMediaData) -> Self {
        self.media = Some(media);
        self
    }

    pub fn content(mut self, content: AttachmentContentData) -> Self {
        self.content = Some(content);
        self
    }

    pub fn action_children(mut self, children: Vec<AnyElement>) -> Self {
        self.action_children = children;
        self
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

/// One `<attachment-media>` child of an `<attachment>`: the preview slot.
/// `src` renders an image cover; `icon` renders a named glyph; generic
/// children paint above the preview.
#[derive(Default)]
pub struct AttachmentMediaData {
    pub src: Option<String>,
    pub icon: Option<String>,
    pub children: Vec<AnyElement>,
}

/// One `<attachment-content>` child of an `<attachment>`: the metadata slot.
/// `title` and `description` are single-line, status-aware texts; generic
/// children (badges, progress) append below them.
#[derive(Default)]
pub struct AttachmentContentData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub children: Vec<AnyElement>,
}

fn status_from(value: Option<&str>) -> AttachmentStatus {
    match value {
        Some("pending") => AttachmentStatus::Pending,
        Some("uploading") => AttachmentStatus::Uploading,
        Some("processing") => AttachmentStatus::Processing,
        Some("failed") => AttachmentStatus::Failed,
        _ => AttachmentStatus::Complete,
    }
}

fn size_from(value: Option<&str>) -> ComponentSize {
    match value {
        Some("xs") => ComponentSize::XSmall,
        Some("sm") => ComponentSize::Small,
        Some("lg") => ComponentSize::Large,
        _ => ComponentSize::Medium,
    }
}

impl RenderOnce for Attachment {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let status = status_from(props.get("status").and_then(|v| v.as_str()));
        let axis = if props.get("axis").and_then(|v| v.as_str()) == Some("vertical") {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };

        let component_id = self.source.id.clone();

        // The card's built-in controls key their state on identity, so the id
        // is always provided by the component's own id.
        let mut attachment = GpuiAttachment::new()
            .id(SharedString::from(component_id.clone()))
            .axis(axis);

        // Wire the built-in handlers through the runtime before the identity
        // is consumed by the slot builders.
        if let (Some(runtime), Some(entity_id)) = (self.runtime.clone(), self.entity_id) {
            for event in ["click", "remove", "retry"] {
                let Some(handler) = self.source.handlers.get(event).cloned() else {
                    continue;
                };
                let component_id = component_id.clone();
                let runtime = Arc::clone(&runtime);
                let call = move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    runtime.call_handler(&handler, &component_id, event);
                    cx.notify(entity_id);
                };
                attachment = match event {
                    "click" => attachment.on_click(call),
                    "remove" => attachment.on_remove(call),
                    "retry" => attachment.on_retry(call),
                    _ => unreachable!("event is one of the wired handler names"),
                };
            }
        }

        attachment = attachment
            .status(status)
            .with_size(size_from(props.get("size").and_then(|v| v.as_str())));

        if let Some(percent) = props
            .get("progress")
            .and_then(|v| v.as_f64())
            .filter(|_| status.is_uploading())
        {
            attachment = attachment.progress(percent as f32);
        }

        if let Some(text) = props.get("tooltip").and_then(|v| v.as_str()) {
            attachment = attachment.tooltip(text.to_string());
        }

        if let Some(media) = self.media {
            let mut slot = GpuiAttachmentMedia::new();
            if let Some(src) = media.src {
                slot = slot.src(SharedString::from(src));
            }
            if let Some(icon) = media.icon {
                slot = slot.child(GpuiIcon::new(map_icon_name(&icon)));
            }
            attachment = attachment.media(slot.children(media.children));
        }

        if let Some(content) = self.content {
            let mut slot = GpuiAttachmentContent::new();
            if let Some(title) = content.title {
                slot = slot.title(AttachmentTitle::new(title));
            }
            if let Some(description) = content.description {
                slot = slot.description(AttachmentDescription::new(description));
            }
            // Slot children (e.g. a Badge or Progress element) mount recursively
            // and continue below the metadata, avoiding duplicate titles.
            attachment = attachment.content(slot.children(content.children));
        }

        if !self.action_children.is_empty() {
            attachment =
                attachment.actions(GpuiAttachmentActions::new().children(self.action_children));
        }

        attachment
    }
}
