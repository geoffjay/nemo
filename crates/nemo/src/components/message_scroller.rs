use gpui::{
    div, px, AnyElement, ElementId, Entity, IntoElement, ParentElement as _, SharedString,
    Styled as _,
};
use gpui_kit::component::message_scroller::{
    MessageScroller as GpuiMessageScroller, MessageScrollerState,
};
use nemo_layout::BuiltComponent;

use super::resolve_color;

/// A virtualized message list (gpui-kit `MessageScroller`). Declarative XML
/// children are the rows, in order — the wrapper feeds each one to the kit's
/// indexed row renderer so only the visible viewport and overdraw region are
/// laid out.
///
/// Scroll bookkeeping (tail following, row anchoring, the jump-to-latest
/// button, and the optional scrollbar) lives in a shared
/// `MessageScrollerState` entity managed by the app. Scrolling notifies the
/// app so the jump button and bottom-fade fade in and out as the reader
/// leaves and rejoins the live tail.
///
/// ```xml
/// <message-scroller id="chat" height="260">
///   <message id="m1" sender="Ada" text="First message" />
///   <message id="m2" sender="Kit" text="Second message" />
/// </message-scroller>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `height` | integer | Viewport height in px (default 300; the list needs a definite height) |
/// | `scrollbar` | bool | Show the built-in vertical scrollbar (default true) |
/// | `jump-button` | bool | Show the built-in jump-to-latest button (default true) |
/// | `jump-button-label` | string | Tooltip text of the jump button (default "Jump to latest") |
/// | `bottom-fade` | color | `theme.*`/hex fade behind the bottom edge while scrolled away from the tail |
#[derive(IntoElement)]
pub struct MessageScroller {
    source: BuiltComponent,
    state: Entity<MessageScrollerState>,
    rows: Vec<AnyElement>,
}

impl MessageScroller {
    pub fn new(
        source: BuiltComponent,
        state: Entity<MessageScrollerState>,
        rows: Vec<AnyElement>,
    ) -> Self {
        Self {
            source,
            state,
            rows,
        }
    }

    fn named_id(&self, suffix: &str) -> ElementId {
        ElementId::Name(SharedString::from(format!("{}-{}", self.source.id, suffix)))
    }
}

impl gpui::RenderOnce for MessageScroller {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let props = &self.source.properties;
        let height = props.get("height").and_then(|v| v.as_f64()).unwrap_or(300.) as f32;
        let scrollbar = props
            .get("scrollbar")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let jump_button = props
            .get("jump_button")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let jump_label = props
            .get("jump_button_label")
            .and_then(|v| v.as_str())
            .unwrap_or("Jump to latest")
            .to_string();
        let root_id = self.named_id("root");
        let bottom_fade = props
            .get("bottom_fade")
            .and_then(|v| v.as_str())
            .and_then(|c| resolve_color(c, cx));

        // The kit's renderer is invoked per visible row index; hand out each
        // pre-built row exactly once. Indices past the declared rows render
        // empty (row count and children should stay in step).
        let mut rows: Vec<Option<AnyElement>> = self.rows.into_iter().map(Some).collect();
        let state = self.state;

        let mut scroller = GpuiMessageScroller::new(root_id, state, move |index, _, _| {
            rows.get_mut(index)
                .and_then(|slot| slot.take())
                .unwrap_or_else(|| div().into_any_element())
        })
        .scrollbar(scrollbar)
        .jump_button(jump_button)
        .with_jump_button_label(jump_label);

        if let Some(color) = bottom_fade {
            scroller = scroller.with_bottom_fade(color);
        }

        // The kit's root clips and sizes itself to the parent, so the wrapper
        // supplies the definite height the virtual list needs.
        div().w_full().h(px(height)).child(scroller)
    }
}
