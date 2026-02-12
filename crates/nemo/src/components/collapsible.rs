use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::v_flex;
use gpui_component::{ActiveTheme, Sizable};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Collapsible {
    source: BuiltComponent,
    open_state: Arc<Mutex<bool>>,
    children: Vec<AnyElement>,
    entity_id: Option<EntityId>,
}

impl Collapsible {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            open_state: Arc::new(Mutex::new(false)),
            children: Vec::new(),
            entity_id: None,
        }
    }

    pub fn open_state(mut self, state: Arc<Mutex<bool>>) -> Self {
        self.open_state = state;
        self
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for Collapsible {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let title = props
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Details")
            .to_string();

        let open = *self.open_state.lock().unwrap();

        let shared_state = Arc::clone(&self.open_state);
        let entity_id = self.entity_id;
        let toggle_id = ElementId::Name(SharedString::from(format!("{}-toggle", self.source.id)));

        let chevron = if open {
            gpui_component::IconName::ChevronDown
        } else {
            gpui_component::IconName::ChevronRight
        };

        let border_color = cx.theme().border;

        v_flex()
            .w_full()
            .child(
                div()
                    .id(toggle_id)
                    .flex()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .px_2()
                    .cursor_pointer()
                    .border_b_1()
                    .border_color(border_color)
                    .hover(|s| s.bg(gpui::hsla(0., 0., 0.5, 0.1)))
                    .child(gpui_component::Icon::new(chevron).xsmall())
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .on_click(move |_, _window, cx| {
                        let mut state = shared_state.lock().unwrap();
                        *state = !*state;
                        if let Some(eid) = entity_id {
                            cx.notify(eid);
                        }
                    }),
            )
            .when(open, |this| this.child(div().p_2().children(self.children)))
    }
}
