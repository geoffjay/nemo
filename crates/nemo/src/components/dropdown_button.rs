use gpui::*;
use gpui_component::button::{
    Button as GpuiButton, ButtonVariants, DropdownButton as GpuiDropdownButton,
};
use gpui_component::menu::PopupMenuItem;
use nemo_config::Value;
use nemo_layout::BuiltComponent;

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct DropdownButton {
    source: BuiltComponent,
}

impl DropdownButton {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
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

        // Parse menu items from the "items" property
        let menu_items: Vec<String> = match props.get("items") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => Vec::new(),
        };

        let mut dropdown = GpuiDropdownButton::new(id).button(button);

        if !menu_items.is_empty() {
            dropdown = dropdown.dropdown_menu(move |menu, _window, _cx| {
                let mut m = menu;
                for item_label in &menu_items {
                    m = m.item(PopupMenuItem::new(SharedString::from(item_label.clone())));
                }
                m
            });
        }

        dropdown
    }
}
