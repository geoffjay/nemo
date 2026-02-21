//! Settings view for the Nemo application.
//!
//! Provides a native GPUI settings page accessible via `ctrl-p`.
//! Shows general application settings and plugin-contributed settings sections.

use gpui::*;
use gpui_component::input::{Input as GpuiInput, InputState};
use gpui_component::label::Label;
use gpui_component::slider::{Slider as GpuiSlider, SliderState};
use gpui_component::switch::Switch as GpuiSwitch;
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use nemo_extension::SettingsPageInfo;
use nemo_plugin_api::PluginValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::runtime::NemoRuntime;

/// Event emitted when the user wants to close the settings view.
pub struct CloseSettingsEvent;

impl EventEmitter<CloseSettingsEvent> for SettingsView {}

/// Which settings page is currently selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    Plugin(usize),
}

/// The settings view entity.
pub struct SettingsView {
    runtime: Arc<NemoRuntime>,
    selected_page: SettingsPage,
    plugin_pages: Vec<SettingsPageInfo>,
    bool_states: HashMap<String, Arc<Mutex<bool>>>,
    input_states: HashMap<String, Entity<InputState>>,
    slider_states: HashMap<String, Entity<SliderState>>,
}

impl SettingsView {
    pub fn new(runtime: Arc<NemoRuntime>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let plugin_pages = runtime
            .extension_manager
            .read()
            .expect("extension_manager lock poisoned")
            .plugin_settings_pages()
            .to_vec();

        Self {
            runtime,
            selected_page: SettingsPage::General,
            plugin_pages,
            bool_states: HashMap::new(),
            input_states: HashMap::new(),
            slider_states: HashMap::new(),
        }
    }

    fn select_page(&mut self, page: SettingsPage, cx: &mut Context<Self>) {
        self.selected_page = page;
        cx.notify();
    }

    fn get_or_create_bool_state(&mut self, id: &str, initial: bool) -> Arc<Mutex<bool>> {
        if let Some(state) = self.bool_states.get(id) {
            return Arc::clone(state);
        }
        let state = Arc::new(Mutex::new(initial));
        self.bool_states.insert(id.to_string(), Arc::clone(&state));
        state
    }

