use gpui::*;
use gpui_component::radio::{Radio as GpuiRadio, RadioGroup};
use nemo_config::Value;
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use crate::runtime::NemoRuntime;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Radio {
    source: BuiltComponent,
    selected_index: Arc<Mutex<Option<usize>>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Radio {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            selected_index: Arc::new(Mutex::new(None)),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn selected_index(mut self, state: Arc<Mutex<Option<usize>>>) -> Self {
        self.selected_index = state;
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

impl RenderOnce for Radio {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let horizontal = props
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("vertical")
            == "horizontal";

        let options: Vec<String> = match props.get("options") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => Vec::new(),
        };

        let current_index = *self.selected_index.lock().unwrap();

        let id = SharedString::from(self.source.id.clone());
        let mut group = if horizontal {
            RadioGroup::horizontal(id)
        } else {
            RadioGroup::vertical(id)
        };

        group = group.selected_index(current_index);

        for (i, option) in options.iter().enumerate() {
            let radio_id = SharedString::from(format!("{}-{}", self.source.id, i));
            group = group.child(GpuiRadio::new(radio_id).label(option.clone()));
        }

        let shared_state = Arc::clone(&self.selected_index);
        let change_handler = self.source.handlers.get("change").cloned();
        let component_id = self.source.id.clone();
        let entity_id = self.entity_id;
        let runtime = self.runtime;
        let opts = options.clone();

        group = group.on_click(move |selected_ix, _window, cx| {
            *shared_state.lock().unwrap() = Some(*selected_ix);
            if let Some(ref handler) = change_handler {
                if let Some(ref rt) = runtime {
                    let value = opts.get(*selected_ix).map(|s| s.as_str()).unwrap_or("");
                    rt.call_handler(handler, &component_id, value);
                }
            }
            if let Some(eid) = entity_id {
                cx.notify(eid);
            }
        });

        group
    }
}
