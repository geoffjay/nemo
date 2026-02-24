use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::tab::{Tab as GpuiTab, TabBar, TabVariant};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

/// A tabbed container component for organizing content into switchable panels.
///
/// # XML Configuration
///
/// ```xml
/// <tabs id="settings" tabs='[{"label":"General"},{"label":"Advanced"}]' variant="pills" active-tab="0">
///   <!-- tab panel children -->
/// </tabs>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `tabs` | JSON array | Array of tab objects with `label` fields |
/// | `variant` | string | Tab style variant |
/// | `active-tab` | int | Index of the initially active tab |
#[derive(IntoElement)]
pub struct Tabs {
    source: BuiltComponent,
    children: Vec<AnyElement>,
    selected_index: Arc<Mutex<Option<usize>>>,
    entity_id: Option<EntityId>,
}

impl Tabs {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            children: Vec::new(),
            selected_index: Arc::new(Mutex::new(Some(0))),
            entity_id: None,
        }
    }

    pub fn selected_index(mut self, state: Arc<Mutex<Option<usize>>>) -> Self {
        self.selected_index = state;
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

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;

        // Read tab labels from the `tabs` property
        let tab_labels: Vec<String> = props
            .get("tabs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Read variant
        let variant = match props.get("variant").and_then(|v| v.as_str()) {
            Some("pill") => TabVariant::Pill,
            Some("segmented") => TabVariant::Segmented,
            Some("outline") => TabVariant::Outline,
            Some("tab") => TabVariant::Tab,
            // Default to underline
            _ => TabVariant::Underline,
        };

        // Current selected index
        let selected = self.selected_index.lock().unwrap().unwrap_or(0);

        let tab_bar_id = ElementId::Name(SharedString::from(format!("{}-tabbar", self.source.id)));

        // Build the tab bar
        let shared_state = Arc::clone(&self.selected_index);
        let entity_id = self.entity_id;

        let tab_bar = TabBar::new(tab_bar_id)
            .with_variant(variant)
            .selected_index(selected)
            .children(
                tab_labels
                    .iter()
                    .map(|label| GpuiTab::new().label(label.clone())),
            )
            .on_click(move |index, _window, cx| {
                let mut state = shared_state.lock().unwrap();
                *state = Some(*index);
                if let Some(eid) = entity_id {
                    cx.notify(eid);
                }
            });

        // Show only the child at the selected index
        let child_count = self.children.len();
        let mut children_vec = self.children;

        div()
            .flex()
            .flex_col()
            .w_full()
            .child(tab_bar)
            .when(selected < child_count, |this| {
                // We need to extract the selected child. Since children_vec is a Vec,
                // swap_remove the selected element to take ownership.
                let child = children_vec.swap_remove(selected);
                this.child(div().pt_2().child(child))
            })
    }
}
