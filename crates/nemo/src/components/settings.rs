use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    div, px, App, Axis, Entity, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _,
};
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{
    NumberFieldOptions, SelectIndex, SettingField, SettingGroup, SettingItem, SettingPage,
    Settings as GpuiSettings,
};
use gpui_kit::component::{ActiveTheme as _, Icon as KitIcon, Sizable as _, Size as ComponentSize};
use nemo_config::Value;
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;
use crate::runtime::NemoRuntime;

/// The typed value of one declarative settings field. The value's type is
/// implied by the field's `type`: switch/checkbox carry a flag, number carries
/// an f64, input/dropdown carry text.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Flag(bool),
    Number(f64),
    Text(String),
}

impl SettingValue {
    /// The value as a bool; non-flag values read as `false`.
    pub(crate) fn as_flag(&self) -> bool {
        matches!(self, SettingValue::Flag(true))
    }

    /// The value as an f64; non-number values read as `0.0`.
    pub(crate) fn as_number(&self) -> f64 {
        match self {
            SettingValue::Number(number) => *number,
            _ => 0.0,
        }
    }

    /// The value as text; non-text values read as empty.
    pub(crate) fn as_text(&self) -> &str {
        match self {
            SettingValue::Text(text) => text,
            _ => "",
        }
    }

    /// The rendered string used for handler payloads and the property
    /// read-back, so scripts see the same `property=value` pairs everywhere.
    pub fn display(&self) -> String {
        match self {
            SettingValue::Flag(flag) => (*flag).to_string(),
            SettingValue::Number(number) => number.to_string(),
            SettingValue::Text(text) => text.clone(),
        }
    }
}

/// The control a `<settings-field>` renders. The kit dispatches its field
/// renderer by the stored value's type, so these become `SettingField<bool>`,
/// `SettingField<f64>`, or `SettingField<SharedString>` when built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsFieldKind {
    Switch,
    Checkbox,
    Input,
    Number,
    Dropdown,
}

/// One selectable option of a dropdown field (a `<settings-option>` child).
#[derive(Clone, Debug)]
pub struct SettingsOptionData {
    pub value: String,
    pub label: String,
}

/// The declarative field of a `<settings-item>`: its control, the property
/// key its value lives under, seed/reset values, number bounds, and change
/// handlers.
#[derive(Clone, Debug)]
pub struct SettingsFieldData {
    /// The built component's id; routes the field-level `on-change` handler.
    pub id: String,
    pub kind: SettingsFieldKind,
    /// The property key the live value is stored and written back under.
    pub property: String,
    /// The seed value from the `value` attribute (the value store is seeded
    /// from this on creation and on schema edits).
    pub initial: SettingValue,
    /// The reset value from the `default` attribute; present only when the
    /// attribute is declared, and it makes the page resettable.
    pub default: Option<SettingValue>,
    /// Number-input bounds and step (kit defaults when undeclared).
    pub min: f64,
    pub max: f64,
    pub step: f64,
    /// Dropdown option list.
    pub options: Vec<SettingsOptionData>,
    /// The field-level `on-change` handler; receives the new value.
    pub on_change: Option<String>,
}

/// One declarative row of a `<settings-group>`: a labelled control with a
/// description, search keywords, and an optional vertical layout.
#[derive(Clone, Debug)]
pub struct SettingsItemData {
    pub title: String,
    pub description: Option<String>,
    /// Search-only keywords (comma-separated in the XML; never rendered).
    pub keywords: Vec<String>,
    pub disabled: bool,
    /// `layout="vertical"` stacks the label above the control.
    pub vertical: bool,
    /// The field child. Items without one are skipped by the collector: the
    /// kit's custom-element items are Rust-closure seams that the
    /// declarative system cannot express.
    pub field: Option<SettingsFieldData>,
}

/// One declarative `<settings-group>`: a titled card of setting items.
#[derive(Clone, Debug)]
pub struct SettingsGroupData {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Group surface override: `normal` (flat), `fill`, or `outline`.
    pub variant: Option<GroupBoxVariant>,
    pub items: Vec<SettingsItemData>,
}

