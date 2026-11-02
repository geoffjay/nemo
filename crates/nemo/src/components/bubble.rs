use gpui::*;
use gpui_kit::component::bubble::{
    Bubble as GpuiBubble, BubbleGroup as GpuiBubbleGroup, BubbleReactionSide,
    BubbleReactions as GpuiBubbleReactions, BubbleVariant,
};
use gpui_kit::component::message::MessageAlignment;
use nemo_layout::BuiltComponent;

/// A chat bubble: the surface-level primitive for a conversation (gpui-kit
/// `Bubble`). It owns alignment, variant colors, an optional reaction region,
/// and a max width of 80% of its parent; text and arbitrary XML children fill
/// the content slot. Presentational only — interactive content goes in
/// child elements (buttons, links).
///
/// # XML Configuration
///
/// ```xml
/// <bubble id="incoming" alignment="start" variant="secondary" text="Can you review this draft?" />
/// <bubble id="outgoing" alignment="end" text="Done!" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `text` | string | Plain-text content of the bubble |
/// | `alignment` | string | Row placement: `start` (incoming) or `end` (outgoing) |
/// | `variant` | string | Surface treatment: `filled` (default), `secondary`, `muted`, `tinted`, `outline`, `ghost`, or `destructive` |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Bubble {
    source: BuiltComponent,
    children: Vec<AnyElement>,
    reactions: Option<BubbleReactionsData>,
}

impl Bubble {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            children: Vec::new(),
            reactions: None,
        }
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }

    pub fn reactions(mut self, reactions: BubbleReactionsData) -> Self {
        self.reactions = Some(reactions);
        self
    }
}

/// One `<bubble-reactions>` child of a `<bubble>`: an absolutely positioned
/// pill on the bubble's top or bottom edge. Interaction stays with the child
/// elements (buttons keep their own handlers via the generic child path).
#[derive(Default)]
pub struct BubbleReactionsData {
    pub side: Option<String>,
    pub alignment: Option<String>,
    pub children: Vec<AnyElement>,
}

fn alignment_from(value: Option<&str>) -> MessageAlignment {
    match value {
        Some("start") => MessageAlignment::Start,
        _ => MessageAlignment::End,
    }
}

impl RenderOnce for Bubble {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let text = props.get("text").and_then(|v| v.as_str());
        let alignment = props.get("alignment").and_then(|v| v.as_str());
        let variant = props
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("filled");

        let mut content_children: Vec<AnyElement> = Vec::new();
        if let Some(text) = text {
            content_children.push(text.to_string().into_any_element());
        }
        content_children.extend(self.children);

        let mut bubble = GpuiBubble::new()
            .with_variant(match variant {
                "secondary" => BubbleVariant::Secondary,
                "muted" => BubbleVariant::Muted,
                "tinted" => BubbleVariant::Tinted,
                "outline" => BubbleVariant::Outline,
                "ghost" => BubbleVariant::Ghost,
                "destructive" => BubbleVariant::Destructive,
                _ => BubbleVariant::Filled,
            })
            .children(content_children);

        // Unset alignment means the parent (e.g. a Message row) owns
        // placement; a standalone bubble stretches to its max width instead of
        // forcing an edge.
        match alignment {
            Some("start") => bubble = bubble.alignment(MessageAlignment::Start),
            Some("end") => bubble = bubble.alignment(MessageAlignment::End),
            _ => {}
        }

        if let Some(reactions) = self.reactions {
            let mut region = GpuiBubbleReactions::new()
                .alignment(alignment_from(reactions.alignment.as_deref()));
            if reactions.side.as_deref() == Some("top") {
                region = region.side(BubbleReactionSide::Top);
            }
            bubble = bubble.reactions(region.children(reactions.children));
        }

        bubble
    }
}

/// A vertical stack of consecutive bubbles from one sender (gpui-kit
/// `BubbleGroup`). It only supplies the shared rhythm; sender identity and
/// headers stay with the application.
///
/// # XML Configuration
///
/// ```xml
/// <bubble-group id="alice">
///   <bubble alignment="start" variant="secondary" text="First paragraph" />
///   <bubble alignment="start" variant="secondary" text="Second paragraph" />
/// </bubble-group>
/// ```
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct BubbleGroup {
    #[allow(dead_code)]
    source: BuiltComponent,
    children: Vec<AnyElement>,
}

impl BubbleGroup {
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

impl RenderOnce for BubbleGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        GpuiBubbleGroup::new().children(self.children)
    }
}
