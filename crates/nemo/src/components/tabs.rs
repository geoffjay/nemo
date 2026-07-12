use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::tab::{Tab as GpuiTab, TabBar, TabVariant};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

/// A single tab, built by the render dispatch from a `<tab-item>` child.
/// `label` is the tab-bar text; `body` holds the item's rendered children (the
/// panel content shown when the tab is active).
pub struct TabItemData {
    pub label: String,
    pub body: Vec<AnyElement>,
}

/// A tabbed container component for organizing content into switchable panels.
///
/// # XML Configuration
///
/// ```xml
/// <tabs id="settings" variant="pill" active-tab="0">
///   <tab-item label="General"><label text="General settings" /></tab-item>
///   <tab-item label="Advanced"><label text="Advanced settings" /></tab-item>
/// </tabs>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `variant` | string | Tab style variant |
/// | `active-tab` | int | Index of the initially active tab |
///
/// Tabs are declared as `<tab-item>` children (`label` plus body children).
#[derive(IntoElement)]
pub struct Tabs {
    source: BuiltComponent,
    items: Vec<TabItemData>,
    selected_index: Arc<Mutex<Option<usize>>>,
    entity_id: Option<EntityId>,
}

impl Tabs {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            items: Vec::new(),
            selected_index: Arc::new(Mutex::new(Some(0))),
            entity_id: None,
        }
    }

    pub fn items(mut self, items: Vec<TabItemData>) -> Self {
        self.items = items;
        self
    }

    pub fn selected_index(mut self, state: Arc<Mutex<Option<usize>>>) -> Self {
        self.selected_index = state;
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

        // Build the tab bar from item labels
        let shared_state = Arc::clone(&self.selected_index);
        let entity_id = self.entity_id;

        let tab_bar = TabBar::new(tab_bar_id)
            .with_variant(variant)
            .selected_index(selected)
            .children(
                self.items
                    .iter()
                    .map(|item| GpuiTab::new().label(item.label.clone())),
            )
            .on_click(move |index, _window, cx| {
                let mut state = shared_state.lock().unwrap();
                *state = Some(*index);
                if let Some(eid) = entity_id {
                    cx.notify(eid);
                }
            });

        // Show only the body of the selected item
        let item_count = self.items.len();
        let mut items = self.items;

        div()
            .flex()
            .flex_col()
            .w_full()
            .child(tab_bar)
            .when(selected < item_count, |this| {
                // Take ownership of the selected item's body.
                let body = items.swap_remove(selected).body;
                this.child(div().pt_2().children(body))
            })
    }
}
