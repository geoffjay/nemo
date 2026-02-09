//! GPUI application wrapper.

use gpui::*;
use gpui_component::v_flex;
use std::sync::Arc;

use crate::components::{Button, Label, Panel, Stack, Text};
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
        let title = runtime
            .get_config("app.window.title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Nemo Application".to_string());
        let header_bar = cx.new(|_cx| HeaderBar::new(title, window, _cx));
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

    /// Renders the children of a component.
    fn render_children(
        &self,
        component: &BuiltComponent,
        layout_manager: &tokio::sync::RwLockReadGuard<'_, nemo_layout::LayoutManager>,
        entity_id: EntityId,
    ) -> Vec<AnyElement> {
        component
            .children
            .iter()
            .filter_map(|child_id| layout_manager.get_component(child_id))
            .map(|child| self.render_component(child, layout_manager, entity_id))
            .collect()
    }

    /// Renders a component and its children recursively.
    fn render_component(
        &self,
        component: &BuiltComponent,
        layout_manager: &tokio::sync::RwLockReadGuard<'_, nemo_layout::LayoutManager>,
        entity_id: EntityId,
    ) -> AnyElement {
        match component.component_type.as_str() {
            "stack" => {
                let children = self.render_children(component, layout_manager, entity_id);
                Stack::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "panel" => {
                let children = self.render_children(component, layout_manager, entity_id);
                Panel::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "label" => Label::new(component.clone()).into_any_element(),
            "button" => Button::new(component.clone())
                .runtime(Arc::clone(&self.runtime))
                .entity_id(entity_id)
                .into_any_element(),
            "text" => Text::new(component.clone()).into_any_element(),
            _ => {
                let children = self.render_children(component, layout_manager, entity_id);
                div().flex().flex_col().children(children).into_any_element()
            }
        }
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
