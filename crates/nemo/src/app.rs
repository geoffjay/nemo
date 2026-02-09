//! GPUI application wrapper.

use gpui::*;
use gpui_component::v_flex;
use std::sync::Arc;

use crate::runtime::NemoRuntime;
use crate::workspace::{DefaultView, HeaderBar};
use nemo_layout::BuiltComponent;

/// The main Nemo GPUI application.
pub struct App {
    runtime: Arc<NemoRuntime>,
    header_bar: Entity<HeaderBar>,
    default_view: Entity<DefaultView>,
    _subscriptions: Vec<Subscription>,
}

impl App {
    /// Creates a new Nemo application.
    pub fn new(runtime: Arc<NemoRuntime>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let header_bar = cx.new(|_cx| HeaderBar::new(window, _cx));
        let default_view = cx.new(|_cx| DefaultView::new(Arc::clone(&runtime), window, _cx));

        let _subscriptions = vec![];

        Self {
            runtime,
            header_bar,
            default_view,
            _subscriptions,
        }
    }

    /// Shutdown the application.
    ///
    /// This is currenty just a placeholder until more shutdown handling is required.
    pub fn shutdown(&mut self, _cx: &mut Context<Self>) {
        ()
    }

    /// Renders the layout from the layout manager.
    fn render_layout(&self, entity_id: EntityId) -> impl IntoElement {
        // Get the layout manager and render the component tree
        let layout_manager = self
            .runtime
            .tokio_runtime
            .block_on(async { self.runtime.layout_manager.read().await });

        // Get the root component
        if let Some(root_id) = layout_manager.root_id() {
            if let Some(root) = layout_manager.get_component(&root_id) {
                return self.render_component(root, &layout_manager, entity_id);
            }
        }

        // Fall back to default view if no layout
        // self.render_default_view()
        self.default_view.clone().into_any_element()
    }

    /// Renders a component and its children recursively.
    fn render_component(
        &self,
        component: &BuiltComponent,
        layout_manager: &tokio::sync::RwLockReadGuard<'_, nemo_layout::LayoutManager>,
        entity_id: EntityId,
    ) -> AnyElement {
        let mut container = match component.component_type.as_str() {
            "stack" => {
                let direction = component
                    .properties
                    .get("direction")
                    .and_then(|v| v.as_str())
                    .unwrap_or("vertical");

                let spacing = component
                    .properties
                    .get("spacing")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(4);

                let base = div().flex().gap(px(spacing as f32));
                if direction == "horizontal" {
                    base.flex_row()
                } else {
                    base.flex_col()
                }
            }
            "panel" => div().flex().flex_col().p_4().rounded_md().bg(rgb(0x313244)),
            "label" => {
                let text = component
                    .properties
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return div().child(text.to_string()).into_any_element();
            }
            "button" => {
                let label = component
                    .properties
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Button");

                let width = component.properties.get("width").and_then(|v| v.as_i64());
                let height = component.properties.get("height").and_then(|v| v.as_i64());
                let flex = component.properties.get("flex").and_then(|v| v.as_f64());

                let mut button = div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xb4befe)))
                    .items_center()
                    .justify_center()
                    .child(label.to_string());

                // Apply sizing
                if let Some(w) = width {
                    button = button.w(px(w as f32));
                }
                if let Some(h) = height {
                    button = button.h(px(h as f32));
                }
                if let Some(f) = flex {
                    button = button.flex_grow().flex_basis(relative(f as f32));
                } else if width.is_none() {
                    // Default: buttons grow equally in horizontal stacks
                    button = button.flex_1();
                }

                // Wire up click handler if present
                if let Some(handler) = component.handlers.get("click") {
                    let runtime = Arc::clone(&self.runtime);
                    let component_id = component.id.clone();
                    let handler = handler.clone();

                    button = button.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        runtime.call_handler(&handler, &component_id, "click");
                        // Trigger re-render to reflect any state changes from the handler
                        cx.notify(entity_id);
                    });
                }

                return button.into_any_element();
            }
            "text" => {
                let content = component
                    .properties
                    .get("content")
                    .or_else(|| component.properties.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return div().child(content.to_string()).into_any_element();
            }
            _ => div().flex().flex_col(),
        };

        // Render children
        for child_id in &component.children {
            if let Some(child) = layout_manager.get_component(child_id) {
                container =
                    container.child(self.render_component(child, layout_manager, entity_id));
            }
        }

        container.into_any_element()
    }
}

impl Render for App {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get theme colors from config or use defaults
        let bg_color = self
            .runtime
            .get_config("app.theme.background")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| parse_hex_color(&s))
            .unwrap_or_else(|| rgb(0x1e1e2e).into());

        let text_color = self
            .runtime
            .get_config("app.theme.foreground")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| parse_hex_color(&s))
            .unwrap_or_else(|| rgb(0xcdd6f4).into());

        // Get the entity ID for re-render notifications
        let entity_id = cx.entity_id();

        v_flex()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .child(self.header_bar.clone())
            .child(self.render_layout(entity_id))
    }
}

/// Parses a hex color string to a Hsla color.
fn parse_hex_color(s: &str) -> Option<Hsla> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }

    let hex = u32::from_str_radix(s, 16).ok()?;
    Some(rgb(hex).into())
}
