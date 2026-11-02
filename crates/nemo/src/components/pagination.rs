use gpui::*;
use gpui_kit::component::pagination::Pagination as GpuiPagination;
use gpui_kit::component::{Disableable, Sizable, Size as ComponentSize};
use nemo_layout::BuiltComponent;

use crate::runtime::NemoRuntime;
use std::rc::Rc;
use std::sync::Arc;

/// Page navigation with previous/next buttons and (unless `compact`) page
/// number buttons; hidden ranges collapse into an ellipsis whose dropdown
/// menu lists the hidden pages (gpui-kit `Pagination`).
///
/// The current page is 1-based and application-owned: supply `current-page`
/// from state, and handle `on-click`, which receives the target page number,
/// to update it. Without a handler the kit leaves navigation unwired, so the
/// bar renders but does not respond.
///
/// ```xml
/// <pagination id="list_pages" current-page="5" total-pages="10" on-click="handle_page" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `current-page` | integer | Current page, 1-based, clamped to `total-pages` |
/// | `total-pages` | integer | Total number of pages clamped to at least 1 |
/// | `visible-pages` | integer | Maximum number of visible page buttons |
/// | `compact` | bool | Show only prev/next buttons without page numbers |
/// | `size` | string | `xsmall`, `small`, `medium` (default), or `large` |
/// | `disabled` | bool | Disable every button |
/// | `on-click` | string | Handler invoked with the target page (1-based) |
#[derive(IntoElement)]
pub struct Pagination {
    source: BuiltComponent,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Pagination {
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

/// Type alias for the kit pagination's page-click callback.
type PageClickFn = dyn Fn(&usize, &mut Window, &mut App);

impl RenderOnce for Pagination {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let current_page = props
            .get("current_page")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1) as usize;
        let total_pages = props
            .get("total_pages")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1) as usize;
        let visible_pages = props
            .get("visible_pages")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .max(1) as usize;
        let compact = props
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let component_id = self.source.id.clone();
        let handler = self.source.handlers.get("click").cloned();
        let runtime = self.runtime.clone();
        let entity_id = self.entity_id;
        let on_click: Option<Rc<PageClickFn>> = handler.map(|handler| {
            let click_component_id = component_id.clone();
            let runtime = runtime.expect("runtime present when handler is");
            let entity_id = entity_id.expect("entity_id present when handler is");
            let f: Rc<PageClickFn> =
                Rc::new(move |page: &usize, _window: &mut Window, cx: &mut App| {
                    runtime.call_handler(&handler, &click_component_id, &page.to_string());
                    cx.notify(entity_id);
                });
            f
        });

        let mut pagination = GpuiPagination::new(ElementId::Name(SharedString::from(format!(
            "{}-pagination",
            component_id
        ))))
        .current_page(current_page)
        .total_pages(total_pages)
        .visible_pages(visible_pages)
        .with_size(Self::size_from(props.get("size").and_then(|v| v.as_str())));
        if compact {
            pagination = pagination.compact();
        }
        if disabled {
            pagination = pagination.disabled(true);
        }
        if let Some(on_click) = on_click {
            pagination = pagination.on_click(move |page, window, cx| on_click(page, window, cx));
        }

        pagination.into_any_element()
    }
}
