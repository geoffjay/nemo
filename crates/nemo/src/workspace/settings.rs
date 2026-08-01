//! Settings view for the Nemo application.
//!
//! Provides a native GPUI settings page accessible via `ctrl-p`.
//! Shows general application settings and plugin-contributed settings sections.

use gpui::*;
use gpui_component::button::{Button as GpuiButton, DropdownButton as GpuiDropdownButton};
use gpui_component::input::{Input as GpuiInput, InputEvent, InputState};
use gpui_component::label::Label;
use gpui_component::menu::PopupMenuItem;
use gpui_component::slider::{Slider as GpuiSlider, SliderState};
use gpui_component::switch::Switch as GpuiSwitch;
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use nemo_extension::SettingsPageInfo;
use nemo_plugin_api::PluginValue;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::config::NemoConfig;
use crate::runtime::NemoRuntime;
use crate::theme;
use crate::theme::tokens::{radius_of, Space, TokenStyled};
use crate::workspace::xml_edit;

/// Callback invoked when a settings dropdown item is chosen. Receives the
/// selected option string plus the GPUI window/app context so it can apply and
/// persist the change.
type OnSelect = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// Event emitted when the user wants to close the settings view.
pub struct CloseSettingsEvent;

impl EventEmitter<CloseSettingsEvent> for SettingsView {}

/// Which settings page is currently selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsPage {
    /// Global settings, persisted to `~/.config/nemo/config.toml`.
    Global,
    /// Project settings, persisted to the loaded project's `app.xml`.
    Project,
    Plugin(usize),
}

/// The settings view entity.
pub struct SettingsView {
    runtime: Arc<NemoRuntime>,
    nemo_config: Arc<Mutex<NemoConfig>>,
    selected_page: SettingsPage,
    plugin_pages: Vec<SettingsPageInfo>,
    bool_states: HashMap<String, Arc<Mutex<bool>>>,
    input_states: HashMap<String, Entity<InputState>>,
    slider_states: HashMap<String, Entity<SliderState>>,
    font_input_state: Entity<InputState>,
    /// Last theme selected on the Project page, cached so the dropdown reflects
    /// the choice before the (read-only) runtime config is reloaded.
    project_theme: Option<String>,
    /// Last color mode selected on the Project page (see `project_theme`).
    project_mode: Option<String>,
}

