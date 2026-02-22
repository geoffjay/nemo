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
use gpui_component::input::TabSize;

use crate::components::{
    apply_rounded, apply_shadow, Accordion, Alert, AreaChart, Avatar, Badge, BarChart, BubbleChart,
    Button, CandlestickChart, Checkbox, ClusteredBarChart, ClusteredColumnChart, CodeEditor,
    Collapsible, ColumnChart, DropdownButton, FunnelChart, HeatmapChart, Icon, Image, Label,
    LineChart, List, Modal, Notification, Panel, PieChart, Progress, PyramidChart, RadarChart,
    Radio, RealtimeChart, ScatterChart, Select, SidenavBar, Slider, Spinner, Stack,
    StackedBarChart, StackedColumnChart, Switch, Table, Tabs, Tag, Text, TextEditor, Textarea,
    Toggle, Tooltip, Tree,
};
use crate::runtime::NemoRuntime;
use nemo_layout::BuiltComponent;

/// The main Nemo GPUI application.
pub struct App {
    runtime: Arc<NemoRuntime>,
    component_states: ComponentStates,
    _subscriptions: Vec<Subscription>,
}

impl App {
    /// Creates a new Nemo application.
    pub fn new(runtime: Arc<NemoRuntime>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
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

    /// Gets or creates an InputState entity configured for a Textarea component.
    fn get_or_create_textarea_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(ComponentState::Input(state)) = self.component_states.get(&component.id) {
            return state.clone();
        }

        let props = &component.properties;
        let placeholder = props
            .get("placeholder")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let default_value = props
            .get("default_value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let rows = props.get("rows").and_then(|v| v.as_i64());
        let auto_grow_min = props.get("auto_grow_min").and_then(|v| v.as_i64());
        let auto_grow_max = props.get("auto_grow_max").and_then(|v| v.as_i64());

        let state = cx.new(|cx| {
            let mut s = InputState::new(window, cx).placeholder(placeholder);

            // Configure mode: auto_grow takes priority, then rows, then default multi_line
            if let (Some(min), Some(max)) = (auto_grow_min, auto_grow_max) {
                s = s.auto_grow(min as usize, max as usize);
            } else {
                s = s.multi_line(true);
                s = s.rows(rows.unwrap_or(4) as usize);
            }

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(component.id.clone(), ComponentState::Input(state.clone()));
        state
    }

    /// Gets or creates an InputState entity configured for a CodeEditor component.
    fn get_or_create_code_editor_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(ComponentState::Input(state)) = self.component_states.get(&component.id) {
            return state.clone();
        }

        let props = &component.properties;
        let language = props
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("plain")
            .to_string();
        let line_number = props
            .get("line_number")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let searchable = props
            .get("searchable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let default_value = props
            .get("default_value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let multi_line = props
            .get("multi_line")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let tab_size_val = props.get("tab_size").and_then(|v| v.as_i64()).unwrap_or(4) as usize;
        let hard_tabs = props
            .get("hard_tabs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let rows = props.get("rows").and_then(|v| v.as_i64());

        let state = cx.new(|cx| {
            let mut s = InputState::new(window, cx)
                .code_editor(language)
                .line_number(line_number)
                .searchable(searchable)
                .tab_size(TabSize {
                    tab_size: tab_size_val,
                    hard_tabs,
                });

            if !multi_line {
                s = s.multi_line(false);
            }

            s = s.rows(rows.unwrap_or(4) as usize);

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(component.id.clone(), ComponentState::Input(state.clone()));
        state
    }

    /// Gets or creates an InputState entity configured for a TextEditor component.
    fn get_or_create_text_editor_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(ComponentState::Input(state)) = self.component_states.get(&component.id) {
            return state.clone();
        }

        let props = &component.properties;
        let placeholder = props
            .get("placeholder")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let default_value = props
            .get("default_value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let rows = props.get("rows").and_then(|v| v.as_i64());

        let state = cx.new(|cx| {
            let mut s = InputState::new(window, cx)
                .multi_line(true)
                .placeholder(placeholder)
                .rows(rows.unwrap_or(4) as usize);

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(component.id.clone(), ComponentState::Input(state.clone()));
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

    /// Wraps an element in a styled div if layout/decoration properties are present.
    fn apply_layout_styles(
        element: AnyElement,
        component: &BuiltComponent,
        cx: &gpui::App,
    ) -> AnyElement {
        let props = &component.properties;

        // Sizing
        let width = props.get("width").and_then(|v| v.as_i64());
        let height = props.get("height").and_then(|v| v.as_i64());
        let min_width = props.get("min_width").and_then(|v| v.as_i64());
        let min_height = props.get("min_height").and_then(|v| v.as_i64());
        let flex = props
            .get("flex")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));

        // Margin
        let margin = props.get("margin").and_then(|v| v.as_i64());
        let margin_x = props.get("margin_x").and_then(|v| v.as_i64());
        let margin_y = props.get("margin_y").and_then(|v| v.as_i64());
        let margin_left = props.get("margin_left").and_then(|v| v.as_i64());
        let margin_right = props.get("margin_right").and_then(|v| v.as_i64());
        let margin_top = props.get("margin_top").and_then(|v| v.as_i64());
        let margin_bottom = props.get("margin_bottom").and_then(|v| v.as_i64());

        // Padding
        let padding = props.get("padding").and_then(|v| v.as_i64());
        let padding_x = props.get("padding_x").and_then(|v| v.as_i64());
        let padding_y = props.get("padding_y").and_then(|v| v.as_i64());
        let padding_left = props.get("padding_left").and_then(|v| v.as_i64());
        let padding_right = props.get("padding_right").and_then(|v| v.as_i64());
        let padding_top = props.get("padding_top").and_then(|v| v.as_i64());
        let padding_bottom = props.get("padding_bottom").and_then(|v| v.as_i64());

        // Border
        let border = props.get("border").and_then(|v| v.as_i64());
        let border_x = props.get("border_x").and_then(|v| v.as_i64());
        let border_y = props.get("border_y").and_then(|v| v.as_i64());
        let border_left = props.get("border_left").and_then(|v| v.as_i64());
        let border_right = props.get("border_right").and_then(|v| v.as_i64());
        let border_top = props.get("border_top").and_then(|v| v.as_i64());
        let border_bottom = props.get("border_bottom").and_then(|v| v.as_i64());
        let border_color = props.get("border_color").and_then(|v| v.as_str());

        // Decoration
        let shadow = props.get("shadow").and_then(|v| v.as_str());
        let rounded = props.get("rounded").and_then(|v| v.as_str());

        // Early return if nothing to apply
        if [
            width,
            height,
            min_width,
            min_height,
            margin,
            margin_x,
            margin_y,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
            padding,
            padding_x,
            padding_y,
            padding_left,
            padding_right,
            padding_top,
            padding_bottom,
            border,
            border_x,
            border_y,
            border_left,
            border_right,
            border_top,
            border_bottom,
        ]
        .iter()
        .all(|v| v.is_none())
            && flex.is_none()
            && shadow.is_none()
            && rounded.is_none()
            && border_color.is_none()
        {
            return element;
        }

        let mut wrapper = div().flex().flex_col().min_h(px(0.));

        // Propagate flex layout for container components whose inner element
        // uses flex_1 — without this the wrapper breaks the flex chain and
        // prevents overflow scrolling.
        if component.component_type == "stack" {
            wrapper = wrapper.flex_1();
        }

        // Sizing
        if let Some(w) = width {
            wrapper = wrapper.w(px(w as f32));
        }
        if let Some(h) = height {
            wrapper = wrapper.h(px(h as f32)).flex_shrink_0();
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
        if let Some(mx) = margin_x {
            wrapper = wrapper.mx(px(mx as f32));
        }
        if let Some(my) = margin_y {
            wrapper = wrapper.my(px(my as f32));
        }
        if let Some(ml) = margin_left {
            wrapper = wrapper.ml(px(ml as f32));
        }
        if let Some(mr) = margin_right {
            wrapper = wrapper.mr(px(mr as f32));
        }
        if let Some(mt) = margin_top {
            wrapper = wrapper.mt(px(mt as f32));
        }
        if let Some(mb) = margin_bottom {
            wrapper = wrapper.mb(px(mb as f32));
        }

        // Padding
        if let Some(p) = padding {
            wrapper = wrapper.p(px(p as f32));
        }
        if let Some(px_val) = padding_x {
            wrapper = wrapper.px(px(px_val as f32));
        }
        if let Some(py_val) = padding_y {
            wrapper = wrapper.py(px(py_val as f32));
        }
        if let Some(pl) = padding_left {
            wrapper = wrapper.pl(px(pl as f32));
        }
        if let Some(pr) = padding_right {
            wrapper = wrapper.pr(px(pr as f32));
        }
        if let Some(pt) = padding_top {
            wrapper = wrapper.pt(px(pt as f32));
        }
        if let Some(pb) = padding_bottom {
            wrapper = wrapper.pb(px(pb as f32));
        }

        // Border
        let resolved_border_color = border_color
            .and_then(|c| crate::components::resolve_color(c, cx))
            .unwrap_or(cx.theme().colors.border);
        let has_any_border = [
            border,
            border_x,
            border_y,
            border_left,
            border_right,
            border_top,
            border_bottom,
        ]
        .iter()
        .any(|v| v.is_some());
        if has_any_border {
            wrapper = wrapper.border_color(resolved_border_color);
        }
        if let Some(b) = border {
            wrapper = wrapper.border(px(b as f32));
        }
        if let Some(bx) = border_x {
            wrapper = wrapper.border_x(px(bx as f32));
        }
        if let Some(by) = border_y {
            wrapper = wrapper.border_y(px(by as f32));
        }
        if let Some(bl) = border_left {
            wrapper = wrapper.border_l(px(bl as f32));
        }
        if let Some(br) = border_right {
            wrapper = wrapper.border_r(px(br as f32));
        }
        if let Some(bt) = border_top {
            wrapper = wrapper.border_t(px(bt as f32));
        }
        if let Some(bb) = border_bottom {
            wrapper = wrapper.border_b(px(bb as f32));
        }

        // Decoration
        wrapper = apply_shadow(wrapper, shadow);
        wrapper = apply_rounded(wrapper, rounded);

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
            "textarea" => {
                let input_state = self.get_or_create_textarea_state(component, window, cx);
                Textarea::new(component.clone())
                    .input_state(input_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "code_editor" => {
                let input_state = self.get_or_create_code_editor_state(component, window, cx);
                CodeEditor::new(component.clone())
                    .input_state(input_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "text_editor" => {
                let input_state = self.get_or_create_text_editor_state(component, window, cx);
                TextEditor::new(component.clone())
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
                let sel_state = self
                    .component_states
                    .get_or_create_selected_value(&component.id, initial);
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
                let initial = component
                    .properties
                    .get("active_tab")
                    .and_then(|v| v.as_i64())
                    .map(|i| Some(i as usize))
                    .unwrap_or(Some(0));
                let tab_state = self
                    .component_states
                    .get_or_create_selected_index(&component.id, initial);
                Tabs::new(component.clone())
                    .selected_index(tab_state)
                    .children(children)
                    .entity_id(entity_id)
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
            "realtime_chart" => RealtimeChart::new(component.clone()).into_any_element(),
            "bar_chart" => BarChart::new(component.clone()).into_any_element(),
            "area_chart" => AreaChart::new(component.clone()).into_any_element(),
            "pie_chart" => PieChart::new(component.clone()).into_any_element(),
            "candlestick_chart" => CandlestickChart::new(component.clone()).into_any_element(),
            "column_chart" => ColumnChart::new(component.clone()).into_any_element(),
            "stacked_column_chart" => StackedColumnChart::new(component.clone()).into_any_element(),
            "clustered_column_chart" => {
                ClusteredColumnChart::new(component.clone()).into_any_element()
            }
            "stacked_bar_chart" => StackedBarChart::new(component.clone()).into_any_element(),
            "clustered_bar_chart" => ClusteredBarChart::new(component.clone()).into_any_element(),
            "scatter_chart" => ScatterChart::new(component.clone()).into_any_element(),
            "bubble_chart" => BubbleChart::new(component.clone()).into_any_element(),
            "heatmap_chart" => HeatmapChart::new(component.clone()).into_any_element(),
            "radar_chart" => RadarChart::new(component.clone()).into_any_element(),
            "pyramid_chart" => PyramidChart::new(component.clone()).into_any_element(),
            "funnel_chart" => FunnelChart::new(component.clone()).into_any_element(),
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
                let coll_state = self
                    .component_states
                    .get_or_create_bool_state(&component.id, initial_open);
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
                let radio_state = self
                    .component_states
                    .get_or_create_selected_index(&component.id, initial_ix);
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
            "sidenav_bar" => {
                // Collect child BuiltComponents for sidenav_bar_item rendering
                let child_components: Vec<BuiltComponent> = component
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|c| c.component_type == "sidenav_bar_item")
                    .cloned()
                    .collect();
                // Render non-item children (e.g. buttons) normally
                let other_children: Vec<AnyElement> = component
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|c| c.component_type != "sidenav_bar_item")
                    .map(|c| self.render_component(c, components, entity_id, window, cx))
                    .collect();
                let initial = component
                    .properties
                    .get("collapsed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let collapsed_state = self
                    .component_states
                    .get_or_create_bool_state(&component.id, initial);
                // Sync property value from scripts into the shared state
                *collapsed_state.lock().unwrap() = initial;
                SidenavBar::new(component.clone())
                    .collapsed_state(collapsed_state)
                    .child_components(child_components)
                    .children(other_children)
                    .entity_id(entity_id)
                    .runtime(Arc::clone(&self.runtime))
                    .into_any_element()
            }
            "sidenav_bar_item" => {
                // Items are rendered by their parent SidenavBar; standalone fallback
                div().into_any_element()
            }
            "spinner" => Spinner::new(component.clone()).into_any_element(),
            "switch" => {
                let initial = component
                    .properties
                    .get("checked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let sw_state = self
                    .component_states
                    .get_or_create_bool_state(&component.id, initial);
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
                let tog_state = self
                    .component_states
                    .get_or_create_bool_state(&component.id, initial);
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
        Self::apply_layout_styles(element, component, cx)
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The header bar is now rendered by AppLayout; App only renders layout content.
        div().size_full().child(self.render_layout(window, cx))
    }
}