/// One declarative `<settings-page>`: a sidebar entry with its groups.
#[derive(Clone, Debug)]
pub struct SettingsPageData {
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    /// `default-open="true"` expands the page's groups in the sidebar.
    pub default_open: bool,
    pub groups: Vec<SettingsGroupData>,
}

/// The declared schema's identity, used by the app to detect XML edits that
/// require re-seeding the value store (stores keep live field values across
/// renders, and their seed comes from the declared `value` attributes).
pub fn settings_schema_signature(pages: &[SettingsPageData]) -> String {
    pages
        .iter()
        .map(|page| {
            let groups = page
                .groups
                .iter()
                .map(|group| {
                    let items = group
                        .items
                        .iter()
                        .map(|item| {
                            let keywords = item.keywords.join("/");
                            let field = item.field.as_ref().map(format_field).unwrap_or_default();
                            format!(
                                "{}/{}/{}/{}/{}/{}",
                                item.title,
                                item.description.clone().unwrap_or_default(),
                                keywords,
                                item.disabled,
                                item.vertical,
                                field
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        "{}/{}/{}/{}",
                        group.title.clone().unwrap_or_default(),
                        group.description.clone().unwrap_or_default(),
                        group
                            .variant
                            .map(|variant| format!("{variant:?}"))
                            .unwrap_or_default(),
                        items
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}/{}/{}/{}/{}",
                page.title,
                page.description.clone().unwrap_or_default(),
                page.icon.clone().unwrap_or_default(),
                page.default_open,
                groups
            )
        })
        .collect()
}

fn format_field(field: &SettingsFieldData) -> String {
    let options = field
        .options
        .iter()
        .map(|option| format!("{}/{}", option.value, option.label))
        .collect::<Vec<_>>()
        .join("|");
    let default = field
        .default
        .as_ref()
        .map(|default| default.display())
        .unwrap_or_default();
    format!(
        "{:?}/{}/{}/{}/{}/{}/{}/{}",
        field.kind,
        field.initial.display(),
        default,
        field.min,
        field.max,
        field.step,
        options,
        field.on_change.clone().unwrap_or_default()
    )
}

/// Tolerantly reads a boolean-typed attribute value: booleans parse directly,
/// and string-typed schemas parse as truthy ("true", "1", "on", "yes").
pub fn setting_flag_value(value: Option<&Value>) -> bool {
    value
        .map(|value| {
            value.as_bool().unwrap_or_else(|| {
                value
                    .as_str()
                    .map(|text| matches!(text.trim(), "true" | "1" | "on" | "yes"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Reads a f64-typed attribute value; unparsable (or string-typed) values
/// fall back to `fallback`.
pub fn setting_number_value(value: Option<&Value>, fallback: f64) -> f64 {
    value
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
        })
        .unwrap_or(fallback)
}

/// Reads a text-typed attribute value, trimmed.
pub fn setting_text_value(value: Option<&Value>) -> String {
    value
        .and_then(|value| value.as_str().map(str::trim))
        .unwrap_or("")
        .to_string()
}

/// The current values of every declarative field in one `<settings>` widget,
/// keyed by each field's `property` attribute.
///
/// The wrapper's get/set closures read and write through this entity so
/// toggles survive re-renders (and GPUI entity updates drive the re-render
/// itself). The app re-seeds the store from the declared `value` attributes
/// whenever the schema signature changes; see
/// `App::get_or_create_settings_state`.
pub struct SettingValues {
    values: HashMap<String, SettingValue>,
}

impl SettingValues {
    pub fn new(values: HashMap<String, SettingValue>) -> Self {
        Self { values }
    }

    pub fn get(&self, property: &str) -> Option<&SettingValue> {
        self.values.get(property)
    }

    pub fn set(&mut self, property: String, value: SettingValue) {
        self.values.insert(property, value);
    }
}

/// Everything a field's set-value closure pushes an edit back into Nemo:
/// the value store, the property read-back, and the change handlers.
#[derive(Clone)]
struct FieldWiring {
    values: Option<Entity<SettingValues>>,
    runtime: Option<Arc<NemoRuntime>>,
    component_id: String,
    field_id: String,
    property: String,
    field_on_change: Option<String>,
    root_on_change: Option<String>,
}

impl FieldWiring {
    /// The shared write path for every field kind: update the store, mirror
    /// the value into the settings component's `property` attribute slot, and
    /// fire the field-level then root-level change handlers.
    fn commit(&self, value: SettingValue, cx: &mut App) {
        let display = value.display();

        if let Some(values) = &self.values {
            values.update(cx, |store, _| {
                store.set(self.property.clone(), value);
            });
        }

        let Some(runtime) = &self.runtime else {
            return;
        };
        if let Ok(mut manager) = runtime.layout_manager.try_write() {
            let _ = manager.set_property(
                &self.component_id,
                &self.property,
                Value::String(display.clone()),
            );
        }
        if let Some(handler) = &self.field_on_change {
            runtime.call_handler(handler, &self.field_id, &display);
        }
        if let Some(handler) = &self.root_on_change {
            runtime.call_handler(
                handler,
                &self.component_id,
                &format!("{}={}", self.property, display),
            );
        }
    }
}

/// An application preferences surface (gpui-kit `Settings`): a resizable,
/// searchable sidebar of pages, and the active page's titled groups of
/// labelled setting controls (switch, checkbox, text input, number input, or
/// dropdown). The hierarchy is declarative — `<settings-page>` children carry
/// `<settings-group>` cards of `<settings-item>` rows, whose single
/// `<settings-field>` child declares the control:
///
/// ```xml
/// <settings id="prefs" on-change="handle_pref_changed">
///   <settings-page title="General" icon="settings">
///     <settings-group title="Preferences">
///       <settings-item title="Autosave" description="Save drafts as you type.">
///         <settings-field type="switch" property="autosave" value="true" default="true" />
///       </settings-item>
///       <settings-item title="Editor font size">
///         <settings-field type="number" property="font_size" value="14" min="10" max="24" step="1" />
///       </settings-item>
///     </settings-group>
///   </settings-page>
/// </settings>
/// ```
///
/// The kit manages the sidebar selection and search with its own keyed state;
/// the only cross-render state Nemo owns is the field value store, which the
/// get/set closures read and write (see `SettingValues`). Toggling a control
/// updates the store, mirrors the value into the settings component's
/// property slot named by the field's `property` attribute, and fires the
/// field's `on-change` (value only) plus the root's `on-change`
/// (`property=value`). A field with a `default` attribute makes the page
/// resettable: the header shows a reset dial while the value differs.
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `height` | integer | Surface height in px (default 400; the page lists need a definite height) |
/// | `size` | string | Scales the composition: `small`, `medium` (default), or `large` |
/// | `sidebar-width` | integer | Sidebar width in px (default 250; resizable 160–360) |
/// | `group-variant` | string | Default group surface: `normal`, `fill`, or `outline` |
/// | `default-page` | string | Title of the page selected initially |
/// | `on-change` | handler | Any field changed; receives `property=value` |
///
/// # Out of scope
///
/// The kit's richer seams are Rust closures the declarative system cannot
/// express, and are not silently dropped:
///
/// * Custom element items (`SettingItem::render`) and custom field elements
///   (`SettingFieldElement`) — a `<settings-item>` without a
///   `<settings-field>` is skipped by the collector.
/// * Group footers and page title suffixes (`SettingGroup::footer`,
///   `SettingPage::title_suffix`).
/// * Custom reset behavior (`SettingField::on_reset` /
///   `SettingItem::on_reset`) — reset uses the typed `default` attribute only.
/// * Input placeholders — the kit's `StringField` renders no placeholder.
#[derive(IntoElement)]
pub struct Settings {
    source: BuiltComponent,
    runtime: Option<Arc<NemoRuntime>>,
    values: Option<Entity<SettingValues>>,
    pages: Vec<SettingsPageData>,
}

impl Settings {
    pub fn new(source: BuiltComponent, pages: Vec<SettingsPageData>) -> Self {
        Self {
            source,
            runtime: None,
            values: None,
            pages,
        }
    }

    pub fn values(mut self, values: Option<Entity<SettingValues>>) -> Self {
        self.values = values;
        self
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    fn size_of(value: Option<&str>) -> ComponentSize {
        match value {
            Some("xsmall") => ComponentSize::XSmall,
            Some("small") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        }
    }

    /// Turns the collected declarative pages into kit pages. Field getters and
    /// setters capture the value store and wiring, so every control reads its
    /// live value from the store and writes edits back through it.
    fn build_pages(&self) -> Vec<SettingPage> {
        self.pages
            .iter()
            .map(|page| {
                let mut built = SettingPage::new(page.title.clone());
                if let Some(description) = &page.description {
                    built = built.description(description.clone());
                }
                if let Some(icon) = &page.icon {
                    built = built.icon(KitIcon::new(map_icon_name(icon)));
                }
                if page.default_open {
                    built = built.default_open(true);
                }
                for group in &page.groups {
                    built = built.group(self.build_group(group));
                }
                built
            })
            .collect()
    }

    fn build_group(&self, group: &SettingsGroupData) -> SettingGroup {
        let mut built = SettingGroup::new();
        if let Some(title) = &group.title {
            built = built.title(title.clone());
        }
        if let Some(description) = &group.description {
            built = built.description(description.clone());
        }
        if let Some(variant) = group.variant {
            built = built.variant(variant);
        }
        for item in &group.items {
            let Some(field) = &item.field else {
                continue;
            };
            built = built.item(self.build_item(item, field));
        }
        built
    }

    fn build_item(&self, item: &SettingsItemData, field: &SettingsFieldData) -> SettingItem {
        let mut built = match field.kind {
            SettingsFieldKind::Switch => {
                SettingItem::new(item.title.clone(), self.flag_field(field, true))
            }
            SettingsFieldKind::Checkbox => {
                SettingItem::new(item.title.clone(), self.flag_field(field, false))
            }
            SettingsFieldKind::Input | SettingsFieldKind::Dropdown => {
                SettingItem::new(item.title.clone(), self.text_field(field))
            }
            SettingsFieldKind::Number => {
                SettingItem::new(item.title.clone(), self.number_field(field))
            }
        };
        if let Some(description) = &item.description {
            built = built.description(description.clone());
        }
        if !item.keywords.is_empty() {
            built = built.keywords(item.keywords.clone());
        }
        if item.disabled {
            built = built.disabled(true);
        }
        if item.vertical {
            built = built.layout(Axis::Vertical);
        }
        built
    }

    /// The wiring bundle a field's set-value closure captures.
    fn wiring(&self, field: &SettingsFieldData) -> FieldWiring {
        FieldWiring {
            values: self.values.clone(),
            runtime: self.runtime.clone(),
            component_id: self.source.id.clone(),
            field_id: field.id.clone(),
            property: field.property.clone(),
            field_on_change: field.on_change.clone(),
            root_on_change: self.source.handlers.get("change").cloned(),
        }
    }

    /// A switch or checkbox control backed by the value store's flag.
    fn flag_field(&self, field: &SettingsFieldData, use_switch: bool) -> SettingField<bool> {
        let values = self.values.clone();
        let property = field.property.clone();
        let fallback = field.initial.as_flag();

        let getter = move |cx: &App| -> bool {
            values
                .as_ref()
                .and_then(|values| values.read(cx).get(&property).cloned())
                .map_or(fallback, |value| value.as_flag())
        };

        let wiring = self.wiring(field);
        let setter = move |value: bool, cx: &mut App| {
            wiring.commit(SettingValue::Flag(value), cx);
        };

        let mut built = if use_switch {
            SettingField::switch(getter, setter)
        } else {
            SettingField::checkbox(getter, setter)
        };
        if let Some(default) = &field.default {
            built = built.default_value(default.as_flag());
        }
        built
    }

    /// A text input or dropdown control backed by the value store's text.
    fn text_field(&self, field: &SettingsFieldData) -> SettingField<SharedString> {
        let values = self.values.clone();
        let property = field.property.clone();
        let fallback = field.initial.as_text().to_string();

        let getter = move |cx: &App| -> SharedString {
            values
                .as_ref()
                .and_then(|values| values.read(cx).get(&property).cloned())
                .map_or_else(
                    || SharedString::from(fallback.clone()),
                    |value| SharedString::from(value.as_text().to_string()),
                )
        };

        let wiring = self.wiring(field);
        let setter = move |value: SharedString, cx: &mut App| {
            wiring.commit(SettingValue::Text(value.to_string()), cx);
        };

        let mut built = if field.kind == SettingsFieldKind::Dropdown {
            let options = field
                .options
                .iter()
                .map(|option| {
                    (
                        SharedString::from(option.value.clone()),
                        SharedString::from(option.label.clone()),
                    )
                })
                .collect::<Vec<_>>();
            SettingField::dropdown(options, getter, setter)
        } else {
            SettingField::input(getter, setter)
        };
        if let Some(default) = &field.default {
            built = built.default_value(SharedString::from(default.as_text().to_string()));
        }
        built
    }

    /// A number input backed by the value store's number.
    fn number_field(&self, field: &SettingsFieldData) -> SettingField<f64> {
        let values = self.values.clone();
        let property = field.property.clone();
        let fallback = field.initial.as_number();

        let getter = move |cx: &App| -> f64 {
            values
                .as_ref()
                .and_then(|values| values.read(cx).get(&property).cloned())
                .map_or(fallback, |value| value.as_number())
        };

        let wiring = self.wiring(field);
        let setter = move |value: f64, cx: &mut App| {
            wiring.commit(SettingValue::Number(value), cx);
        };

        let mut built = SettingField::number_input(
            NumberFieldOptions {
                min: field.min,
                max: field.max,
                step: field.step,
            },
            getter,
            setter,
        );
        if let Some(default) = &field.default {
            built = built.default_value(default.as_number());
        }
        built
    }
}

impl RenderOnce for Settings {
    fn render(self, _window: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        if self.pages.is_empty() {
            // Nothing declarative to compose: a muted placeholder keeps the
            // slot visible without building an empty kit surface.
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Settings: no settings-page children declared")
                .into_any_element();
        }

        let props = &self.source.properties;
        let size = Self::size_of(props.get("size").and_then(|v| v.as_str()));
        // The kit's page lists and the resizable sidebar need a definite
        // height; an undeclared height collapses them to a strip.
        let height = props.get("height").and_then(|v| v.as_f64()).unwrap_or(400.) as f32;
        let sidebar_width = px(props
            .get("sidebar_width")
            .and_then(|v| v.as_i64())
            .unwrap_or(250) as f32);
        let group_variant = match props.get("group_variant").and_then(|v| v.as_str()) {
            Some("fill") => GroupBoxVariant::Fill,
            Some("outline") => GroupBoxVariant::Outline,
            _ => GroupBoxVariant::Normal,
        };
        let default_page = props
            .get("default_page")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let default_page_ix = self
            .pages
            .iter()
            .position(|page| page.title == default_page)
            .unwrap_or(0);

        let widget = GpuiSettings::new(SharedString::from(self.source.id.clone()))
            .pages(self.build_pages())
            .with_group_variant(group_variant)
            .with_size(size)
            .sidebar_width(sidebar_width)
            .default_selected_index(SelectIndex {
                page_ix: default_page_ix,
                group_ix: None,
            });

        div()
            .w_full()
            .h(px(height))
            .child(widget)
            .into_any_element()
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    fn pages() -> Vec<SettingsPageData> {
        let field = SettingsFieldData {
            id: "prefs_autosave".to_string(),
            kind: SettingsFieldKind::Switch,
            property: "autosave".to_string(),
            initial: SettingValue::Flag(true),
            default: Some(SettingValue::Flag(true)),
            min: f64::MIN,
            max: f64::MAX,
            step: 1.0,
            options: Vec::new(),
            on_change: Some("handle_autosave".to_string()),
        };
        let item = SettingsItemData {
            title: "Autosave".to_string(),
            description: Some("Save drafts as you type.".to_string()),
            keywords: vec!["save".to_string()],
            disabled: false,
            vertical: false,
            field: Some(field),
        };
        let group = SettingsGroupData {
            title: Some("Preferences".to_string()),
            description: None,
            variant: Some(GroupBoxVariant::Outline),
            items: vec![item],
        };
        vec![SettingsPageData {
            title: "General".to_string(),
            description: Some("Base preferences".to_string()),
            icon: Some("settings".to_string()),
            default_open: true,
            groups: vec![group],
        }]
    }

    #[test]
    fn schema_signature_tracks_definitions() {
        let before = settings_schema_signature(&pages());
        // A stable schema is byte-stable.
        assert_eq!(before, settings_schema_signature(&pages()));

        // An initial-value edit must re-seed the store.
        let mut edited = pages();
        edited[0].groups[0].items[0].field.as_mut().unwrap().initial = SettingValue::Flag(false);
        assert_ne!(
            before,
            settings_schema_signature(&edited),
            "editing a field's initial value changes the schema"
        );

        // New declaration order (e.g. a new item) — signature changes.
        let mut grown = pages();
        grown[0].groups[0].items.push(SettingsItemData {
            title: "Compact rows".to_string(),
            description: None,
            keywords: Vec::new(),
            disabled: true,
            vertical: true,
            field: None,
        });
        assert_ne!(before, settings_schema_signature(&grown));
        // Items without a field participate in the signature even though the
        // collector skips them when building pages.
    }

    #[test]
    fn display_strings_match_handler_contract() {
        assert_eq!(SettingValue::Flag(true).display(), "true");
        assert_eq!(SettingValue::Flag(false).display(), "false");
        assert_eq!(SettingValue::Number(14.0).display(), "14");
        assert_eq!(SettingValue::Number(0.5).display(), "0.5");
        assert_eq!(SettingValue::Text("sans".to_string()).display(), "sans");
    }

    #[test]
    fn store_keeps_latest_values() {
        let mut store = SettingValues::new(HashMap::from([(
            "autosave".to_string(),
            SettingValue::Flag(true),
        )]));
        assert_eq!(store.get("autosave"), Some(&SettingValue::Flag(true)));
        store.set("autosave".to_string(), SettingValue::Flag(false));
        assert_eq!(store.get("autosave"), Some(&SettingValue::Flag(false)));
        store.set("font_size".to_string(), SettingValue::Number(14.0));
        assert_eq!(store.get("font_size"), Some(&SettingValue::Number(14.0)));
        assert!(store.get("missing").is_none());
    }

    #[test]
    fn tolerant_attribute_parsers() {
        assert!(setting_flag_value(Some(&Value::Bool(true))));
        assert!(setting_flag_value(Some(&Value::String("1".into()))));
        assert!(!setting_flag_value(Some(&Value::String("no".into()))));
        assert!(!setting_flag_value(None));

        assert_eq!(setting_number_value(Some(&Value::Float(2.5)), 1.0), 2.5);
        assert_eq!(
            setting_number_value(Some(&Value::String("3".into())), 1.0),
            3.0
        );
        assert_eq!(setting_number_value(None, 4.0), 4.0);

        assert_eq!(
            setting_text_value(Some(&Value::String(" sans ".into()))),
            "sans"
        );
        assert_eq!(setting_text_value(None), "");
    }
}
