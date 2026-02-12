//! GPUI application wrapper.

use gpui::*;
use gpui_component::input::InputState;
use gpui_component::slider::SliderState;
use gpui_component::table::TableState;
use gpui_component::tree::TreeState;
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use nemo_config::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::components::state::{ComponentState, ComponentStates};
use crate::components::table::NemoTableDelegate;
use crate::components::tree::values_to_tree_items;
use crate::components::{
    Accordion, Alert, AreaChart, Avatar, Badge, BarChart, Button, CandlestickChart, Checkbox,
    Collapsible, DropdownButton, Icon, Image, Label, LineChart, List, Modal, Notification, Panel,
    PieChart, Progress, Radio, Select, Slider, Spinner, Stack, Switch, Table, Tabs, Tag, Text,
    Toggle, Tooltip, Tree,
};
use crate::runtime::NemoRuntime;
use crate::workspace::HeaderBar;
use nemo_layout::BuiltComponent;

/// The main Nemo GPUI application.
pub struct App {
    runtime: Arc<NemoRuntime>,
    header_bar: Entity<HeaderBar>,
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
        let header_bar = cx.new(|_cx| HeaderBar::new(title, github_url, theme_toggle, window, _cx));

        // Wait for data_notify signals and apply updates when data arrives.
        let poll_runtime = Arc::clone(&runtime);
        let data_notify = Arc::clone(&runtime.data_notify);
        let _data_task = cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| loop {
            data_notify.notified().await;
            if poll_runtime.apply_pending_data_updates() {
                let _ = this.update(cx, |_app: &mut App, cx: &mut Context<App>| {
                    cx.notify();
                });
            }
        });
        _data_task.detach();

        let _subscriptions = vec![];

        Self {
            runtime,
            header_bar,
            component_states: ComponentStates::new(),
            _subscriptions,
        }
    }

    /// Shutdown the application and its runtime.
    pub fn shutdown(&mut self, _cx: &mut Context<Self>) {
        self.runtime.shutdown();
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

    /// Gets or creates a TableState entity for the given component.
    fn get_or_create_table_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TableState<NemoTableDelegate>> {
        let current_data = match component.properties.get("data") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };

        if let Some(ComponentState::Table { state, last_data }) =
            self.component_states.get_mut(&component.id)
        {
            if *last_data != current_data {
                let new_data = current_data.clone();
                state.update(cx, |s, cx| {
                    s.delegate_mut().set_rows(new_data);
                    s.refresh(cx);
                });
                *last_data = current_data;
            }
            return state.clone();
        }

        let delegate = NemoTableDelegate::from_properties(&component.properties);
        let state = cx.new(|cx| TableState::new(delegate, window, cx));
        self.component_states.insert(
            component.id.clone(),
            ComponentState::Table {
                state: state.clone(),
                last_data: current_data,
            },
        );
        state
    }

    /// Gets or creates a TreeState entity for the given component.
    fn get_or_create_tree_state(
        &mut self,
        component: &BuiltComponent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TreeState> {
        let current_items = match component.properties.get("items") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };

        if let Some(ComponentState::Tree { state, last_items }) =
            self.component_states.get_mut(&component.id)
        {
            if *last_items != current_items {
                let tree_items = values_to_tree_items(&current_items);
                state.update(cx, |s, cx| {
                    s.set_items(tree_items, cx);
                });
                *last_items = current_items;
            }
            return state.clone();
        }

        let tree_items = values_to_tree_items(&current_items);
        let state = cx.new(|cx| TreeState::new(cx).items(tree_items));
        self.component_states.insert(
            component.id.clone(),
            ComponentState::Tree {
                state: state.clone(),
                last_items: current_items,
            },
        );
        state
    }

    /// Gets or creates a SliderState entity for the given component.
    fn get_or_create_slider_state(
        &mut self,
        component: &BuiltComponent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SliderState> {
        if let Some(ComponentState::Slider(state)) = self.component_states.get(&component.id) {
            return state.clone();
        }

        let props = &component.properties;
        let min = props.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let max = props.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
        let step = props.get("step").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let value = props.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

        let state = cx.new(|_cx| {
            SliderState::new()
                .min(min)
                .max(max)
                .step(step)
                .default_value(value)
        });
        self.component_states
            .insert(component.id.clone(), ComponentState::Slider(state.clone()));
        state
    }

    /// Renders the layout from the layout manager.
    fn render_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let entity_id = cx.entity_id();

        // Snapshot all components once to avoid re-acquiring the lock per component.
        let (root_id, components) = {
            let layout_manager = self
                .runtime
                .layout_manager
                .read()
                .expect("layout_manager lock poisoned");
            let root_id = layout_manager.root_id();
            let components: HashMap<String, BuiltComponent> = layout_manager
                .component_ids()
                .into_iter()
                .filter_map(|id| layout_manager.get_component(&id).cloned().map(|c| (id, c)))
                .collect();
            (root_id, components)
        };

        // Render the root component from the snapshot
        if let Some(root_id) = root_id {
            if let Some(root) = components.get(&root_id) {
                return self.render_component(root, &components, entity_id, window, cx);
            }
        }

        // Fallback: empty layout
        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Welcome to Nemo"),
            )
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().colors.muted_foreground)
                    .child("Configure your application in app.hcl"),
            )
            .into_any_element()
    }

    /// Renders the children of a component using a pre-snapshotted component map.
    fn render_children(
        &mut self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
        entity_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        component
            .children
            .iter()
            .filter_map(|child_id| components.get(child_id))
            .map(|child| self.render_component(child, components, entity_id, window, cx))
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
        let margin = props.get("margin").and_then(|v| v.as_i64());

        if width.is_none()
            && height.is_none()
            && min_width.is_none()
            && min_height.is_none()
            && flex.is_none()
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
        if let Some(m) = margin {
            wrapper = wrapper.m(px(m as f32));
        }
        wrapper.child(element).into_any_element()
    }

    /// Renders a component and its children recursively.
    fn render_component(
        &mut self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
        entity_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let element = match component.component_type.as_str() {
            "stack" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Stack::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "panel" => {
                let children = self.render_children(component, components, entity_id, window, cx);
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
                let input_state = self.get_or_create_input_state(&component.id, window, cx);
                crate::components::Input::new(component.clone())
                    .input_state(input_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "select" => {
                let initial = component
                    .properties
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let sel_state = self.component_states.get_or_create_selected_value(&component.id, initial);
                Select::new(component.clone())
                    .selected_value(sel_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "progress" => Progress::new(component.clone()).into_any_element(),
            "image" => Image::new(component.clone()).into_any_element(),
            "notification" => Notification::new(component.clone()).into_any_element(),
            "tabs" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Tabs::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "modal" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Modal::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "tooltip" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Tooltip::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "table" => {
                let table_state = self.get_or_create_table_state(component, window, cx);
                Table::new(component.clone())
                    .table_state(table_state)
                    .into_any_element()
            }
            "list" => List::new(component.clone()).into_any_element(),
            "tree" => {
                let tree_state = self.get_or_create_tree_state(component, window, cx);
                Tree::new(component.clone())
                    .tree_state(tree_state)
                    .into_any_element()
            }
            "line_chart" => LineChart::new(component.clone()).into_any_element(),
            "bar_chart" => BarChart::new(component.clone()).into_any_element(),
            "area_chart" => AreaChart::new(component.clone()).into_any_element(),
            "pie_chart" => PieChart::new(component.clone()).into_any_element(),
            "candlestick_chart" => CandlestickChart::new(component.clone()).into_any_element(),
            "accordion" => {
                let acc_state = self.component_states.get_or_create_accordion_state(
                    &component.id,
                    component.properties.get("items"),
                );
                Accordion::new(component.clone())
                    .open_indices(acc_state)
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "alert" => Alert::new(component.clone()).into_any_element(),
            "avatar" => Avatar::new(component.clone()).into_any_element(),
            "badge" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Badge::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "collapsible" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                let initial_open = component
                    .properties
                    .get("open")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let coll_state = self.component_states.get_or_create_bool_state(&component.id, initial_open);
                Collapsible::new(component.clone())
                    .open_state(coll_state)
                    .children(children)
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "dropdown_button" => DropdownButton::new(component.clone()).into_any_element(),
            "radio" => {
                let options: Vec<String> = component
                    .properties
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let initial_val = component.properties.get("value").and_then(|v| v.as_str());
                let initial_ix = initial_val.and_then(|val| options.iter().position(|o| o == val));
                let radio_state = self.component_states.get_or_create_selected_index(&component.id, initial_ix);
                Radio::new(component.clone())
                    .selected_index(radio_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "slider" => {
                let slider_state = self.get_or_create_slider_state(component, window, cx);
                Slider::new(component.clone())
                    .slider_state(slider_state)
                    .into_any_element()
            }
            "spinner" => Spinner::new(component.clone()).into_any_element(),
            "switch" => {
                let initial = component
                    .properties
                    .get("checked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let sw_state = self.component_states.get_or_create_bool_state(&component.id, initial);
                Switch::new(component.clone())
                    .checked_state(sw_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "tag" => Tag::new(component.clone()).into_any_element(),
            "toggle" => {
                let initial = component
                    .properties
                    .get("checked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let tog_state = self.component_states.get_or_create_bool_state(&component.id, initial);
                Toggle::new(component.clone())
                    .checked_state(tog_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            _ => {
                let children = self.render_children(component, components, entity_id, window, cx);
                div()
                    .flex()
                    .flex_col()
                    .children(children)
                    .into_any_element()
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
            .overflow_hidden()
            .bg(bg_color)
            .text_color(text_color)
            .child(self.header_bar.clone())
            .child(self.render_layout(window, cx))
    }
}
