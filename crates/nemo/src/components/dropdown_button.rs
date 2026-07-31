use gpui::*;
use gpui_component::button::{
    Button as GpuiButton, ButtonVariants, DropdownButton as GpuiDropdownButton,
};
use gpui_component::menu::PopupMenuItem;
use nemo_layout::BuiltComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

/// A single menu entry carried through from a `<menu-item>` child.
#[derive(Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub on_click: Option<String>,
}

/// A button with a dropdown menu component.
///
/// # XML Configuration
///
/// ```xml
/// <dropdown-button id="actions" label="Actions" variant="primary">
///   <menu-item label="Edit" on-click="edit" />
///   <menu-item label="Delete" on-click="delete" />
/// </dropdown-button>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `label` | string | Button text label |
/// | `variant` | string | Button style variant |
///
/// Menu entries are declared as `<menu-item>` children (`label`, optional
/// `on-click` handler invoked with the usual `(component_id, event_data)`).
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct DropdownButton {
    source: BuiltComponent,
    items: Vec<MenuItem>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl DropdownButton {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            items: Vec::new(),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
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

impl RenderOnce for DropdownButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let label = props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Action")
            .to_string();
        let variant = props.get("variant").and_then(|v| v.as_str()).unwrap_or("");

        let id = SharedString::from(self.source.id.clone());
        let btn_id = SharedString::from(format!("{}-btn", self.source.id));

        let mut button = GpuiButton::new(btn_id).label(SharedString::from(label));
        button = match variant {
            "primary" => button.primary(),
            "danger" => button.danger(),
            "ghost" => button.ghost(),
            _ => button,
        };

        let menu_items = self.items;
        let runtime = self.runtime;
        let entity_id = self.entity_id;

        let mut dropdown = GpuiDropdownButton::new(id).button(button);

        if !menu_items.is_empty() {
            dropdown = dropdown.dropdown_menu(move |menu, _window, _cx| {
                let mut m = menu;
                for item in &menu_items {
                    let mut menu_item = PopupMenuItem::new(SharedString::from(item.label.clone()));
                    if let (Some(handler), Some(runtime), Some(entity_id)) =
                        (item.on_click.clone(), runtime.clone(), entity_id)
                    {
                        let item_id = item.id.clone();
                        menu_item = menu_item.on_click(move |_event, _window, cx| {
                            runtime.call_handler(&handler, &item_id, "click");
                            cx.notify(entity_id);
                        });
                    }
                    m = m.item(menu_item);
                }
                m
            });
        }

        dropdown
    }
}
