use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Window};
use gpui_kit::component::avatar::Avatar as GpuiAvatar;
use gpui_kit::component::bubble::{Bubble, BubbleVariant};
use gpui_kit::component::message::{
    Message as GpuiMessage, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
    MessageHeader,
};
use nemo_layout::BuiltComponent;

/// A chat message row: sender identity, metadata, a bubble body, and a
/// footer, aligned to the leading or trailing edge.
///
/// # XML Configuration
///
/// ```xml
/// <message id="msg_1" sender="Alice" timestamp="10:24 AM"
///         avatar="Alice" alignment="start" variant="secondary"
///         footer="Read">
///   Can you review this draft?
/// </message>
/// ```
///
/// # Properties
///
/// | Property   | Type    | Default     | Description                          |
/// |-----------|---------|-------------|--------------------------------------|
/// | `sender`    | string  | none        | Sender name shown in the header      |
/// | `timestamp` | string | none        | Metadata text shown after the sender |
/// | `avatar`     | string | none        | Avatar identity; omit for no avatar |
/// | `alignment`  | enum    | `start`     | `start` (leading) or `end` (trailing) |
/// | `variant`    | enum    | `secondary` | Bubble surface treatment            |
/// | `footer`     | string  | none        | Footer text (delivery state, hints)  |
///
/// Children render as extra content below the bubble inside the body slot.
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Message {
    source: BuiltComponent,
    children: Vec<AnyElement>,
}

impl Message {
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

/// Maps the schema's kebab-case bubble variant to the kit enum.
fn bubble_variant(name: &str) -> BubbleVariant {
    match name {
        "filled" => BubbleVariant::Filled,
        "muted" => BubbleVariant::Muted,
        "tinted" => BubbleVariant::Tinted,
        "outline" => BubbleVariant::Outline,
        "ghost" => BubbleVariant::Ghost,
        "destructive" => BubbleVariant::Destructive,
        _ => BubbleVariant::Secondary,
    }
}

impl RenderOnce for Message {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let sender = props.get("sender").and_then(|v| v.as_str());
        let timestamp = props.get("timestamp").and_then(|v| v.as_str());
        let avatar = props.get("avatar").and_then(|v| v.as_str());
        let footer = props.get("footer").and_then(|v| v.as_str());
        let alignment = match props.get("alignment").and_then(|v| v.as_str()) {
            Some("end") => MessageAlignment::End,
            _ => MessageAlignment::Start,
        };
        // Docs examples pair incoming with the near-background secondary
        // surface and outgoing with the filled primary surface.
        let variant = props
            .get("variant")
            .and_then(|v| v.as_str())
            .map(bubble_variant)
            .unwrap_or(match alignment {
                MessageAlignment::Start => BubbleVariant::Secondary,
                MessageAlignment::End => BubbleVariant::Filled,
            });

        let mut message = GpuiMessage::new().alignment(alignment);

        if let Some(name) = avatar {
            message = message.avatar_slot(
                MessageAvatar::new().child(GpuiAvatar::new().name(SharedString::from(name))),
            );
        }

        if sender.is_some() || timestamp.is_some() {
            let mut header = MessageHeader::new();
            if let Some(sender) = sender {
                header = header.child(sender.to_string());
            }
            if let Some(timestamp) = timestamp {
                header = header.child(timestamp.to_string());
            }
            message = message.header(header);
        }

        // Body: one bubble carrying the message text, then any children as
        // extra rich content below it.
        let mut content = MessageContent::new();
        let text = self
            .source
            .properties
            .get("text")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if let Some(text) = text {
            content = content.bubble(Bubble::new().with_variant(variant).child(text));
        }
        if !self.children.is_empty() {
            content = content.children(self.children);
        }
        message = message.content(content);

        if let Some(footer) = footer {
            message = message.footer(MessageFooter::new().child(footer.to_string()));
        }

        message
    }
}
