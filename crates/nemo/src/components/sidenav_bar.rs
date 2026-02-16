use gpui::*;
use gpui_component::{ActiveTheme, Sizable};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use super::icon::map_icon_name;

/// A vertical navigation sidebar that displays icon+label items.
///
/// When `collapsed = true`, only icons are shown. When `collapsed = false`,
/// icons and labels are shown side by side. Has a 1px border on left and right
/// by default.
#[derive(IntoElement)]
pub struct SidenavBar {
    source: BuiltComponent,
    collapsed_state: Arc<Mutex<bool>>,
    /// Pre-rendered child elements (SidenavBarItem instances rendered by the app).
    children: Vec<AnyElement>,
    /// Raw child BuiltComponents so we can render SidenavBarItems ourselves.
    child_components: Vec<BuiltComponent>,
    entity_id: Option<EntityId>,
}

impl SidenavBar {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            collapsed_state: Arc::new(Mutex::new(false)),
            children: Vec::new(),
            child_components: Vec::new(),
            entity_id: None,
        }
    }

    pub fn collapsed_state(mut self, state: Arc<Mutex<bool>>) -> Self {
        self.collapsed_state = state;
        self
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }

    pub fn child_components(mut self, components: Vec<BuiltComponent>) -> Self {
        self.child_components = components;
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for SidenavBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let collapsed = *self.collapsed_state.lock().unwrap();
        let border_color = cx.theme().colors.sidebar_border;
        let bg = cx.theme().colors.sidebar;

        let expanded_width = self
            .source
            .properties
            .get("width")
            .and_then(|v| v.as_i64())
            .unwrap_or(200) as f32;
        let collapsed_width: f32 = 48.0;
        let current_width = if collapsed {
            collapsed_width
        } else {
            expanded_width
        };

        let mut container = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .h_full()
            .w(px(current_width))
            .bg(bg)
            .border_x_1()
            .border_color(border_color)
            .py_2();

        // Render sidenav_bar_item children from their BuiltComponent data
        let items: Vec<AnyElement> = self
            .child_components
            .iter()
            .filter(|c| c.component_type == "sidenav_bar_item")
            .map(|child| {
                SidenavBarItem::from_built_component(child, collapsed)
                    .render(_window, cx)
                    .into_any_element()
            })
            .collect();

        // Non-sidenav_bar_item children (e.g. buttons) are passed through as-is
        container = container.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap_1()
                .children(items),
        );

        // Append any non-item children (like a toggle button) at the bottom
        if !self.children.is_empty() {
            container = container.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.children),
            );
        }

        container.into_any_element()
    }
}

/// A single item in a SidenavBar, showing an icon and optionally a label.
#[derive(IntoElement)]
pub struct SidenavBarItem {
    icon: String,
    label: String,
    collapsed: bool,
}

impl SidenavBarItem {
    pub fn new(icon: String, label: String, collapsed: bool) -> Self {
        Self {
            icon,
            label,
            collapsed,
        }
    }

    /// Build a SidenavBarItem from a BuiltComponent's properties.
    pub fn from_built_component(component: &BuiltComponent, collapsed: bool) -> Self {
        let icon = component
            .properties
            .get("icon")
            .and_then(|v| v.as_str())
            .unwrap_or("info")
            .to_string();
        let label = component
            .properties
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Self::new(icon, label, collapsed)
    }
}

impl RenderOnce for SidenavBarItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let icon_name = map_icon_name(&self.icon);
        let fg = cx.theme().colors.sidebar_foreground;
        let hover_bg = cx.theme().colors.list_hover;

        let mut row = div()
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .py_1()
            .mx_1()
            .rounded_md()
            .cursor_pointer()
            .text_color(fg)
            .hover(move |s| s.bg(hover_bg));

        if self.collapsed {
            // Icon only, centered
            row = row.justify_center();
            row = row.child(
                gpui_component::Icon::new(icon_name)
                    .with_size(gpui_component::Size::Small),
            );
        } else {
            // Icon + label
            row = row.gap_3();
            row = row.child(
                gpui_component::Icon::new(icon_name)
                    .with_size(gpui_component::Size::Small),
            );
            row = row.child(
                div()
                    .text_sm()
                    .child(self.label),
            );
        }

        row
    }
}