impl SettingsView {
    pub fn new(
        runtime: Arc<NemoRuntime>,
        nemo_config: Arc<Mutex<NemoConfig>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let plugin_pages = runtime
            .extension_manager
            .read()
            .expect("extension_manager lock poisoned")
            .plugin_settings_pages()
            .to_vec();

        let current_font = cx.theme().font_family.to_string();
        let font_input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("System default");
            state.set_value(current_font, window, cx);
            state
        });

        // Subscribe to font input events (blur and enter) to apply + persist
        {
            let nemo_config = Arc::clone(&nemo_config);
            cx.subscribe_in(
                &font_input_state,
                window,
                move |_this, font_state, event, window, cx| {
                    let should_apply =
                        matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. });
                    if !should_apply {
                        return;
                    }

                    let value = font_state.read(cx).value().to_string();
                    let trimmed = value.trim().to_string();

                    if trimmed.is_empty() {
                        let default_font: SharedString = if cfg!(target_os = "macos") {
                            ".SystemUIFont".into()
                        } else {
                            "sans-serif".into()
                        };
                        gpui_component::Theme::global_mut(cx).font_family = default_font;
                        if let Ok(mut cfg) = nemo_config.lock() {
                            cfg.app.font_family = None;
                            let _ = cfg.save();
                        }
                    } else {
                        let family: SharedString = trimmed.clone().into();
                        gpui_component::Theme::global_mut(cx).font_family = family;
                        if let Ok(mut cfg) = nemo_config.lock() {
                            cfg.app.font_family = Some(trimmed);
                            let _ = cfg.save();
                        }
                    }
                    window.refresh();
                    cx.notify();
                },
            )
            .detach();
        }

        Self {
            runtime,
            nemo_config,
            selected_page: SettingsPage::Global,
            plugin_pages,
            bool_states: HashMap::new(),
            input_states: HashMap::new(),
            slider_states: HashMap::new(),
            font_input_state,
            project_theme: None,
            project_mode: None,
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
        let item_radius = radius_of("md", cx);

        let mut sidebar = v_flex()
            .w(px(200.))
            .flex_shrink_0()
            .border_r_1()
            .border_color(border_color)
            .py_t(Space::Sm);

        // Global + Project items
        for (page, id, label) in [
            (SettingsPage::Global, "settings-global", "Global"),
            (SettingsPage::Project, "settings-project", "Project"),
        ] {
            let is_selected = self.selected_page == page;
            let item_bg = if is_selected {
                selected_bg
            } else {
                transparent_black()
            };
            sidebar = sidebar.child({
                let page = page.clone();
                let mut item = div()
                    .id(id)
                    .px_t(Space::Md)
                    .py_1p5()
                    .mx_t(Space::Sm)
                    .rounded(item_radius)
                    .cursor_pointer()
                    .bg(item_bg)
                    .child(Label::new(label).text_size(px(14.)))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.select_page(page.clone(), cx);
                    }));
                if !is_selected {
                    item = item.hover(|s| s.bg(hover_bg));
                }
                item
            });
        }

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
                    .px_t(Space::Md)
                    .py_1p5()
                    .mx_t(Space::Sm)
                    .rounded(item_radius)
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

    /// Build a dropdown selector: a button showing `current_label` that opens a
    /// menu of `options`, invoking `on_select(option)` when an item is clicked.
    fn selector(
        id: String,
        current_label: String,
        options: Vec<String>,
        on_select: OnSelect,
    ) -> impl IntoElement {
        let button = GpuiButton::new(SharedString::from(format!("{id}-btn")))
            .label(SharedString::from(current_label));
        GpuiDropdownButton::new(SharedString::from(id))
            .button(button)
            .outline()
            .dropdown_menu(move |menu, _window, _cx| {
                let mut m = menu;
                for opt in &options {
                    let on_select = on_select.clone();
                    let val = opt.clone();
                    m = m.item(
                        PopupMenuItem::new(SharedString::from(opt.clone()))
                            .on_click(move |_ev, window, cx| on_select(val.clone(), window, cx)),
                    );
                }
                m
            })
    }

    /// A labelled settings row with an interactive control on the right.
    fn setting_field(label: &str, control: impl IntoElement) -> Div {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .py_1p5()
            .child(Label::new(label.to_string()).text_size(px(14.)))
            .child(control)
    }

    /// Renders the Global settings page (persists to `~/.config/nemo/config.toml`).
    fn render_global_page(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().colors.muted_foreground;
        let entity = cx.entity();

        let (cur_theme, cur_mode) = {
            let cfg = self.nemo_config.lock().expect("nemo_config lock poisoned");
            (
                cfg.app.theme_name.clone(),
                cfg.app
                    .theme_mode
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
            )
        };

        let theme_options = theme::get_theme_set_names();
        let on_theme: OnSelect = {
            let cfg = Arc::clone(&self.nemo_config);
            let entity = entity.clone();
            Rc::new(move |sel, window, cx| {
                let name_lc = sel.to_lowercase();
                let mode = {
                    let c = cfg.lock().expect("nemo_config lock poisoned");
                    c.app
                        .theme_mode
                        .clone()
                        .unwrap_or_else(|| "system".to_string())
                };
                theme::apply_configured_theme(&name_lc, &mode, None, cx);
                if let Ok(mut c) = cfg.lock() {
                    c.app.theme_name = name_lc;
                    let _ = c.save();
                }
                entity.update(cx, |_this, cx| cx.notify());
                window.refresh();
            })
        };

        let mode_options = vec![
            "dark".to_string(),
            "light".to_string(),
            "system".to_string(),
        ];
        let on_mode: OnSelect = {
            let cfg = Arc::clone(&self.nemo_config);
            let entity = entity.clone();
            Rc::new(move |sel, window, cx| {
                let name = {
                    let c = cfg.lock().expect("nemo_config lock poisoned");
                    c.app.theme_name.clone()
                };
                if name != "default" {
                    theme::apply_configured_theme(&name, &sel, None, cx);
                }
                if let Ok(mut c) = cfg.lock() {
                    c.app.theme_mode = Some(sel);
                    let _ = c.save();
                }
                entity.update(cx, |_this, cx| cx.notify());
                window.refresh();
            })
        };

        v_flex()
            .gap_t(Space::Lg)
            .child(
                Label::new("Global Settings")
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                Label::new("Applies to every project. Stored in ~/.config/nemo/config.toml.")
                    .text_size(px(12.))
                    .text_color(muted),
            )
            .child(
                v_flex()
                    .gap_t(Space::Md)
                    .child(Self::setting_field(
                        "Theme",
                        Self::selector(
                            "global-theme".to_string(),
                            display_set_name(&cur_theme),
                            theme_options,
                            on_theme,
                        ),
                    ))
                    .child(Self::setting_field(
                        "Color Mode",
                        Self::selector("global-mode".to_string(), cur_mode, mode_options, on_mode),
                    ))
                    .child(
                        v_flex()
                            .gap_t(Space::Xs)
                            .child(Label::new("Font Family").text_size(px(14.)))
                            .child(GpuiInput::new(&self.font_input_state)),
                    )
                    .child(settings_row("Version", env!("CARGO_PKG_VERSION"), muted)),
            )
    }

    /// Renders the Project settings page (persists to the loaded `app.xml`).
    fn render_project_page(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().colors.muted_foreground;
        let entity = cx.entity();
        let path = self.runtime.config_path().to_path_buf();

        let cur_theme = self
            .project_theme
            .clone()
            .or_else(|| {
                self.runtime
                    .get_config("app.theme.name")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "default".to_string());
        let cur_mode = self
            .project_mode
            .clone()
            .or_else(|| {
                self.runtime
                    .get_config("app.theme.mode")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "dark".to_string());

        let project_dir = self
            .runtime
            .get_config("app.project_dir")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "(not set)".to_string());

        let theme_options = theme::get_theme_set_names();
        let on_theme: OnSelect = {
            let path = path.clone();
            let runtime = Arc::clone(&self.runtime);
            let entity = entity.clone();
            Rc::new(move |sel, window, cx| {
                let name_lc = sel.to_lowercase();
                let mode = entity
                    .read(cx)
                    .project_mode
                    .clone()
                    .or_else(|| {
                        runtime
                            .get_config("app.theme.mode")
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                    })
                    .unwrap_or_else(|| "dark".to_string());
                theme::apply_configured_theme(&name_lc, &mode, None, cx);
                if let Err(e) = xml_edit::set_app_theme(&path, &name_lc, &mode) {
                    tracing::error!("Failed to write theme to {}: {}", path.display(), e);
                }
                entity.update(cx, |this, cx| {
                    this.project_theme = Some(name_lc);
                    cx.notify();
                });
                window.refresh();
            })
        };

        let mode_options = vec![
            "dark".to_string(),
            "light".to_string(),
            "system".to_string(),
        ];
        let on_mode: OnSelect = {
            let path = path.clone();
            let runtime = Arc::clone(&self.runtime);
            let entity = entity.clone();
            Rc::new(move |sel, window, cx| {
                let name = entity.read(cx).project_theme.clone().or_else(|| {
                    runtime
                        .get_config("app.theme.name")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                });
                if let Some(name) = name {
                    let name_lc = name.to_lowercase();
                    theme::apply_configured_theme(&name_lc, &sel, None, cx);
                    if let Err(e) = xml_edit::set_app_theme(&path, &name_lc, &sel) {
                        tracing::error!("Failed to write theme to {}: {}", path.display(), e);
                    }
                }
                entity.update(cx, |this, cx| {
                    this.project_mode = Some(sel);
                    cx.notify();
                });
                window.refresh();
            })
        };

        v_flex()
            .gap_t(Space::Lg)
            .child(
                Label::new("Project Settings")
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                Label::new(format!(
                    "Overrides global settings. Stored in {}.",
                    path.display()
                ))
                .text_size(px(12.))
                .text_color(muted),
            )
            .child(
                v_flex()
                    .gap_t(Space::Md)
                    .child(Self::setting_field(
                        "Theme",
                        Self::selector(
                            "project-theme".to_string(),
                            display_set_name(&cur_theme),
                            theme_options,
                            on_theme,
                        ),
                    ))
                    .child(Self::setting_field(
                        "Color Mode",
                        Self::selector("project-mode".to_string(), cur_mode, mode_options, on_mode),
                    ))
                    .child(settings_row("Project Directory", &project_dir, muted)),
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

        let mut container = v_flex().gap_t(Space::Lg).child(
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
                    div().flex().flex_row().gap_t(Space::Md)
                } else {
                    div().flex().flex_col().gap_t(Space::Md)
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
                let panel_radius = radius_of("md", cx);
                let mut panel = v_flex()
                    .gap_t(Space::Md)
                    .p_t(Space::Lg)
                    .border_1()
                    .border_color(border_color)
                    .rounded(panel_radius);

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
                    .gap_t(Space::Xs)
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
                    .gap_t(Space::Xs)
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

/// Map a stored theme value (e.g. "kanagawa" or "default") to a display label,
/// matching the canonical set names case-insensitively.
fn display_set_name(stored: &str) -> String {
    if stored.eq_ignore_ascii_case("default") {
        return "Default".to_string();
    }
    theme::get_theme_set_names()
        .into_iter()
        .find(|n| n.eq_ignore_ascii_case(stored))
        .unwrap_or_else(|| stored.to_string())
}

/// Renders a simple key-value row for the settings pages.
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

        // Content area
        let sidebar = self.render_sidebar(window, cx);

        let content = match &self.selected_page {
            SettingsPage::Global => self.render_global_page(cx).into_any_element(),
            SettingsPage::Project => self.render_project_page(cx).into_any_element(),
            SettingsPage::Plugin(idx) => {
                let idx = *idx;
                self.render_plugin_page(idx, window, cx)
            }
        };

        let content_panel = div()
            .id("settings-content")
            .flex_1()
            .p_t(Space::Xl)
            .overflow_y_scroll()
            .child(content);

        let body = div()
            .flex()
            .flex_row()
            .flex_1()
            .overflow_hidden()
            .child(sidebar)
            .child(content_panel);

        v_flex().size_full().bg(bg).child(body)
    }
}
