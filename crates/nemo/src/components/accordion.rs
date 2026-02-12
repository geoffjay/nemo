use gpui::*;
use gpui_component::accordion::{Accordion as GpuiAccordion, AccordionItem};
use nemo_config::Value;
use nemo_layout::BuiltComponent;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Accordion {
    source: BuiltComponent,
    open_indices: Arc<Mutex<HashSet<usize>>>,
    entity_id: Option<EntityId>,
}

impl Accordion {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            open_indices: Arc::new(Mutex::new(HashSet::new())),
            entity_id: None,
        }
    }

    pub fn open_indices(mut self, indices: Arc<Mutex<HashSet<usize>>>) -> Self {
        self.open_indices = indices;
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let multiple = props
            .get("multiple")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let bordered = props
            .get("bordered")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let current_open = self.open_indices.lock().unwrap().clone();

        let mut accordion = GpuiAccordion::new(SharedString::from(self.source.id.clone()))
            .multiple(multiple)
            .bordered(bordered);

        if let Some(Value::Array(items)) = props.get("items") {
            for (ix, item_val) in items.iter().enumerate() {
                if let Some(obj) = item_val.as_object() {
                    let title = obj
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content = obj
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let open = current_open.contains(&ix);

                    accordion = accordion.item(move |item: AccordionItem| {
                        item.title(title)
                            .open(open)
                            .child(div().p_2().child(content))
                    });
                }
            }
        }

        let shared_state = Arc::clone(&self.open_indices);
        let entity_id = self.entity_id;
        accordion = accordion.on_toggle_click(move |open_indices, _window, cx| {
            let indices: HashSet<usize> = open_indices.iter().copied().collect();
            *shared_state.lock().unwrap() = indices;
            if let Some(eid) = entity_id {
                cx.notify(eid);
            }
        });

        accordion
    }
}
