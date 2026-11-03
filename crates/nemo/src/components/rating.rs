use gpui::*;
use gpui_kit::component::rating::Rating as GpuiRating;
use gpui_kit::component::Size as ComponentSize;
use nemo_layout::BuiltComponent;

use super::resolve_color;
use crate::runtime::NemoRuntime;
use std::rc::Rc;
use std::sync::Arc;

/// A star rating row (gpui-kit `Rating`). The kit owns the current value in
/// its own keyed state, seeded from `value`: clicking an unfilled star sets
/// the value, clicking a filled star steps down by one, and `on-click`
/// receives the new value so the app can persist it. Hover previews the
/// would-be value.
///
/// The active color defaults to the theme's yellow; override it with
/// `color` (`theme.*` reference or hex).
///
/// ```xml
/// <rating id="product" value="3" max="5" on-click="handle_rating" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `value` | integer | Initial rating, 0..=max (kit-owned afterwards) |
/// | `max` | integer | Number of stars, default 5 |
/// | `size` | string | `xsmall`, `small`, `medium` (default), or `large` |
/// | `color` | string | Active color: `theme.*` reference or hex (default theme yellow) |
/// | `disabled` | bool | Read-only display |
/// | `on-click` | string | Handler invoked with the new rating value |
#[derive(IntoElement)]
pub struct Rating {
    source: BuiltComponent,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Rating {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            runtime: None,
            entity_id: None,
        }
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    fn size_from(value: Option<&str>) -> ComponentSize {
        match value {
            Some("xsmall") => ComponentSize::XSmall,
            Some("small") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        }
    }
}

/// Type alias for the kit rating's value-click callback.
type RatingClickFn = dyn Fn(&usize, &mut Window, &mut App);

impl RenderOnce for Rating {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let value = props
            .get("value")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0) as usize;
        let max = props
            .get("max")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .max(1) as usize;
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let color = props
            .get("color")
            .and_then(|v| v.as_str())
            .and_then(|c| resolve_color(c, cx));

        let component_id = self.source.id.clone();
        let handler = self.source.handlers.get("click").cloned();
        let runtime = self.runtime.clone();
        let entity_id = self.entity_id;
        let on_click: Option<Rc<RatingClickFn>> = handler.map(|handler| {
            let click_component_id = component_id.clone();
            let runtime = runtime.expect("runtime present when handler is");
            let entity_id = entity_id.expect("entity_id present when handler is");
            let f: Rc<RatingClickFn> =
                Rc::new(move |value: &usize, _window: &mut Window, cx: &mut App| {
                    runtime.call_handler(&handler, &click_component_id, &value.to_string());
                    cx.notify(entity_id);
                });
            f
        });

        let mut rating = GpuiRating::new(ElementId::Name(SharedString::from(format!(
            "{}-rating",
            component_id
        ))))
        .value(value)
        .max(max)
        .with_size(Self::size_from(props.get("size").and_then(|v| v.as_str())));
        if let Some(color) = color {
            rating = rating.color(color);
        }
        if disabled {
            rating = rating.disabled(true);
        }
        if let Some(on_click) = on_click {
            rating = rating.on_click(move |value, window, cx| on_click(value, window, cx));
        }

        rating
    }
}