    fn get_or_create_input_state_with_placeholder(
        &mut self,
        id: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(state) = self.input_states.get(id) {
            return state.clone();
        }
        let ph = placeholder.to_string();
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(ph));
        self.input_states.insert(id.to_string(), state.clone());
        state
    }

    fn get_or_create_slider_state(
        &mut self,
        id: &str,
        min: f32,
        max: f32,
        step: f32,
        value: f32,
        cx: &mut Context<Self>,
    ) -> Entity<SliderState> {
        if let Some(state) = self.slider_states.get(id) {
            return state.clone();
        }
        let state = cx.new(|_cx| {
            SliderState::new()
                .min(min)
                .max(max)
                .step(step)
                .default_value(value)
        });
        self.slider_states.insert(id.to_string(), state.clone());
        state
    }

    /// Renders the left sidebar with page list.
    fn render_sidebar(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let border_color = cx.theme().colors.border;
        let selected_bg = cx.theme().colors.list_active;
        let hover_bg = cx.theme().colors.list_hover;

        let mut sidebar = v_flex()
            .w(px(200.))
            .flex_shrink_0()
            .border_r_1()
            .border_color(border_color)
            .py_2();

        // General item
        let is_selected = self.selected_page == SettingsPage::General;
        let general_bg = if is_selected {
            selected_bg
        } else {
            transparent_black()
        };
        sidebar = sidebar.child({
            let mut item = div()
                .id("settings-general")
                .px_3()
                .py_1p5()
                .mx_2()
                .rounded_md()
                .cursor_pointer()
                .bg(general_bg)
                .child(Label::new("General").text_size(px(14.)))
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.select_page(SettingsPage::General, cx);
                }));
            if !is_selected {
                item = item.hover(|s| s.bg(hover_bg));
            }
            item
        });

        // Plugin items
        for (idx, page_info) in self.plugin_pages.iter().enumerate() {
            let is_selected = self.selected_page == SettingsPage::Plugin(idx);
            let name = page_info.display_name.clone();
            let item_bg = if is_selected {
                selected_bg
            } else {
                transparent_black()
            };
            sidebar = sidebar.child({
                let mut item = div()
                    .id(ElementId::NamedInteger(
                        "settings-plugin".into(),
                        idx as u64,
                    ))
                    .px_3()
                    .py_1p5()
                    .mx_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(item_bg)
                    .child(Label::new(name).text_size(px(14.)))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.select_page(SettingsPage::Plugin(idx), cx);
                    }));
                if !is_selected {
                    item = item.hover(|s| s.bg(hover_bg));
                }
                item
            });
        }

        sidebar
    }

    /// Renders the General settings page.
    fn render_general_page(&self, cx: &App) -> Div {
        let muted = cx.theme().colors.muted_foreground;

        let project_dir = self
            .runtime
            .get_config("app.project_dir")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "(not set)".to_string());

        let theme_name = self
            .runtime
            .get_config("app.theme.name")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "default".to_string());

        let theme_mode = self
            .runtime
            .get_config("app.theme.mode")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "dark".to_string());

        v_flex()
            .gap_4()
            .child(
                Label::new("General Settings")
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(settings_row("Theme", &theme_name, muted))
                    .child(settings_row("Color Mode", &theme_mode, muted))
                    .child(settings_row("Project Directory", &project_dir, muted))
                    .child(settings_row("Version", env!("CARGO_PKG_VERSION"), muted)),
            )
    }

    /// Renders a plugin settings page from its PluginValue definition.
    fn render_plugin_page(
        &mut self,
        idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let page_info = &self.plugin_pages[idx];
        let title = page_info.display_name.clone();
        let page = page_info.page.clone();
        let plugin_id = page_info.plugin_id.clone();

        let mut container = v_flex().gap_4().child(
            Label::new(title)
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD),
        );

        if let PluginValue::Object(obj) = &page {
            if let Some(PluginValue::Array(children)) = obj.get("children") {
                for (i, child) in children.iter().enumerate() {
                    let child_id = format!("{}.{}.{}", plugin_id, idx, i);
                    let element = self.render_plugin_widget(child, &child_id, window, cx);
                    container = container.child(element);
                }
            }
        }

        container.into_any_element()
    }

    /// Recursively renders a plugin widget from a PluginValue definition.
    fn render_plugin_widget(
        &mut self,
        value: &PluginValue,
        id_prefix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let obj = match value {
            PluginValue::Object(obj) => obj,
            _ => return div().into_any_element(),
        };

        let widget_type = pv_str(obj, "type").unwrap_or("unknown");

        match widget_type {
            "stack" => {
                let direction = pv_str(obj, "direction").unwrap_or("vertical");

                let mut container = if direction == "horizontal" {
                    div().flex().flex_row().gap_3()
                } else {
                    div().flex().flex_col().gap_3()
                };

                if let Some(PluginValue::Array(children)) = obj.get("children") {
                    for (i, child) in children.iter().enumerate() {
                        let child_id = format!("{}.{}", id_prefix, i);
                        let element = self.render_plugin_widget(child, &child_id, window, cx);
                        container = container.child(element);
                    }
                }

                container.into_any_element()
            }
            "panel" => {
                let title = obj.get("title").and_then(|v| {
                    if let PluginValue::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                });

                let border_color = cx.theme().colors.border;
                let mut panel = v_flex()
                    .gap_3()
                    .p_4()
                    .border_1()
                    .border_color(border_color)
                    .rounded_md();

                if let Some(t) = title {
                    panel = panel.child(
                        Label::new(t)
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM),
                    );
                }

                if let Some(PluginValue::Array(children)) = obj.get("children") {
                    for (i, child) in children.iter().enumerate() {
                        let child_id = format!("{}.{}", id_prefix, i);
                        let element = self.render_plugin_widget(child, &child_id, window, cx);
                        panel = panel.child(element);
                    }
                }

                panel.into_any_element()
            }
            "label" => {
                let text = obj
                    .get("text")
                    .and_then(|v| {
                        if let PluginValue::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                Label::new(text).into_any_element()
            }
            "switch" => {
                let label_text = pv_string(obj, "label").unwrap_or_default();
                let default = obj
                    .get("default")
                    .and_then(|v| {
                        if let PluginValue::Bool(b) = v {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false);

                let switch_id = format!("sw-{}", id_prefix);
                let checked = self.get_or_create_bool_state(&switch_id, default);
                let is_checked = *checked.lock().unwrap();

                let entity_id = cx.entity_id();
                let checked_clone = checked.clone();
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(Label::new(label_text))
                    .child(
                        GpuiSwitch::new(SharedString::from(switch_id))
                            .checked(is_checked)
                            .on_click(move |checked_val, _window, cx| {
                                *checked_clone.lock().unwrap() = *checked_val;
                                cx.notify(entity_id);
                            }),
                    )
                    .into_any_element()
            }
            "input" => {
                let label_text = pv_string(obj, "label").unwrap_or_default();
                let placeholder = pv_string(obj, "placeholder").unwrap_or_default();

                let input_id = format!("inp-{}", id_prefix);
                let input_state = self.get_or_create_input_state_with_placeholder(
                    &input_id,
                    &placeholder,
                    window,
                    cx,
                );

                v_flex()
                    .gap_1()
                    .child(Label::new(label_text))
                    .child(GpuiInput::new(&input_state))
                    .into_any_element()
            }
            "slider" => {
                let label_text = pv_string(obj, "label").unwrap_or_default();
                let min = pv_f32(obj, "min", 0.0);
                let max = pv_f32(obj, "max", 100.0);
                let step = pv_f32(obj, "step", 1.0);
                let value = pv_f32(obj, "value", 50.0);

                let slider_id = format!("sl-{}", id_prefix);
                let slider_state =
                    self.get_or_create_slider_state(&slider_id, min, max, step, value, cx);

                v_flex()
                    .gap_1()
                    .child(Label::new(label_text))
                    .child(GpuiSlider::new(&slider_state))
                    .into_any_element()
            }
            "button" => {
                let label_text = pv_string(obj, "label").unwrap_or_else(|| "Button".to_string());

                gpui_component::button::Button::new(ElementId::Name(
                    format!("btn-{}", id_prefix).into(),
                ))
                .label(label_text)
                .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }
}

/// Extract a string reference from a PluginValue object.
fn pv_str<'a>(obj: &'a indexmap::IndexMap<String, PluginValue>, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| {
        if let PluginValue::String(s) = v {
            Some(s.as_str())
        } else {
            None
        }
    })
}

/// Extract an owned String from a PluginValue object.
fn pv_string(obj: &indexmap::IndexMap<String, PluginValue>, key: &str) -> Option<String> {
    pv_str(obj, key).map(|s| s.to_string())
}

/// Extract f32 from a PluginValue object.
fn pv_f32(obj: &indexmap::IndexMap<String, PluginValue>, key: &str, default: f32) -> f32 {
    obj.get(key)
        .map(|v| match v {
            PluginValue::Float(f) => *f as f32,
            PluginValue::Integer(i) => *i as f32,
            _ => default,
        })
        .unwrap_or(default)
}

/// Renders a simple key-value row for the general settings page.
fn settings_row(label: &str, value: &str, muted: Hsla) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .py_1p5()
        .child(Label::new(label.to_string()).text_size(px(14.)))
        .child(
            Label::new(value.to_string())
                .text_size(px(14.))
                .text_color(muted),
        )
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = cx.theme().colors.background;
        let border_color = cx.theme().colors.border;

        // Header
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(border_color)
            .child(
                Label::new("Settings")
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                gpui_component::button::Button::new("settings-back")
                    .label("Back")
                    .on_click(cx.listener(|_this, _, _window, cx| {
                        cx.emit(CloseSettingsEvent);
                    })),
            );

        // Content area
        let sidebar = self.render_sidebar(window, cx);

        let content = match &self.selected_page {
            SettingsPage::General => self.render_general_page(cx).into_any_element(),
            SettingsPage::Plugin(idx) => {
                let idx = *idx;
                self.render_plugin_page(idx, window, cx)
            }
        };

        let content_panel = div()
            .id("settings-content")
            .flex_1()
            .p_6()
            .overflow_y_scroll()
            .child(content);

        let body = div()
            .flex()
            .flex_row()
            .flex_1()
            .overflow_hidden()
            .child(sidebar)
            .child(content_panel);

        v_flex().size_full().bg(bg).child(header).child(body)
    }
}
