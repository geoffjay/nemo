use gpui::*;
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

#[derive(IntoElement, NemoComponent)]
pub struct Button {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property(default = "Button")]
    label: String,
    #[property]
    width: Option<i64>,
    #[property]
    height: Option<i64>,
    #[property]
    flex: Option<f64>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Button {
    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let click_handler = self.source.handlers.get("click").cloned();
        let component_id = self.source.id.clone();

        let mut el = div()
            .px_4()
            .py_2()
            .bg(rgb(0x89b4fa))
            .text_color(rgb(0x1e1e2e))
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0xb4befe)))
            .items_center()
            .justify_center()
            .child(self.label);

        if let Some(w) = self.width {
            el = el.w(px(w as f32));
        }
        if let Some(h) = self.height {
            el = el.h(px(h as f32));
        }
        if let Some(f) = self.flex {
            el = el.flex_grow().flex_basis(relative(f as f32));
        } else if self.width.is_none() {
            el = el.flex_1();
        }

        if let Some(handler) = click_handler {
            if let (Some(runtime), Some(entity_id)) = (self.runtime, self.entity_id) {
                el = el.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    runtime.call_handler(&handler, &component_id, "click");
                    cx.notify(entity_id);
                });
            }
        }

        el
    }
}
