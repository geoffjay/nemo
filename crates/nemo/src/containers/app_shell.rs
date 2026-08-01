use gpui::*;
use gpui_component::{ActiveTheme, Sizable};
use nemo_layout::BuiltComponent;
use std::sync::{Arc, Mutex};

use crate::components::icon::map_icon_name;
use crate::runtime::NemoRuntime;
use crate::theme::tokens::{radius_of, FontSize, Space, TokenStyled};

/// A standard application shell: a left sidenav of icon+label items, a
/// switchable content area, and a full-width status footer.
///
/// `AppShell` packages a common app layout as a single container so authors
/// don't hand-assemble stacks, sidebars, and page-toggle handlers. Clicking a
/// `<sidenav-item target="pageX">` shows the matching `<page id="pageX">` and
/// highlights the active item — page switching is built in and needs no script.
///
/// # XML Configuration
///
/// ```xml
/// <app-shell sidenav-width="200">
///   <app-sidenav>
///     <sidenav-item icon="layout-dashboard" label="Overview" target="overview"/>
///     <sidenav-item icon="chart-pie"        label="Reports"  target="reports"/>
///   </app-sidenav>
///   <app-content>
///     <page id="overview"><!-- ... --></page>
///     <page id="reports"><!-- ... --></page>
///   </app-content>
///   <app-footer>
///     <stack direction="horizontal"><label text="Ready"/></stack>
///   </app-footer>
/// </app-shell>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `sidenav-width` | int | Sidenav column width in pixels (default: 200) |
/// | `collapsed` | bool | Show only icons in the sidenav when true |
#[derive(IntoElement)]
pub struct AppShell {
    source: BuiltComponent,
    /// Raw `sidenav_item` BuiltComponents; rendered by the shell itself.
    sidenav_items: Vec<BuiltComponent>,
    /// Pre-rendered body of the currently active page.
    content_children: Vec<AnyElement>,
    /// Pre-rendered footer children.
    footer_children: Vec<AnyElement>,
    /// Shared active-page target (matches a `<page id="...">`).
    active_state: Arc<Mutex<String>>,
    entity_id: Option<EntityId>,
    runtime: Option<Arc<NemoRuntime>>,
}

impl AppShell {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            sidenav_items: Vec::new(),
            content_children: Vec::new(),
            footer_children: Vec::new(),
            active_state: Arc::new(Mutex::new(String::new())),
            entity_id: None,
            runtime: None,
        }
    }

    pub fn sidenav_items(mut self, items: Vec<BuiltComponent>) -> Self {
        self.sidenav_items = items;
        self
    }

    pub fn content_children(mut self, children: Vec<AnyElement>) -> Self {
        self.content_children = children;
        self
    }

    pub fn footer_children(mut self, children: Vec<AnyElement>) -> Self {
        self.footer_children = children;
        self
    }

    pub fn active_state(mut self, state: Arc<Mutex<String>>) -> Self {
        self.active_state = state;
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }
}

