use crate::theme::tokens::{Space, TokenStyled};
use gpui::*;
use gpui_component::accordion::{Accordion as GpuiAccordion, AccordionItem};
use nemo_layout::BuiltComponent;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// A single accordion section, built by the render dispatch from an
/// `<accordion-item>` child component. `body` holds the item's rendered
/// children (the panel content).
pub struct AccordionItemData {
    pub title: String,
    pub body: Vec<AnyElement>,
}

/// An expandable accordion component.
///
/// # XML Configuration
///
/// ```xml
/// <accordion id="faq" multiple="true" bordered="true">
///   <accordion-item title="Question 1">
///     <label text="Answer 1" />
///   </accordion-item>
///   <accordion-item title="Question 2" open="true">
///     <button label="Do it" on-click="go" />
///   </accordion-item>
/// </accordion>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `multiple` | bool | Allow multiple sections open simultaneously |
/// | `bordered` | bool | Show borders around sections |
///
/// Sections are declared as `<accordion-item>` children. Each item takes a
/// `title` (string) and an optional `open` (bool); its own children form the
/// panel body.
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Accordion {
    source: BuiltComponent,
    items: Vec<AccordionItemData>,
    open_indices: Arc<Mutex<HashSet<usize>>>,
    entity_id: Option<EntityId>,
}

impl Accordion {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            items: Vec::new(),
            open_indices: Arc::new(Mutex::new(HashSet::new())),
            entity_id: None,
        }
    }

    pub fn items(mut self, items: Vec<AccordionItemData>) -> Self {
        self.items = items;
        self
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

        for (ix, item_data) in self.items.into_iter().enumerate() {
            let title = item_data.title;
            let body = item_data.body;
            let open = current_open.contains(&ix);

            accordion = accordion.item(move |item: AccordionItem| {
                item.title(title)
                    .open(open)
                    .child(div().p_t(Space::Sm).children(body))
            });
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
