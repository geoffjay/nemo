use gpui::*;
use gpui_component::switch::Switch as GpuiSwitch;
use gpui_component::Disableable;
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use crate::runtime::NemoRuntime;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Switch {
    source: BuiltComponent,
    checked_state: Arc<Mutex<bool>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Switch {
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

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let checked = *self.checked_state.lock().unwrap();
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut sw = GpuiSwitch::new(SharedString::from(self.source.id.clone())).checked(checked);

        if let Some(label) = props.get("label").and_then(|v| v.as_str()) {
            sw = sw.label(label.to_string());
        }

        if disabled {
            sw = sw.disabled(true);
        }

        let shared_state = Arc::clone(&self.checked_state);
        let change_handler = self.source.handlers.get("change").cloned();
        let component_id = self.source.id.clone();
        let entity_id = self.entity_id;
        let runtime = self.runtime;

        sw = sw.on_click(move |new_checked, _window, cx| {
            *shared_state.lock().unwrap() = *new_checked;
            if let Some(ref handler) = change_handler {
                if let Some(ref rt) = runtime {
                    let data = if *new_checked { "true" } else { "false" };
                    rt.call_handler(handler, &component_id, data);
                }
            }
            if let Some(eid) = entity_id {
                cx.notify(eid);
            }
        });

        sw
    }
}