impl RenderOnce for AppShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let sidebar_bg = colors.sidebar;
        let sidebar_border = colors.sidebar_border;
        let sidebar_fg = colors.sidebar_foreground;
        let hover_bg = colors.list_hover;
        let active_bg = colors.list_active;
        let border = colors.border;

        let props = &self.source.properties;
        let sidenav_width = props
            .get("sidenav_width")
            .and_then(|v| v.as_i64())
            .unwrap_or(200) as f32;
        let collapsed = props
            .get("collapsed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let active = self.active_state.lock().unwrap().clone();
        let item_radius = radius_of("md", cx);

        // ── Sidenav column ────────────────────────────────────────────────
        let items: Vec<AnyElement> = self
            .sidenav_items
            .iter()
            .map(|item| {
                let icon = item
                    .properties
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .unwrap_or("info");
                let label = item
                    .properties
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let target = item
                    .properties
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let handler = item.handlers.get("click").cloned();
                let is_active = !target.is_empty() && target == active;

                let mut row = div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .rounded(item_radius)
                    .mx_t(Space::Xs)
                    .cursor_pointer()
                    .text_color(sidebar_fg)
                    .hover(move |s| s.bg(hover_bg));

                if is_active {
                    row = row.bg(active_bg);
                }

                if collapsed {
                    row = row.justify_center().size(px(40.));
                    row = row.child(
                        gpui_component::Icon::new(map_icon_name(icon))
                            .with_size(gpui_component::Size::Small),
                    );
                } else {
                    row = row.px_t(Space::Sm).py_t(Space::Xs).gap_t(Space::Md);
                    row = row.child(
                        gpui_component::Icon::new(map_icon_name(icon))
                            .with_size(gpui_component::Size::Small),
                    );
                    row = row.child(div().text_t(FontSize::Sm).child(label));
                }

                // Clicking an item selects its target page (built-in switching)
                // and, if present, also invokes the author's on-click handler.
                if let (Some(entity_id), false) = (self.entity_id, target.is_empty()) {
                    let active_state = Arc::clone(&self.active_state);
                    let runtime = self.runtime.clone();
                    let item_id = item.id.clone();
                    let target = target.clone();
                    row = row.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        *active_state.lock().unwrap() = target.clone();
                        if let (Some(runtime), Some(handler)) = (&runtime, &handler) {
                            runtime.call_handler(handler, &item_id, "click");
                        }
                        cx.notify(entity_id);
                    });
                }

                row.into_any_element()
            })
            .collect();

        let sidenav = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .h_full()
            .w(px(if collapsed { 48.0 } else { sidenav_width }))
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(sidebar_border)
            .py_t(Space::Sm)
            .gap_t(Space::Xs)
            .children(items);

        // ── Content region ────────────────────────────────────────────────
        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .overflow_hidden()
            .children(self.content_children);

        // ── Footer ────────────────────────────────────────────────────────
        let footer = div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .flex_shrink_0()
            .min_h(px(32.))
            .border_t_1()
            .border_color(border)
            .px_t(Space::Sm)
            .children(self.footer_children);

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h(px(0.))
                    .items_stretch()
                    .child(sidenav)
                    .child(content),
            )
            .child(footer)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use crate::components::state::ComponentStates;

    /// Selecting the active page: mirrors the resolution logic in the
    /// `app_shell` render arm — the active target defaults to the first page's
    /// id, and a stored value that matches no page falls back to the first.
    fn resolve_active<'a>(page_ids: &'a [&str], stored: Option<&str>) -> Option<&'a str> {
        let default_target = page_ids.first().copied().unwrap_or("");
        let active = stored.unwrap_or(default_target);
        page_ids
            .iter()
            .copied()
            .find(|id| *id == active)
            .or_else(|| page_ids.first().copied())
    }

    #[test]
    fn active_page_defaults_to_first() {
        let pages = ["overview", "reports", "settings"];
        assert_eq!(resolve_active(&pages, None), Some("overview"));
    }

    #[test]
    fn active_page_selects_matching_target() {
        let pages = ["overview", "reports", "settings"];
        assert_eq!(resolve_active(&pages, Some("reports")), Some("reports"));
    }

    #[test]
    fn active_page_falls_back_when_target_unknown() {
        let pages = ["overview", "reports"];
        assert_eq!(resolve_active(&pages, Some("missing")), Some("overview"));
    }

    #[test]
    fn active_state_persists_selection_across_renders() {
        // get_or_create_selected_value sets the initial only once; a later
        // click stores a value that subsequent renders must observe.
        let mut states = ComponentStates::new();
        let state = states.get_or_create_selected_value("shell", "overview".to_string());
        *state.lock().unwrap() = "reports".to_string();
        // A second render re-fetches with the same default and sees the click.
        let again = states.get_or_create_selected_value("shell", "overview".to_string());
        assert_eq!(*again.lock().unwrap(), "reports");
    }
}
