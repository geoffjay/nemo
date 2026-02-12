use gpui::*;
use gpui_component::{ActiveTheme, Sizable};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use crate::runtime::NemoRuntime;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Toggle {
    source: BuiltComponent,
    checked_state: Arc<Mutex<bool>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Toggle {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            checked_state: Arc::new(Mutex::new(false)),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn checked_state(mut self, state: Arc<Mutex<bool>>) -> Self {
        self.checked_state = state;
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

impl RenderOnce for Toggle {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let checked = *self.checked_state.lock().unwrap();
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let label = props.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());
        let icon_name = props.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());

        let accent = cx.theme().accent;
        let accent_fg = cx.theme().accent_foreground;
        let radius = cx.theme().radius;

        let shared_state = Arc::clone(&self.checked_state);
        let change_handler = self.source.handlers.get("change").cloned();
        let component_id = self.source.id.clone();
        let entity_id = self.entity_id;
        let runtime = self.runtime;

        let id = ElementId::Name(SharedString::from(self.source.id.clone()));

        let mut el = div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .min_w_8()
            .h_8()
            .px_2()
            .rounded(radius)
            .cursor_pointer();

        if checked {
            el = el.bg(accent).text_color(accent_fg);
        } else if !disabled {
            el = el.hover(move |s| s.bg(accent).text_color(accent_fg));
        }

        if let Some(name) = icon_name {
            el = el.child(
                gpui_component::Icon::new(super::icon::map_icon_name(&name))
                    .with_size(gpui_component::Size::Small),
            );
        }

        if let Some(text) = label {
            el = el.child(text);
        }

        if !disabled {
            el = el.on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                let new_checked = !checked;
                *shared_state.lock().unwrap() = new_checked;
                if let Some(ref handler) = change_handler {
                    if let Some(ref rt) = runtime {
                        let data = if new_checked { "true" } else { "false" };
                        rt.call_handler(handler, &component_id, data);
                    }
                }
                if let Some(eid) = entity_id {
                    cx.notify(eid);
                }
            });
        }

        el
    }
}
