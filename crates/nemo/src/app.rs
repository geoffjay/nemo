//! GPUI application wrapper.

use gpui::*;
use gpui_component::input::InputState;
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use std::sync::Arc;

use crate::components::state::{ComponentState, ComponentStates};
use crate::components::{
    Button, Checkbox, Icon, Image, Label, List, Modal, Notification, Panel, Progress, Select,
    Stack, Table, Tabs, Text, Tooltip, Tree,
};
use crate::runtime::NemoRuntime;
use crate::workspace::{DefaultView, HeaderBar};
use nemo_layout::BuiltComponent;

/// The main Nemo GPUI application.
pub struct App {
    runtime: Arc<NemoRuntime>,
    header_bar: Entity<HeaderBar>,
    default_view: Entity<DefaultView>,
    component_states: ComponentStates,
    _subscriptions: Vec<Subscription>,
}

impl App {
    /// Creates a new Nemo application.
    pub fn new(runtime: Arc<NemoRuntime>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title = runtime
            .get_config("app.window.title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Nemo Application".to_string());
        let github_url = runtime
            .get_config("app.window.header_bar.github_url")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let theme_toggle = runtime
            .get_config("app.window.header_bar.theme_toggle")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let header_bar =
            cx.new(|_cx| HeaderBar::new(title, github_url, theme_toggle, window, _cx));
        let default_view = cx.new(|_cx| DefaultView::new(Arc::clone(&runtime), window, _cx));

        let _subscriptions = vec![];

        Self {
            runtime,
            header_bar,
            default_view,
            component_states: ComponentStates::new(),
            _subscriptions,
        }
    }

    /// Shutdown the application.
    ///
    /// This is currenty just a placeholder until more shutdown handling is required.
    pub fn shutdown(&mut self, _cx: &mut Context<Self>) {
        ()
    }

    /// Gets or creates an InputState entity for the given component id.
    fn get_or_create_input_state(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(ComponentState::Input(state)) = self.component_states.get(id) {
            return state.clone();
        }

        let state = cx.new(|cx| InputState::new(window, cx));
        self.component_states
            .insert(id.to_string(), ComponentState::Input(state.clone()));
        state
    }

    /// Renders the layout from the layout manager.
    fn render_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let entity_id = cx.entity_id();

        // Get the layout manager and render the component tree
        let layout_manager = self
            .runtime
            .tokio_runtime
            .block_on(async { self.runtime.layout_manager.read().await });

        // Get the root component
        if let Some(root_id) = layout_manager.root_id() {
            if let Some(root) = layout_manager.get_component(&root_id) {
                let root = root.clone();
                drop(layout_manager);
                return self.render_component(&root, entity_id, window, cx);
            }
        }

        drop(layout_manager);
        self.default_view.clone().into_any_element()
    }

    /// Renders the children of a component.
    fn render_children(
        &mut self,
        component: &BuiltComponent,
        entity_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let layout_manager = self
            .runtime
            .tokio_runtime
            .block_on(async { self.runtime.layout_manager.read().await });

        let children: Vec<BuiltComponent> = component
            .children
            .iter()
            .filter_map(|child_id| layout_manager.get_component(child_id).cloned())
            .collect();
        drop(layout_manager);

        children
            .iter()
            .map(|child| self.render_component(child, entity_id, window, cx))
            .collect()
    }

    /// Wraps an element in a styled div if layout properties are present.
    fn apply_sizing(element: AnyElement, component: &BuiltComponent) -> AnyElement {
        let props = &component.properties;
        let width = props.get("width").and_then(|v| v.as_i64());
        let height = props.get("height").and_then(|v| v.as_i64());
        let min_width = props.get("min_width").and_then(|v| v.as_i64());
        let min_height = props.get("min_height").and_then(|v| v.as_i64());
        let flex = props
            .get("flex")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));
        let padding = props.get("padding").and_then(|v| v.as_i64());
        let margin = props.get("margin").and_then(|v| v.as_i64());

        if width.is_none()
            && height.is_none()
            && min_width.is_none()
            && min_height.is_none()
            && flex.is_none()
            && padding.is_none()
            && margin.is_none()
        {
            return element;
        }

        let mut wrapper = div().flex().flex_col();
        if let Some(w) = width {
            wrapper = wrapper.w(px(w as f32));
        }
        if let Some(h) = height {
            wrapper = wrapper.h(px(h as f32));
        }
        if let Some(mw) = min_width {
            wrapper = wrapper.min_w(px(mw as f32));
        }
        if let Some(mh) = min_height {
            wrapper = wrapper.min_h(px(mh as f32));
        }
        if flex.is_some() {
            wrapper = wrapper.flex_1();
        }
        if let Some(p) = padding {
            wrapper = wrapper.p(px(p as f32));
        }
        if let Some(m) = margin {
            wrapper = wrapper.m(px(m as f32));
        }
        wrapper.child(element).into_any_element()
    }

    /// Renders a component and its children recursively.
    fn render_component(
        &mut self,
        component: &BuiltComponent,
        entity_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let element = match component.component_type.as_str() {
            "stack" => {
                let children = self.render_children(component, entity_id, window, cx);
                Stack::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "panel" => {
                let children = self.render_children(component, entity_id, window, cx);
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
            "icon" => Icon::new(component.clone()).into_any_element(),
            "checkbox" => Checkbox::new(component.clone())
                .runtime(Arc::clone(&self.runtime))
                .entity_id(entity_id)
                .into_any_element(),
            "input" => {
                let input_state =
                    self.get_or_create_input_state(&component.id, window, cx);
                crate::components::Input::new(component.clone())
                    .input_state(input_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "select" => Select::new(component.clone())
                .runtime(Arc::clone(&self.runtime))
                .entity_id(entity_id)
                .into_any_element(),
            "progress" => Progress::new(component.clone()).into_any_element(),
            "image" => Image::new(component.clone()).into_any_element(),
            "notification" => Notification::new(component.clone()).into_any_element(),
            "tabs" => {
                let children = self.render_children(component, entity_id, window, cx);
                Tabs::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "modal" => {
                let children = self.render_children(component, entity_id, window, cx);
                Modal::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "tooltip" => {
                let children = self.render_children(component, entity_id, window, cx);
                Tooltip::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "table" => Table::new(component.clone()).into_any_element(),
            "list" => List::new(component.clone()).into_any_element(),
            "tree" => Tree::new(component.clone()).into_any_element(),
            _ => {
                let children = self.render_children(component, entity_id, window, cx);
                div().flex().flex_col().children(children).into_any_element()
            }
        };
        Self::apply_sizing(element, component)
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_color = cx.theme().colors.background;
        let text_color = cx.theme().colors.foreground;

        v_flex()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .child(self.header_bar.clone())
            .child(self.render_layout(window, cx))
    }
}
