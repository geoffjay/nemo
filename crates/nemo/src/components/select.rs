use gpui::*;
use gpui_component::ActiveTheme;
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use crate::runtime::NemoRuntime;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Select {
    source: BuiltComponent,
    selected_value: Arc<Mutex<String>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Select {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            selected_value: Arc::new(Mutex::new(String::new())),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn selected_value(mut self, state: Arc<Mutex<String>>) -> Self {
        self.selected_value = state;
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

impl RenderOnce for Select {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let options: Vec<String> = self
            .source
            .properties
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let selected = self.selected_value.lock().unwrap().clone();
        let change_handler = self.source.handlers.get("change").cloned();
        let component_id = self.source.id.clone();

        let border_color = cx.theme().colors.border;
        let accent = cx.theme().colors.accent;
        let list_hover = cx.theme().colors.list_hover;

        let mut el = div()
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(border_color);

        for option in options {
            let is_selected = option == selected;
            let handler = change_handler.clone();
            let cid = component_id.clone();
            let opt = option.clone();
            let runtime = self.runtime.clone();
            let entity_id = self.entity_id;
            let shared_state = Arc::clone(&self.selected_value);

            let mut item = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "{}-{}",
                    self.source.id, option
                ))))
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .child(option.clone());

            if is_selected {
                item = item.bg(accent);
            } else {
                item = item.hover(move |s| s.bg(list_hover));
            }

            item = item.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                *shared_state.lock().unwrap() = opt.clone();
                if let Some(ref handler) = handler {
                    if let Some(ref runtime) = runtime {
                        runtime.call_handler(handler, &cid, &opt);
                    }
                }
                if let Some(eid) = entity_id {
                    cx.notify(eid);
                }
            });

            el = el.child(item);
        }

        el
    }
}
