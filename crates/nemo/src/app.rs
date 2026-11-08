//! GPUI application wrapper.

use gpui::*;
use gpui_kit::component::calendar::{CalendarEvent, CalendarState, Date as CalendarDate, Matcher};
use gpui_kit::component::carousel::{CarouselEvent, CarouselState};
use gpui_kit::component::command::CommandState;
use gpui_kit::component::dock::{DockArea as KitDockArea, DockSkin};
use gpui_kit::component::input::{EditorState, InputEvent, InputState, TextareaState};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::component::questionnaire::{
    QuestionnaireChoiceDefinition, QuestionnaireEvent, QuestionnaireInputDefinition,
    QuestionnaireItemDefinition, QuestionnaireShortcutMode, QuestionnaireState,
};
use gpui_kit::component::slider::SliderState;
use gpui_kit::component::table::TableState;
use gpui_kit::component::tree::TreeState;
use gpui_kit::component::v_flex;
use gpui_kit::component::ActiveTheme;
use nemo_config::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use crate::components::state::{ComponentState, ComponentStates};
use crate::components::table::{
    DeclaredCell, DeclaredColumn, DeclaredRow, DeclaredTableData, NemoTableDelegate,
};
use crate::components::tree::{declared_to_tree_items, values_to_tree_items, DeclaredTreeNode};
use crate::components::{parse_date, selected_date_string, CommandEntryData};
use gpui_kit::component::diff::{DiffEvent, DiffFile, DiffState};
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::input::TabSize;
use gpui_kit::component::table::{Column, TableEvent};

use crate::components::{
    apply_dock_layout, apply_rounded, apply_shadow, data_table_autodetect_columns,
    data_table_schema_signature, diff_context_lines_of, diff_inline_unit_of, diff_mode_of,
    diff_schema_signature, diff_selection_display, dock_layout_signature, dock_placement_value,
    questionnaire_schema_signature, serialize_questionnaire_answers, setting_flag_value,
    setting_number_value, setting_text_value, settings_schema_signature,
    validate_questionnaire_schema, Accordion, AccordionItemData, Alert, AreaChart, Attachment,
    AttachmentContentData, AttachmentMediaData, Avatar, Badge, BarChart, Bubble, BubbleChart,
    BubbleGroup, BubbleReactionsData, Button, Calendar, CandlestickChart, Carousel, Checkbox,
    ClusteredBarChart, ClusteredColumnChart, CodeEditor, Collapsible, ColumnChart, Command,
    DataTable, DataTableColumnData, DataTableDelegate, DeclarativePanel, Diff, Dock, DockPanelSpec,
    DropdownButton, FunnelChart, HeatmapChart, Icon, Image, Kbd, Label, LineChart, List, Marker,
    Menu, MenuEntryData, MenuItem, Message, MessageScroller, Modal, Notification, OptionData,
    Pagination, Panel, PieChart, Progress, PyramidChart, Questionnaire, QuestionnaireChoiceData,
    QuestionnaireInputData, QuestionnaireItemData, RadarChart, Radio, Rating, RealtimeChart,
    ScatterChart, Select, SettingValue, SettingValues, Settings, SettingsFieldData,
    SettingsFieldKind, SettingsGroupData, SettingsItemData, SettingsOptionData, SettingsPageData,
    SidenavBar, Skeleton, Slider, Spinner, Stack, StackedBarChart, StackedColumnChart, Stepper,
    StepperItemData, Svg, Switch, TabItemData, Table, Tabs, Tag, Text, TextEditor, Textarea,
    Toggle, Tooltip, Tree,
};
use crate::containers::{AppShell, NavLink, Router};
use crate::runtime::NemoRuntime;
use crate::theme::tokens::{FontSize, Space, TokenStyled};
use nemo_layout::BuiltComponent;

/// The main Nemo GPUI application.
pub struct App {
    runtime: Arc<NemoRuntime>,
    component_states: ComponentStates,
    _subscriptions: Vec<Subscription>,
    /// Last `value` we pushed into each `<input>`'s `InputState`, keyed by id.
    ///
    /// Lets the render pass tell script-driven property changes (e.g. clearing a
    /// field after submit) apart from user typing: only when the component's
    /// `value` property differs from this cached copy do we push it back into
    /// the `InputState`, so we never fight the cursor while the user types.
    input_values: HashMap<String, String>,
    /// Last parse error for each `<diff>`, keyed by id. An empty entry means
    /// the current patch parsed clean; the value is shown in place of the
    /// view when the patch could not be parsed.
    diff_errors: HashMap<String, String>,
}

impl App {
    /// Creates a new Nemo application.
    pub fn new(runtime: Arc<NemoRuntime>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Wait for data_notify signals and apply updates when data arrives.
        let poll_runtime = Arc::clone(&runtime);
        let data_notify = Arc::clone(&runtime.data_notify);
        let _data_task = cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| loop {
            data_notify.notified().await;
            // Apply queued navigations first so any route path/param projections
            // they flag are picked up by the data-update pass that follows.
            let navigated = poll_runtime.apply_pending_navigations();
            // Fire the initial `on-enter` for any router the render pass just
            // seeded to its default path, before the data-update pass picks up
            // the route projection it flags (issue #81).
            let initial_enters = poll_runtime.fire_pending_initial_enters();
            let data_updated = poll_runtime.apply_pending_data_updates();
            // A runtime `set_roundness()` call needs the gpui `Theme` (an `App`),
            // which only exists inside `this.update`; apply it there.
            let roundness = poll_runtime.take_pending_roundness();
            if navigated || initial_enters || data_updated || roundness.is_some() {
                let _ = this.update(cx, |_app: &mut App, cx: &mut Context<App>| {
                    if let Some(value) = roundness {
                        crate::theme::apply_roundness(&value, cx);
                    }
                    cx.notify();
                });
            }
        });
        _data_task.detach();

        let _subscriptions = vec![];

        // Run the one-shot on-load handler (if configured via
        // `<script on-load="…" />`) now that scripts are loaded and the layout
        // is built. This is where an app hydrates its UI from persisted state,
        // so the first paint already reflects it — no deferring onto the first
        // user interaction.
        if let Some(handler) = runtime.on_load_handler() {
            runtime.call_handler(&handler, "app", "load");
        }

        Self {
            runtime,
            component_states: ComponentStates::new(),
            _subscriptions,
            input_values: HashMap::new(),
            diff_errors: HashMap::new(),
        }
    }

    /// Shutdown the application and its runtime.
    pub fn shutdown(&mut self, _cx: &mut Context<Self>) {
        self.runtime.shutdown();
    }

    /// Gets or creates an `InputState` entity for the given `<input>` component.
    ///
    /// On first creation this also subscribes to the input's events so that:
    ///
    /// * on every `Change`/`Blur` the typed value is written back into the
    ///   component's `value` property, making `get_component_property(id,
    ///   "value")` return live text that a handler (or button) can read; and
    /// * on `PressEnter` the `on-change` handler (if any) fires with the current
    ///   value as its event data, giving a "submit" affordance.
    ///
    /// Every render it also pushes any script-set `value` back into the
    /// `InputState` (e.g. clearing a field after submit) without disturbing the
    /// cursor while the user types — see [`App::input_values`].
    fn get_or_create_input_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let id = component.id.clone();
        let desired = component
            .properties
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(ComponentState::Input(state)) = self.component_states.get(&id) {
            let state = state.clone();
            self.sync_input_value(&state, &id, &desired, window, cx);
            return state;
        }

        let state = cx.new(|cx| InputState::new(window, cx));

        // Subscribe once to keep the `value` property in sync with what the
        // user types, and to route Enter to the on-change handler.
        let change_handler = component.handlers.get("change").cloned();
        let sub_id = id.clone();
        let runtime = Arc::clone(&self.runtime);
        let subscription = cx.subscribe_in(
            &state,
            window,
            move |this: &mut App, input_state, event: &InputEvent, _window, cx| {
                let sync_value = |this: &mut App, value: String| {
                    if let Ok(mut lm) = runtime.layout_manager.try_write() {
                        let _ = lm.set_property(&sub_id, "value", Value::String(value.clone()));
                    }
                    // Record what the property now holds so the render pass does
                    // not push it back into the InputState and reset the cursor.
                    this.input_values.insert(sub_id.clone(), value);
                };

                match event {
                    InputEvent::Change | InputEvent::Blur => {
                        let value = input_state.read(cx).value().to_string();
                        sync_value(this, value);
                    }
                    InputEvent::PressEnter { .. } => {
                        let value = input_state.read(cx).value().to_string();
                        sync_value(this, value.clone());
                        if let Some(handler) = &change_handler {
                            runtime.call_handler(handler, &sub_id, &value);
                            cx.notify();
                        }
                    }
                    InputEvent::Focus => {}
                }
            },
        );
        self._subscriptions.push(subscription);

        self.component_states
            .insert(id.clone(), ComponentState::Input(state.clone()));
        self.sync_input_value(&state, &id, &desired, window, cx);
        state
    }

    /// Pushes `desired` into an input's `InputState` only when it differs from
    /// the last value we synced for `id` (i.e. a script changed it), so user
    /// typing is never overwritten mid-edit.
    fn sync_input_value(
        &mut self,
        state: &Entity<InputState>,
        id: &str,
        desired: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.input_values.get(id).map(String::as_str) == Some(desired) {
            return;
        }
        let value = desired.to_string();
        state.update(cx, |s, cx| s.set_value(value, window, cx));
        self.input_values
            .insert(id.to_string(), desired.to_string());
    }

    /// Gets or creates a TableState entity for the given component.
    fn get_or_create_table_state(
        &mut self,
        component: &BuiltComponent,
        declared_columns: &[DeclaredColumn],
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

        // Declarative `<table-column>` children feed the data table's
        // columns (replacing the `columns` JSON attribute); fall back to the
        // attribute/autodetect when no header is declared.
        let delegate = if declared_columns.is_empty() {
            NemoTableDelegate::from_properties(&component.properties)
        } else {
            let columns: Vec<Column> = declared_columns
                .iter()
                .map(|col| {
                    let mut column = Column::new(col.field.clone(), col.label.clone());
                    if let Some(w) = col.width {
                        column = column.width(px(w));
                    }
                    column
                })
                .collect();
            let column_keys: Vec<String> = declared_columns
                .iter()
                .map(|col| col.field.clone())
                .collect();
            let rows = match component.properties.get("data") {
                Some(Value::Array(arr)) => arr.clone(),
                _ => Vec::new(),
            };
            NemoTableDelegate::from_parts(columns, column_keys, rows)
        };
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

    /// Collects the declared `<data-table-column>` children of a
    /// `<data-table>` into column definitions (drop-in richer siblings of
    /// the `<table>` header's columns: per-column sort flags, pinning,
    /// alignment, resize/move behavior).
    fn collect_data_table_columns(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<DataTableColumnData> {
        let mut columns = Vec::new();
        for child_id in &component.children {
            let Some(child) = components.get(child_id) else {
                continue;
            };
            if child.component_type != "data_table_column" {
                continue;
            }
            let column = DataTableColumnData::from_properties(&child.properties);
            if std::env::var("NEMO_DT_DEBUG").is_ok() {
                eprintln!(
                    "[dt-debug] type={} props={:?}",
                    child.component_type, child.properties
                );
            }
            if !column.key.is_empty() {
                columns.push(column);
            }
        }
        columns
    }

    /// Gets or creates a `TableState<DataTableDelegate>` for a
    /// `<data-table>` component.
    ///
    /// Column / behavior-flag changes rebuild the state — the kit bakes the
    /// columns into its header layout at construction. A `data` change never
    /// rebuilds; it replaces the delegate's rows in place (same contract as
    /// [`App::get_or_create_table_state`]).
    ///
    /// On creation this also subscribes to the kit's `TableEvent` stream:
    /// row/column/cell selections mirror into the component's
    /// `selected-row` / `selected-col` / `selected-cell` properties (cleared
    /// properties become null), and the `on-*` handlers fire with the
    /// affected indexes as their event data.
    fn get_or_create_data_table_state(
        &mut self,
        component: &BuiltComponent,
        declared_columns: &[DataTableColumnData],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TableState<DataTableDelegate>> {
        let id = component.id.clone();
        let current_data = match component.properties.get("data") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };
        let schema = data_table_schema_signature(declared_columns, &component.properties);

        if let Some(ComponentState::DataTable {
            state,
            last_data,
            last_schema,
        }) = self.component_states.get_mut(&id)
        {
            if *last_schema == schema {
                if *last_data != current_data {
                    let new_data = current_data.clone();
                    state.update(cx, |s, cx| {
                        s.delegate_mut().replace_rows(new_data);
                        s.refresh(cx);
                    });
                    *last_data = current_data;
                }
                return state.clone();
            }
        }

        // A schema change drops the old state (its entity is retired; the
        // stale subscription no-ops once the entity is unreferenced).
        self.component_states.remove(&id);

        // No declared columns: auto-detect from the first row object's
        // fields, in order.
        let columns = if declared_columns.is_empty() {
            data_table_autodetect_columns(&current_data)
        } else {
            declared_columns.to_vec()
        };
        let kit_columns: Vec<Column> = columns.iter().map(|c| c.to_kit_column()).collect();
        let column_keys: Vec<String> = columns.iter().map(|c| c.key.clone()).collect();
        let delegate = DataTableDelegate::new(kit_columns, column_keys, current_data.clone());
        let state = cx.new(|cx| TableState::new(delegate, window, cx));

        // Bridge the kit's event stream: selection state mirrors into
        // properties so scripts and other components can read it, and the
        // declarative `on-*` handlers fire with the affected indexes.
        let handlers = component.handlers.clone();
        let runtime = Arc::clone(&self.runtime);
        let sub_id = id.clone();
        let subscription = cx.subscribe_in(
            &state,
            window,
            move |_this: &mut App, _state, event: &TableEvent, _window, cx| {
                let record = |key: &str, value: Value| {
                    if let Ok(mut lm) = runtime.layout_manager.try_write() {
                        let _ = lm.set_property(&sub_id, key, value);
                    }
                };
                let fire = |event_name: &str, data: String, cx: &mut Context<App>| {
                    if let Some(handler) = handlers.get(event_name) {
                        runtime.call_handler(handler, &sub_id, &data);
                        cx.notify();
                    }
                };
                match event {
                    TableEvent::SelectRow(row) => {
                        record("selected_row", Value::Integer(*row as i64));
                        fire("select_row", format!("row={row}"), cx);
                    }
                    TableEvent::DoubleClickedRow(row) => {
                        fire("double_click_row", format!("row={row}"), cx);
                    }
                    TableEvent::SelectColumn(col) => {
                        record("selected_col", Value::Integer(*col as i64));
                        fire("select_col", format!("col={col}"), cx);
                    }
                    TableEvent::SelectCell(row, col) => {
                        record("selected_cell", Value::String(format!("{row},{col}")));
                        fire("select_cell", format!("row={row},col={col}"), cx);
                    }
                    TableEvent::DoubleClickedCell(row, col) => {
                        fire("double_click_cell", format!("row={row},col={col}"), cx);
                    }
                    TableEvent::MoveColumn(from, to) => {
                        fire("move_column", format!("from={from},to={to}"), cx);
                    }
                    TableEvent::ClearSelection => {
                        record("selected_cell", Value::Null);
                        record("selected_row", Value::Null);
                        record("selected_col", Value::Null);
                        fire("clear_selection", String::new(), cx);
                    }
                    // Row/cell right-clicks have no declarative menu seam,
                    // and width changes are pure view state — documented as
                    // out of scope on the wrapper.
                    TableEvent::RightClickedRow(_)
                    | TableEvent::RightClickedCell(..)
                    | TableEvent::ColumnWidthsChanged(_) => {}
                }
            },
        );
        self._subscriptions.push(subscription);

        self.component_states.insert(
            id.clone(),
            ComponentState::DataTable {
                state: state.clone(),
                last_data: current_data,
                last_schema: schema,
            },
        );
        state
    }

    /// Gets or creates a `DiffState` entity for a `<diff>` component.
    ///
    /// Mode/context/expansion/inline-emphasis changes rebuild the state (the
    /// kit bakes them into its row model at creation). A `diff` patch change
    /// never rebuilds — the parsed files replace the state's rows via
    /// [`DiffState::set_files`], which resets selection. Parse failures set
    /// the entry in [`App::diff_errors`] (cleared when a later patch parses)
    /// and swap in an empty-files state so the render shows the message.
    ///
    /// On creation this also subscribes to `DiffEvent`: the gutter/gesture
    /// selection mirrors into the component's `selected-lines` property (`path:start-end`,
    /// null on clear) and `on-selection-changed` fires as the selection moves
    /// and when the gesture ends.
    fn get_or_create_diff_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<DiffState> {
        let id = component.id.clone();
        let current_data = component
            .properties
            .get("diff")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        // Parse the patch first: a malformed patch never reaches a live
        // state, and its located error is what the render shows.
        let parsed = current_data.as_ref().map(|patch| DiffFile::parse(patch));

        let error = match &parsed {
            Some(Ok(_)) => String::new(),
            Some(Err(error)) => error.to_string(),
            None => String::new(),
        };

        let schema = diff_schema_signature(&component.properties);
        if error.is_empty() {
            self.diff_errors.remove(&id);
        } else {
            self.diff_errors.insert(id.clone(), error);
        }

        if let Some(ComponentState::Diff {
            state,
            last_data,
            last_schema,
        }) = self.component_states.get_mut(&id)
        {
            if *last_schema == schema {
                if *last_data != current_data {
                    if let Some(Ok(files)) = parsed.as_ref() {
                        let files = files.clone();
                        state.update(cx, |s, cx| {
                            s.set_files(files, cx);
                        });
                    }
                    *last_data = current_data.clone();
                }
                return state.clone();
            }
        }

        // (Re)build the state from the parsed files (empty on parse failure —
        // the render shows the located message instead).
        let files: Vec<DiffFile> = match &parsed {
            Some(Ok(files)) => files.clone(),
            _ => Vec::new(),
        };

        // Per-creation configuration lives in the state (rebuilt on change).
        let props = &component.properties;
        let state = cx.new(|cx| {
            let config = DiffState::new(files, cx)
                .with_mode(diff_mode_of(props.get("mode").and_then(|v| v.as_str())))
                .with_context_lines(diff_context_lines_of(props))
                .with_expansion_lines(
                    props
                        .get("expansion_lines")
                        .and_then(|v| v.as_i64())
                        .map(|n| n.max(0) as usize)
                        .unwrap_or(20),
                )
                .with_min_collapsed_lines(
                    props
                        .get("min_collapsed_lines")
                        .and_then(|v| v.as_i64())
                        .map(|n| n.max(0) as usize)
                        .unwrap_or(2),
                );
            match diff_inline_unit_of(props.get("inline_emphasis").and_then(|v| v.as_str())) {
                Some(unit) => config.with_inline_unit(Some(unit)),
                None => config.with_inline_unit(None),
            }
        });

        // Bridge the kit's event stream: selection state mirrors into the
        // `selected-lines` property, and the declarative handler fires with
        // the range (or an empty string when the selection clears).
        if let Some(handler) = component.handlers.get("selection_changed").cloned() {
            let runtime = Arc::clone(&self.runtime);
            let sub_id = id.clone();
            let subscription = cx.subscribe_in(
                &state,
                window,
                move |_this: &mut App, _state, event: &DiffEvent, _window, cx| {
                    let record = |value: Option<String>| {
                        if let Ok(mut lm) = runtime.layout_manager.try_write() {
                            let value = value.map(Value::String).unwrap_or(Value::Null);
                            let _ = lm.set_property(&sub_id, "selected_lines", value);
                        }
                    };
                    match event {
                        DiffEvent::SelectionChanged(range) => {
                            let message = range
                                .as_ref()
                                .map(|r| diff_selection_display(Some(r)))
                                .unwrap_or_default();
                            record(range.as_ref().map(|r| diff_selection_display(Some(r))));
                            runtime.call_handler(handler.as_str(), &sub_id, message.as_str());
                            cx.notify();
                        }
                        DiffEvent::SelectionEnded(range) => {
                            let message = diff_selection_display(Some(range));
                            record(Some(message.clone()));
                            runtime.call_handler(handler.as_str(), &sub_id, message.as_str());
                            cx.notify();
                        }
                        DiffEvent::SelectionStarted(_) => {}
                        // Expansion and conflict resolution are kit-side view
                        // state; conflict output has no declarative seam. The
                        // event set is `#[non_exhaustive]`, so stay forward-
                        // compatible with unknown variants.
                        DiffEvent::FileExpanded(_)
                        | DiffEvent::FileCollapsed(_)
                        | DiffEvent::ConflictResolved(..)
                        | _ => {}
                    }
                },
            );
            self._subscriptions.push(subscription);
        }

        self.component_states.insert(
            id.clone(),
            ComponentState::Diff {
                state: state.clone(),
                last_data: current_data.clone(),
                last_schema: schema,
            },
        );
        state
    }

    /// Extracts `<table-column>` children from the table's
    /// `<table-header>` into column definitions. Shared by both table
    /// modes: declared mode renders the header band from them, data mode
    /// feeds them to the virtualized delegate.
    fn extract_declared_columns(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<DeclaredColumn> {
        let mut columns: Vec<DeclaredColumn> = Vec::new();
        for child_id in &component.children {
            let Some(child) = components.get(child_id) else {
                continue;
            };
            if child.component_type != "table_header" {
                continue;
            }
            for col_id in &child.children {
                let Some(col) = components.get(col_id) else {
                    continue;
                };
                if col.component_type != "table_column" {
                    continue;
                }
                let field = col
                    .properties
                    .get("field")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                // The header label: text content, then the `label` attribute.
                let label = col
                    .properties
                    .get("content")
                    .or_else(|| col.properties.get("label"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| field.clone());
                let width = col
                    .properties
                    .get("width")
                    .and_then(|v| v.as_i64())
                    .map(|w| w as f32);
                let align = match col
                    .properties
                    .get("align")
                    .and_then(|v| v.as_str())
                    .unwrap_or("left")
                {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => TextAlign::Left,
                };
                columns.push(DeclaredColumn {
                    field,
                    label,
                    width,
                    align,
                });
            }
        }
        columns
    }

    /// Extracts `<table-header>`/`<table-row>` children into declared-mode
    /// data. Returns `None` when the table has no declared rows or when live
    /// `data` is present (data mode wins so a bound table replaces its
    /// declared placeholder rows once the source delivers).
    fn extract_declared_table(
        &mut self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
        entity_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<DeclaredTableData> {
        // Live data wins over declared rows.
        if let Some(Value::Array(arr)) = component.properties.get("data") {
            if !arr.is_empty() {
                return None;
            }
        }

        let columns = self.extract_declared_columns(component, components);

        // Rows: `<table-row>` children, each holding `<table-cell>` children.
        let mut rows: Vec<DeclaredRow> = Vec::new();
        for child_id in &component.children {
            let Some(child) = components.get(child_id) else {
                continue;
            };
            if child.component_type != "table_row" {
                continue;
            }
            let row_col_span = child
                .properties
                .get("col_span")
                .and_then(|v| v.as_i64())
                .map(|v| v.max(1) as usize)
                .unwrap_or(1);
            let mut cells: Vec<DeclaredCell> = Vec::new();
            for cell_id in &child.children {
                let Some(cell) = components.get(cell_id) else {
                    continue;
                };
                if cell.component_type != "table_cell" {
                    continue;
                }
                // Text-only cells: the `content` property becomes a text
                // child; otherwise the cell's children are the content.
                let content: Vec<AnyElement> =
                    if let Some(Value::String(text)) = cell.properties.get("content") {
                        // Text cells render the string directly. The element id
                        // derives from the CELL's component id — two cells may
                        // hold the same string (e.g. a `"false"` default column),
                        // and gpui's a11y pass panics on duplicate element ids
                        // in debug builds.
                        vec![gpui::Text::new(
                            gpui::ElementId::Name(SharedString::from(format!("{}_text", cell.id))),
                            text.clone().into(),
                        )
                        .into_any_element()]
                    } else {
                        self.render_children(cell, components, entity_id, window, cx)
                    };
                let align = match cell
                    .properties
                    .get("align")
                    .and_then(|v| v.as_str())
                    .unwrap_or("left")
                {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => TextAlign::Left,
                };
                cells.push(DeclaredCell {
                    col_span: cell
                        .properties
                        .get("col_span")
                        .and_then(|v| v.as_i64())
                        .map(|v| v.max(1) as usize)
                        .unwrap_or(1),
                    row_span: cell
                        .properties
                        .get("row_span")
                        .and_then(|v| v.as_i64())
                        .map(|v| v.max(1) as usize)
                        .unwrap_or(1),
                    align,
                    content,
                });
            }
            rows.push(DeclaredRow {
                col_span: row_col_span,
                cells,
            });
        }

        if rows.is_empty() {
            return None;
        }

        Some(DeclaredTableData { columns, rows })
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

    /// Gets or creates the TreeState for a declared-mode tree. The structure
    /// is identical every frame (the XML does not change without a config
    /// reload), so the state is created once from the first extraction and
    /// reused, preserving user expansion/selection across frames.
    fn get_or_create_declared_tree_state(
        &mut self,
        component_id: &str,
        nodes: &[DeclaredTreeNode],
        cx: &mut Context<Self>,
    ) -> Entity<TreeState> {
        if let Some(ComponentState::Tree { state, .. }) =
            self.component_states.get_mut(component_id)
        {
            return state.clone();
        }
        let items = declared_to_tree_items(nodes);
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        self.component_states.insert(
            component_id.to_string(),
            ComponentState::Tree {
                state: state.clone(),
                last_items: Vec::new(),
            },
        );
        state
    }

    /// Extracts a single `<tree-node>`'s declared structure.
    ///
    /// Content children are reduced to plain data the row renderer can build
    /// fresh on every call: `<icon>` children contribute leading/trailing
    /// icon names, `<text>`/`<label>` children contribute the label. This
    /// is required because the kit tree renders each row once per layout
    /// pass and once per paint pass — pre-rendered GPUI elements would be
    /// consumed by the first pass.
    fn extract_declared_tree_node(
        &self,
        node: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Option<DeclaredTreeNode> {
        let mut children_nodes: Vec<DeclaredTreeNode> = Vec::new();
        let mut leading_icon: Option<String> = None;
        let mut trailing_icon: Option<String> = None;
        let mut text_label: Option<String> = None;

        for child_id in &node.children {
            let Some(child) = components.get(child_id) else {
                continue;
            };
            match child.component_type.as_str() {
                "tree_node" => {
                    if let Some(child_node) = self.extract_declared_tree_node(child, components) {
                        children_nodes.push(child_node);
                    }
                }
                "icon" => {
                    // The first icon leads, a second trails the label.
                    if let Some(n) = child.properties.get("name").and_then(|v| v.as_str()) {
                        if leading_icon.is_none() {
                            leading_icon = Some(n.to_string());
                        } else if trailing_icon.is_none() {
                            trailing_icon = Some(n.to_string());
                        }
                    }
                }
                "text" | "label" if text_label.is_none() => {
                    // A text child provides the displayed label.
                    text_label = child
                        .properties
                        .get("content")
                        .or_else(|| child.properties.get("text"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                _ => {}
            }
        }

        // Label precedence: a text/label child, then the element's own text
        // content, then the `label` attribute, then the id.
        let label_attr = node
            .properties
            .get("label")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let own_text = node
            .properties
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let label = text_label
            .or(own_text)
            .or(label_attr)
            .unwrap_or_else(|| node.id.clone());

        Some(DeclaredTreeNode {
            id: node.id.clone(),
            label,
            leading_icon,
            trailing_icon,
            children: children_nodes,
            expanded: node
                .properties
                .get("expanded")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            disabled: node
                .properties
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }

    /// Extracts the tree's direct `<tree-node>` children (recursively).
    fn extract_declared_tree(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Option<Vec<DeclaredTreeNode>> {
        let mut nodes: Vec<DeclaredTreeNode> = Vec::new();
        for child_id in &component.children {
            let Some(child) = components.get(child_id) else {
                continue;
            };
            if child.component_type == "tree_node" {
                if let Some(node) = self.extract_declared_tree_node(child, components) {
                    nodes.push(node);
                }
            }
        }

        if nodes.is_empty() {
            None
        } else {
            Some(nodes)
        }
    }

    /// Gets or creates a `CarouselState` entity for the given `<carousel>`.
    ///
    /// Created once with the slide count, axis, looping, and initial index;
    /// every later render recomputes the props from the component and pushes
    /// only genuine changes in silently (`set_item_count` /
    /// `set_selected_index`, which do not emit `CarouselEvent`). The first
    /// creation also subscribes so that every *user-driven* selection —
    /// swipe, wheel, keyboard, controls — fires the `on-change` handler with
    /// the newly selected index.
    fn get_or_create_carousel_state(
        &mut self,
        component: &BuiltComponent,
        slide_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<CarouselState> {
        let props = &component.properties;
        let axis = match props.get("axis").and_then(|v| v.as_str()) {
            Some("vertical") => gpui::Axis::Vertical,
            _ => gpui::Axis::Horizontal,
        };
        let looping = props
            .get("looping")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let index = props.get("index").and_then(|v| v.as_i64()).unwrap_or(0);

        if let Some(ComponentState::Carousel {
            state,
            last_count,
            last_index,
        }) = self.component_states.get_mut(&component.id)
        {
            let state = state.clone();
            if *last_count != slide_count {
                state.update(cx, |s, cx| s.set_item_count(slide_count, cx));
                *last_count = slide_count;
            }
            if *last_index != index {
                state.update(cx, |s, cx| s.set_selected_index(index.max(0) as usize, cx));
                *last_index = index;
            }
            return state;
        }

        let state = cx.new(|_| {
            CarouselState::new(slide_count)
                .with_axis(axis)
                .with_looping(looping)
                .with_selected_index(index.max(0) as usize)
        });

        // Subscribe once: every user-driven selection (swipe, wheel, keyboard,
        // controls) reports the newly selected index to the app so it can
        // mirror or persist it.
        let on_change = component.handlers.get("change").cloned();
        let component_id = component.id.clone();
        let runtime = Arc::clone(&self.runtime);
        if let Some(on_change) = on_change {
            let subscription = cx.subscribe_in(
                &state,
                window,
                move |_this: &mut App, _state, event: &CarouselEvent, _window, cx| {
                    let CarouselEvent::Change(new_index) = event;
                    runtime.call_handler(&on_change, &component_id, &new_index.to_string());
                    cx.notify();
                },
            );
            self._subscriptions.push(subscription);
        }

        self.component_states.insert(
            component.id.clone(),
            ComponentState::Carousel {
                state: state.clone(),
                last_count: slide_count,
                last_index: index,
            },
        );
        state
    }

    /// Gets or creates a `MessageScrollerState` entity for the given
    /// `<message-scroller>`.
    ///
    /// Created once with the declared row count; a later render that sees a
    /// different row count re-`reset`s the list (the declarative XML model
    /// cannot express incremental `append`/`prepend`/`splice` operations, so
    /// resets intentionally re-engage tail following). The first creation
    /// also observes the entity: scrolling notifies it, and the observation
    /// re-renders the app so the jump button and bottom fade track the
    /// reader leaving and rejoining the live tail.
    fn get_or_create_message_scroller_state(
        &mut self,
        component: &BuiltComponent,
        row_count: usize,
        cx: &mut Context<Self>,
    ) -> Entity<MessageScrollerState> {
        if let Some(ComponentState::MessageScroller { state, last_count }) =
            self.component_states.get_mut(&component.id)
        {
            if *last_count != row_count {
                let count = row_count;
                state.update(cx, |s, cx| s.reset(count, cx));
                *last_count = count;
            }
            return state.clone();
        }

        let state = cx.new(|cx| MessageScrollerState::new(row_count, cx));

        let subscription = cx.observe(&state, move |_this, _state, cx| {
            cx.notify();
        });
        self._subscriptions.push(subscription);

        self.component_states.insert(
            component.id.clone(),
            ComponentState::MessageScroller {
                state: state.clone(),
                last_count: row_count,
            },
        );
        state
    }

    /// Gets or creates a `CalendarState` entity for the given `<calendar>`.
    ///
    /// Created once with the value window/disabled-day restrictions built
    /// from the props; the first creation also subscribes so every selection
    /// writes the ISO date back into the `value` property (making
    /// `get_component_property(id, "value")` live) and fires the `on-change`
    /// handler. Every later render pushes script-driven `value` changes back
    /// in silently via `set_date`.
    fn get_or_create_calendar_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<CalendarState> {
        let props = &component.properties;

        if let Some(ComponentState::Calendar { state, last_value }) =
            self.component_states.get_mut(&component.id)
        {
            let state = state.clone();
            let desired = props
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if *last_value != desired {
                if let Some(day) = parse_date(&desired) {
                    // The selection may already equal `desired` (the event
                    // handler wrote it back after a click) — avoid a spurious
                    // re-notify in that case.
                    let already = match state.read(cx).date() {
                        CalendarDate::Single(Some(current)) => current == day,
                        _ => false,
                    };
                    if !already {
                        state.update(cx, |s, cx| {
                            s.set_date(CalendarDate::Single(Some(day)), window, cx)
                        });
                    }
                }
                *last_value = desired;
            }
            return state;
        }

        let state = cx.new(|cx| CalendarState::new(window, cx));

        // Build the disabled matcher: dates strictly outside the min/max
        // window, plus listed weekdays (0 = Sunday…6 = Saturday, matching
        // the kit's `Matcher::DayOfWeek` indices).
        let (min, max) = (
            props
                .get("min")
                .and_then(|v| v.as_str())
                .and_then(parse_date),
            props
                .get("max")
                .and_then(|v| v.as_str())
                .and_then(parse_date),
        );
        let days: Vec<u32> = props
            .get("disabled_days")
            .and_then(|v| v.as_str())
            .map(|list| {
                list.split(',')
                    .filter_map(|part| part.trim().parse::<u32>().ok())
                    .filter(|day| *day < 7)
                    .collect()
            })
            .unwrap_or_default();
        if min.is_some() || max.is_some() || !days.is_empty() {
            state.update(cx, |s, cx| {
                s.set_disabled_matcher(
                    Matcher::custom(move |date: &chrono::NaiveDate| {
                        (min.is_some() && *date < min.unwrap())
                            || (max.is_some() && *date > max.unwrap())
                            || (!days.is_empty()
                                && days.contains(
                                    &chrono::Datelike::weekday(date).num_days_from_sunday(),
                                ))
                    }),
                    window,
                    cx,
                );
            });
        }
        // Subscribe once: mirror the selected date back into the `value`
        // property and route it to the on-change handler.
        let on_change = component.handlers.get("change").cloned();
        let component_id = component.id.clone();
        let runtime = Arc::clone(&self.runtime);
        if let Some(on_change) = on_change {
            let subscription = cx.subscribe_in(
                &state,
                window,
                move |_this: &mut App, _state, event: &CalendarEvent, _window, cx| {
                    let CalendarEvent::Selected(date) = event;
                    if let Some(iso) = selected_date_string(date) {
                        if let Ok(mut lm) = runtime.layout_manager.try_write() {
                            let _ =
                                lm.set_property(&component_id, "value", Value::String(iso.clone()));
                        }
                        runtime.call_handler(&on_change, &component_id, &iso);
                        cx.notify();
                    }
                },
            );
            self._subscriptions.push(subscription);
        }

        let last_value = props
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let seeded = parse_date(&last_value);
        if let Some(day) = seeded {
            state.update(cx, |s, cx| {
                s.set_date(CalendarDate::Single(Some(day)), window, cx)
            });
        }

        self.component_states.insert(
            component.id.clone(),
            ComponentState::Calendar {
                state: state.clone(),
                last_value,
            },
        );
        state
    }

    /// Gets or creates a `QuestionnaireState` entity for the given
    /// `<questionnaire>`, built from the collected `<questionnaire-item>`
    /// schema.
    ///
    /// The upstream schema is fixed at construction, so the collected
    /// definition signature acts as a cache key: a change re-creates the state
    /// (and the freeform input entities, which need a window). An invalid
    /// schema yields no entity — the wrapper logs the reason at creation and
    /// renders a placeholder.
    ///
    /// The first creation also subscribes: `CurrentItemChanged` mirrors the
    /// active item into the `current` property, `AnswerChanged` mirrors the
    /// snapshot into `answers` and fires `on-change` with the changed item's
    /// serialized answer, and `Submit` fires `on-submit` with the validated
    /// full snapshot (`Completed` only refreshes `answers`, because the kit
    /// emits it once per completion alongside the first `Submit`).
    fn get_or_create_questionnaire_state(
        &mut self,
        component: &BuiltComponent,
        items: &[QuestionnaireItemData],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<QuestionnaireState>> {
        let signature = questionnaire_schema_signature(items);

        if let Some(ComponentState::Questionnaire {
            state,
            last_items,
            last_current,
        }) = self.component_states.get_mut(&component.id)
        {
            // Schema changed: drop the cached state and rebuild below.
            if *last_items != signature {
                self.component_states.remove(&component.id);
            } else {
                // `current-item` property sync: script-driven navigation.
                let desired = component
                    .properties
                    .get("current_item")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !desired.is_empty() && desired != *last_current {
                    let state = state.clone().unwrap();
                    state.update(cx, |s, cx| {
                        let _ = s.set_current_item(&desired, window, cx);
                    });
                    *last_current = desired;
                }
                return state.clone();
            }
        }

        if let Err(reason) = validate_questionnaire_schema(items) {
            tracing::warn!("questionnaire `{}`: {reason}", component.id);
            self.component_states.insert(
                component.id.clone(),
                ComponentState::Questionnaire {
                    state: None,
                    last_items: signature,
                    last_current: String::new(),
                },
            );
            return None;
        }

        // The freeform answer entities exist before the questionnaire state;
        // their placeholders come from the XML.
        let mut inputs = HashMap::new();
        for item in items {
            if let Some(input) = &item.input {
                let entity: Entity<InputState> = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);
                    if let Some(placeholder) = &input.placeholder {
                        state = state.placeholder(placeholder.clone());
                    }
                    state
                });
                inputs.insert(item.name.clone(), entity);
            }
        }

        let definitions: Vec<QuestionnaireItemDefinition> = items
            .iter()
            .map(|item| {
                let mut definition =
                    QuestionnaireItemDefinition::new(item.name.clone(), item.label.clone())
                        .with_required(item.required)
                        .with_multiple(item.multiple)
                        .with_disabled(item.disabled);
                if let Some(description) = &item.description {
                    definition = definition.with_description(description.clone());
                }
                definition = definition.with_choices(item.choices.iter().map(|choice| {
                    let base = QuestionnaireChoiceDefinition::new(
                        choice.value.clone(),
                        choice.label.clone(),
                    )
                    .with_disabled(choice.disabled);
                    let base = if choice.default_selected {
                        base.with_default_selected(true)
                    } else {
                        base
                    };
                    match &choice.description {
                        Some(description) => base.with_description(description.clone()),
                        None => base,
                    }
                }));
                if let Some(input) = &item.input {
                    definition = definition.with_input(
                        QuestionnaireInputDefinition::new(
                            inputs.get(&item.name).unwrap().clone(),
                            input.label.clone(),
                        )
                        .with_disabled(input.disabled),
                    );
                }
                definition
            })
            .collect();

        let desired_current = component
            .properties
            .get("current_item")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let desired_current_known = !desired_current.is_empty()
            && items
                .iter()
                .any(|item| item.name == desired_current && !item.disabled);
        let shortcuts = match component
            .properties
            .get("shortcuts")
            .and_then(|v| v.as_str())
        {
            Some("letters") => Some(QuestionnaireShortcutMode::Letters),
            Some("numbers") => Some(QuestionnaireShortcutMode::Numbers),
            _ => None,
        };

        let state = cx.new(|cx| {
            match QuestionnaireState::new(definitions, cx) {
                Ok(state) => {
                    let state = if let Some(mode) = shortcuts {
                        state.with_shortcuts(mode)
                    } else {
                        state
                    };
                    // `with_current_item` only errors on names the schema
                    // does not define; the existence check above keeps the
                    // fallback unreachable.
                    if desired_current_known {
                        match state.with_current_item(desired_current.clone()) {
                            Ok(state) => state,
                            // The existence check makes this unreachable;
                            // keep the unseeded state on any residual error.
                            Err(_) => unreachable!("current item validated"),
                        }
                    } else {
                        state
                    }
                }
                // Pre-validation keeps this reserved; treat any residual
                // schema failure the same way.
                Err(reason) => {
                    tracing::warn!("questionnaire `{}`: {reason}", component.id);
                    unreachable!("schema pre-validated before entity creation")
                }
            }
        });

        let on_change = component.handlers.get("change").cloned();
        let on_submit = component.handlers.get("submit").cloned();
        let component_id = component.id.clone();
        let runtime = Arc::clone(&self.runtime);
        let subscription = cx.subscribe_in(
            &state,
            window,
            move |this: &mut App, state, event: &QuestionnaireEvent, _window, cx| {
                let write_property = |_this: &mut App, key: &str, value: String| {
                    if let Ok(mut lm) = runtime.layout_manager.try_write() {
                        let _ = lm.set_property(&component_id, key, Value::String(value));
                    }
                };
                match event {
                    QuestionnaireEvent::CurrentItemChanged { current, .. } => {
                        let name = current
                            .as_ref()
                            .map(|name| name.as_ref().to_string())
                            .unwrap_or_default();
                        write_property(this, "current", name.clone());
                        if let Some(ComponentState::Questionnaire { last_current, .. }) =
                            this.component_states.get_mut(&component_id)
                        {
                            *last_current = name;
                        }
                        cx.notify();
                    }
                    QuestionnaireEvent::AnswerChanged(change) => {
                        let snapshot = serialize_questionnaire_answers(&state.read(cx).answers());
                        write_property(this, "answers", snapshot.clone());
                        let item = change.item().as_ref().to_string();
                        if let Some(handler) = &on_change {
                            runtime.call_handler(handler, &component_id, &item);
                            cx.notify();
                        }
                    }
                    // Completed transitions emit alongside the first Submit;
                    // only the snapshot refresh happens here.
                    QuestionnaireEvent::Completed(_) | QuestionnaireEvent::Submit(_) => {
                        let snapshot = serialize_questionnaire_answers(&state.read(cx).answers());
                        write_property(this, "answers", snapshot.clone());
                        if let QuestionnaireEvent::Submit(_) = event {
                            if let Some(handler) = &on_submit {
                                runtime.call_handler(handler, &component_id, snapshot.as_str());
                                cx.notify();
                            }
                        }
                    }
                    // The event enum is `#[non_exhaustive]` upstream.
                    _ => {}
                }
            },
        );
        self._subscriptions.push(subscription);

        let last_current = desired_current;
        self.component_states.insert(
            component.id.clone(),
            ComponentState::Questionnaire {
                state: Some(state.clone()),
                last_items: signature,
                last_current,
            },
        );
        Some(state)
    }

    ///
    /// Gets or creates a `CommandState` entity for the given `<command>`.
    ///
    /// Created once; the query lives inside the kit state and is seeded (and
    /// later re-seeded) from the `query` property. The set-agenda callback
    /// wiring happens per-render in the wrapper, so nothing is subscribed
    /// here — script-driven seeds route through the shared echo flag so the
    /// reflexive `on_query` stays silent.
    fn get_or_create_command_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<CommandState>, Rc<RefCell<Option<String>>>) {
        let desired = component
            .properties
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(ComponentState::Command {
            state,
            last_query,
            echo,
        }) = self.component_states.get_mut(&component.id)
        {
            let state = state.clone();
            let echo = echo.clone();
            if *last_query != desired {
                // set_query fires on_query even for programmatic writes;
                // stage the echo so the callback consumes it silently.
                *echo.borrow_mut() = Some(desired.clone());
                state.update(cx, |s, cx| s.set_query(desired.clone(), window, cx));
                *last_query = desired;
            }
            return (state, echo);
        }

        let state = cx.new(|cx| CommandState::new(window, cx));
        let echo = Rc::new(RefCell::new(None));

        let last_query = desired.clone();
        if !desired.is_empty() {
            *echo.borrow_mut() = Some(desired.clone());
            state.update(cx, |s, cx| s.set_query(desired, window, cx));
        }

        self.component_states.insert(
            component.id.clone(),
            ComponentState::Command {
                state: state.clone(),
                last_query,
                echo: echo.clone(),
            },
        );
        (state, echo)
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

    /// Gets or creates a TextareaState entity configured for a Textarea component.
    fn get_or_create_textarea_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        let id = component.id.clone();

        if let Some(ComponentState::Textarea(state)) = self.component_states.get(&id) {
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
            let mut s = TextareaState::new(window, cx).placeholder(placeholder);

            // Configure sizing: auto_grow takes priority, then rows.
            if let (Some(min), Some(max)) = (auto_grow_min, auto_grow_max) {
                s = s.auto_grow(min as usize, max as usize);
            } else {
                s = s.rows(rows.unwrap_or(4) as usize);
            }

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(id.clone(), ComponentState::Textarea(state.clone()));

        state
    }

    /// Gets or creates an EditorState entity configured for a CodeEditor component.
    fn get_or_create_code_editor_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<EditorState> {
        let id = component.id.clone();

        if let Some(ComponentState::Editor(state)) = self.component_states.get(&id) {
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
        let tab_size_val = props.get("tab_size").and_then(|v| v.as_i64()).unwrap_or(4) as usize;
        let hard_tabs = props
            .get("hard_tabs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let state = cx.new(|cx| {
            let mut s = EditorState::new(window, cx)
                .language(language)
                .line_number(line_number)
                .searchable(searchable)
                .tab_size(TabSize {
                    tab_size: tab_size_val,
                    hard_tabs,
                });

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(id.clone(), ComponentState::Editor(state.clone()));

        state
    }

    /// Gets or creates a TextareaState entity configured for a TextEditor component.
    fn get_or_create_text_editor_state(
        &mut self,
        component: &BuiltComponent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        let id = component.id.clone();

        if let Some(ComponentState::Textarea(state)) = self.component_states.get(&id) {
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
            let mut s = TextareaState::new(window, cx)
                .placeholder(placeholder)
                .rows(rows.unwrap_or(4) as usize);

            if let Some(val) = default_value {
                s = s.default_value(val);
            }
            s
        });

        self.component_states
            .insert(id.clone(), ComponentState::Textarea(state.clone()));

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

        // Render the root component from the snapshot. Stacks are content-sized
        // by default, but the layout root must fill the viewport — inject a
        // truthy `flex` so it grows through the normal grow path.
        if let Some(root_id) = root_id {
            if let Some(root) = components.get(&root_id) {
                let mut root = root.clone();
                root.properties
                    .entry("flex".to_string())
                    .or_insert(Value::Integer(1));
                return self.render_component(&root, &components, entity_id, window, cx);
            }
        }

        // Fallback: empty layout
        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_t(Space::Lg)
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Welcome to Nemo"),
            )
            .child(
                div()
                    .text_t(FontSize::Lg)
                    .text_color(cx.theme().colors.muted_foreground)
                    .child("Configure your application in app.nemo"),
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

    /// Collects declarative `<menu>` children (items, labels, separators, or
    /// nested submenus) into render-ready entry data. Submenu children
    /// recurse; any other child type is ignored by the menu.
    fn collect_menu_entries(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<MenuEntryData> {
        component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter_map(|child| match child.component_type.as_str() {
                "menu_item" => Some(MenuEntryData::Item {
                    id: child.id.clone(),
                    label: child
                        .properties
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    icon: child
                        .properties
                        .get("icon")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    disabled: child
                        .properties
                        .get("disabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    checked: child.properties.get("checked").and_then(|v| v.as_bool()),
                    href: child
                        .properties
                        .get("href")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    on_click: child.handlers.get("click").cloned(),
                }),
                "menu_label" => Some(MenuEntryData::Label(
                    child
                        .properties
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                )),
                "menu_separator" => Some(MenuEntryData::Separator),
                "menu_submenu" => Some(MenuEntryData::Submenu {
                    label: child
                        .properties
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Submenu")
                        .to_string(),
                    icon: child
                        .properties
                        .get("icon")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    items: self.collect_menu_entries(child, components),
                }),
                _ => None,
            })
            .collect()
    }

    /// Collects the declarative `<command-item>`, `<command-group>` and
    /// `<command-separator>` children of a `<command>` palette, mirroring
    /// `collect_menu_entries`.
    fn collect_command_entries(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<CommandEntryData> {
        component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter_map(|child| match child.component_type.as_str() {
                "command_item" => {
                    let keywords = child
                        .properties
                        .get("keywords")
                        .and_then(|v| v.as_str())
                        .map(|list| {
                            list.split(',')
                                .map(|part| part.trim().to_string())
                                .filter(|part| !part.is_empty())
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(CommandEntryData::Item {
                        id: child.id.clone(),
                        label: child
                            .properties
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        icon: child
                            .properties
                            .get("icon")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        keywords,
                        disabled: child
                            .properties
                            .get("disabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        checked: child.properties.get("checked").and_then(|v| v.as_bool()),
                        on_confirm: child.handlers.get("confirm").cloned(),
                    })
                }
                "command_group" => Some(CommandEntryData::Group {
                    label: child
                        .properties
                        .get("label")
                        .and_then(|v| v.as_str())
                        .filter(|label| !label.is_empty())
                        .map(str::to_string),
                    items: self.collect_command_entries(child, components),
                }),
                "command_separator" => Some(CommandEntryData::Separator),
                _ => None,
            })
            .collect()
    }

    /// Collects the declarative `<questionnaire>` schema — `<questionnaire-item>`
    /// children (with `<questionnaire-choice>` sub-children and an optional
    /// `<questionnaire-input>`) — into render-ready definition data. Any other
    /// child type is ignored by the questionnaire.
    fn collect_questionnaire_items(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<QuestionnaireItemData> {
        component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter(|child| child.component_type == "questionnaire_item")
            .map(|item| QuestionnaireItemData {
                name: item
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                label: item
                    .properties
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: item
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .filter(|d| !d.is_empty())
                    .map(str::to_string),
                required: item
                    .properties
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                multiple: item
                    .properties
                    .get("multiple")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                disabled: item
                    .properties
                    .get("disabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                choices: item
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|choice| choice.component_type == "questionnaire_choice")
                    .map(|choice| QuestionnaireChoiceData {
                        value: choice
                            .properties
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        label: choice
                            .properties
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        description: choice
                            .properties
                            .get("description")
                            .and_then(|v| v.as_str())
                            .filter(|d| !d.is_empty())
                            .map(str::to_string),
                        disabled: choice
                            .properties
                            .get("disabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        default_selected: choice
                            .properties
                            .get("default_selected")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    })
                    .collect(),
                input: item
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .find(|input| input.component_type == "questionnaire_input")
                    .map(|input| QuestionnaireInputData {
                        label: input
                            .properties
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        placeholder: input
                            .properties
                            .get("placeholder")
                            .and_then(|v| v.as_str())
                            .filter(|p| !p.is_empty())
                            .map(str::to_string),
                        disabled: input
                            .properties
                            .get("disabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    }),
            })
            .collect()
    }

    /// Collects the four-level `<settings>` hierarchy: pages → groups →
    /// items → fields (with dropdown `<settings-option>` children).
    fn collect_settings_pages(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<SettingsPageData> {
        component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter(|child| child.component_type == "settings_page")
            .map(|page| {
                let props = &page.properties;
                let description = props
                    .get("description")
                    .and_then(|v| v.as_str())
                    .filter(|d| !d.is_empty())
                    .map(str::to_string);
                let icon = props
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .filter(|d| !d.is_empty())
                    .map(str::to_string);
                let default_open = props
                    .get("default_open")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let title = props
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled page")
                    .to_string();

                let groups: Vec<SettingsGroupData> = page
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|child| child.component_type == "settings_group")
                    .map(|group| {
                        let variant = match group.properties.get("variant").and_then(|v| v.as_str())
                        {
                            Some("fill") => Some(GroupBoxVariant::Fill),
                            Some("outline") => Some(GroupBoxVariant::Outline),
                            _ => None,
                        };
                        let items: Vec<SettingsItemData> = group
                            .children
                            .iter()
                            .filter_map(|id| components.get(id))
                            .filter(|child| child.component_type == "settings_item")
                            .map(|item| self.collect_settings_item(item, components))
                            .collect();
                        SettingsGroupData {
                            title: group
                                .properties
                                .get("title")
                                .and_then(|v| v.as_str())
                                .filter(|d| !d.is_empty())
                                .map(str::to_string),
                            description: group
                                .properties
                                .get("description")
                                .and_then(|v| v.as_str())
                                .filter(|d| !d.is_empty())
                                .map(str::to_string),
                            variant,
                            items,
                        }
                    })
                    .collect();

                SettingsPageData {
                    title,
                    description,
                    icon,
                    default_open,
                    groups,
                }
            })
            .collect()
    }

    /// Collects one `<settings-item>` row and its `<settings-field>` child.
    fn collect_settings_item(
        &self,
        item: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> SettingsItemData {
        let props = &item.properties;
        let field = item
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .find(|child| child.component_type == "settings_field")
            .map(|field| self.collect_settings_field(field, components));

        SettingsItemData {
            title: props
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled setting")
                .to_string(),
            description: props
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|d| !d.is_empty())
                .map(str::to_string),
            keywords: props
                .get("keywords")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|keyword| !keyword.is_empty())
                .map(str::to_string)
                .collect(),
            disabled: props
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            vertical: props
                .get("layout")
                .and_then(|v| v.as_str())
                .map(|layout| layout == "vertical")
                .unwrap_or(false),
            field,
        }
    }

    /// Collects one `<settings-field>` control and its `<settings-option>`
    /// children. The declared `value` seeds the value store and `default`
    /// enables the page's reset dial; both parse tolerantly because their
    /// type follows the field's `type`.
    fn collect_settings_field(
        &self,
        field: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> SettingsFieldData {
        let props = &field.properties;
        // The control attribute is `kind`, not `type`: the pipeline reserves
        // the `type` key for the component's own type, and a literal `type`
        // attribute would clobber it (the element would resolve as, e.g.,
        // a `dropdown` component).
        let kind = match props.get("kind").and_then(|v| v.as_str()) {
            Some("checkbox") => SettingsFieldKind::Checkbox,
            Some("input") | Some("text") | Some("string") => SettingsFieldKind::Input,
            Some("number") => SettingsFieldKind::Number,
            Some("dropdown") | Some("select") => SettingsFieldKind::Dropdown,
            _ => SettingsFieldKind::Switch,
        };

        let options: Vec<SettingsOptionData> = field
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter(|child| child.component_type == "settings_option")
            .map(|option| SettingsOptionData {
                value: option
                    .properties
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                label: option
                    .properties
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();

        let initial = match kind {
            SettingsFieldKind::Switch | SettingsFieldKind::Checkbox => {
                SettingValue::Flag(setting_flag_value(props.get("value")))
            }
            SettingsFieldKind::Number => {
                SettingValue::Number(setting_number_value(props.get("value"), 0.0))
            }
            SettingsFieldKind::Input => SettingValue::Text(setting_text_value(props.get("value"))),
            SettingsFieldKind::Dropdown => {
                // The tool stores whatever was declared, but if it is not one
                // of the options the dropdown renders a foreign value; snap
                // to the first declared option instead.
                let text = setting_text_value(props.get("value"));
                let known = options.iter().any(|option| option.value == text);
                SettingValue::Text(if known {
                    text
                } else {
                    options
                        .first()
                        .map(|option| option.value.clone())
                        .unwrap_or(text)
                })
            }
        };
        let default = props.get("default").map(|_| match kind {
            SettingsFieldKind::Switch | SettingsFieldKind::Checkbox => {
                SettingValue::Flag(setting_flag_value(props.get("default")))
            }
            SettingsFieldKind::Number => {
                SettingValue::Number(setting_number_value(props.get("default"), 0.0))
            }
            SettingsFieldKind::Input | SettingsFieldKind::Dropdown => {
                SettingValue::Text(setting_text_value(props.get("default")))
            }
        });

        SettingsFieldData {
            id: field.id.clone(),
            kind,
            property: props
                .get("property")
                .and_then(|v| v.as_str())
                .filter(|p| !p.is_empty())
                .unwrap_or(&field.id)
                .to_string(),
            initial,
            default,
            min: setting_number_value(props.get("min"), f64::MIN),
            max: setting_number_value(props.get("max"), f64::MAX),
            step: setting_number_value(props.get("step"), 1.0),
            options,
            on_change: field.handlers.get("change").cloned(),
        }
    }

    /// Gets or creates the value store entity for a `<settings>`. The store
    /// keeps live field values across renders; a schema-signature change
    /// (XML edit to any collected field) re-seeds it from the declared
    /// `value` attributes. An observer on the store drives the re-render that
    /// repaints the control after a set-value closure commits an edit.
    fn get_or_create_settings_state(
        &mut self,
        component: &BuiltComponent,
        pages: &[SettingsPageData],
        cx: &mut Context<Self>,
    ) -> Entity<SettingValues> {
        let signature = settings_schema_signature(pages);

        if let Some(ComponentState::Settings {
            values,
            last_schema,
        }) = self.component_states.get_mut(&component.id)
        {
            // Same schema: keep the live values (a re-seed would lose edits).
            if *last_schema == signature {
                return values.clone().unwrap();
            }
            // Schema changed: drop the cached store and rebuild below.
            self.component_states.remove(&component.id);
        }

        let mut initial = HashMap::new();
        for page in pages {
            for group in &page.groups {
                for item in &group.items {
                    if let Some(field) = &item.field {
                        initial.insert(field.property.clone(), field.initial.clone());
                    }
                }
            }
        }
        let store = cx.new(|_| SettingValues::new(initial));
        let subscription = cx.observe(&store, |_, _, cx| cx.notify());
        self._subscriptions.push(subscription);

        self.component_states.insert(
            component.id.clone(),
            ComponentState::Settings {
                values: Some(store.clone()),
                last_schema: signature,
            },
        );
        store
    }

    /// Collects declarative `<dock-panel>` children (in declaration order,
    /// which is each region's tab order) into panel specs. Any other child
    /// type is ignored by the dock.
    fn collect_dock_panels(
        &self,
        component: &BuiltComponent,
        components: &HashMap<String, BuiltComponent>,
    ) -> Vec<DockPanelSpec> {
        component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .filter(|child| child.component_type == "dock_panel")
            .map(|panel| {
                let props = &panel.properties;
                DockPanelSpec {
                    id: panel.id.clone(),
                    title: props
                        .get("title")
                        .and_then(|v| v.as_str())
                        .filter(|t| !t.is_empty())
                        .unwrap_or("Untitled panel")
                        .to_string(),
                    icon: props
                        .get("icon")
                        .and_then(|v| v.as_str())
                        .filter(|d| !d.is_empty())
                        .map(str::to_string),
                    placement: dock_placement_value(
                        props.get("placement").and_then(|v| v.as_str()),
                    ),
                    size: props.get("size").and_then(|v| v.as_i64()),
                    active: setting_flag_value(props.get("active")),
                    closable: props
                        .get("closable")
                        .map(|v| setting_flag_value(Some(v)))
                        .unwrap_or(true),
                    visible: props
                        .get("visible")
                        .map(|v| setting_flag_value(Some(v)))
                        .unwrap_or(true),
                }
            })
            .collect()
    }

    /// Gets or creates a `<dock>`'s area plus panel state.
    ///
    /// The area (skinned by the kit's `DockSkin`) is created once and kept
    /// across renders; panel entities are keyed by their `<dock-panel>` id
    /// and rebuilt with the area only when the layout signature changes.
    /// Declarative panel attributes (title, icon, flags) are mirrored into
    /// the live entities every pass without touching the layout, so the
    /// user's own drags, resizes, toggles and tab picks survive.
    fn get_or_create_dock_state(
        &mut self,
        component: &BuiltComponent,
        specs: &[DockPanelSpec],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (
        Entity<KitDockArea>,
        HashMap<String, Entity<DeclarativePanel>>,
    ) {
        let id = component.id.clone();
        let signature = dock_layout_signature(specs);

        let existing = match self.component_states.get(&id) {
            Some(ComponentState::Dock {
                area,
                panels,
                last_layout,
            }) => Some((area.clone(), panels.clone(), last_layout.clone())),
            _ => None,
        };

        let (area, panels) = match existing {
            Some((area, station, last)) if last == signature => {
                // Same layout: mirror the presentation attributes and stop.
                self.mirror_dock_panels(specs, &station, cx);
                (area, station)
            }
            Some((area, station, _)) => {
                // Layout changed: keep the entities whose ids survive, create
                // the missing ones, then re-install the layout (displaced
                // panels hear `on_removed` from the area itself).
                let mut rebuilt = HashMap::new();
                for spec in specs {
                    let entity = match station.get(&spec.id) {
                        Some(entity) => entity.clone(),
                        None => cx.new(|cx| DeclarativePanel::new(spec, cx)),
                    };
                    rebuilt.insert(spec.id.clone(), entity);
                }
                apply_dock_layout(&area, &rebuilt, specs, window, cx);
                self.mirror_dock_panels(specs, &rebuilt, cx);
                if let Some(ComponentState::Dock {
                    panels,
                    last_layout,
                    ..
                }) = self.component_states.get_mut(&id)
                {
                    *panels = rebuilt.clone();
                    *last_layout = signature;
                }
                (area, rebuilt)
            }
            None => {
                // First render: create the skinned area, one panel entity per
                // declared panel, and install the layout.
                let (area, _skin) =
                    DockSkin::dock_area(format!("nemo-dock-{id}"), None, window, cx);
                let mut panels = HashMap::new();
                for spec in specs {
                    let entity = cx.new(|cx| DeclarativePanel::new(spec, cx));
                    panels.insert(spec.id.clone(), entity);
                }
                apply_dock_layout(&area, &panels, specs, window, cx);
                self.component_states.insert(
                    id,
                    ComponentState::Dock {
                        area: area.clone(),
                        panels: panels.clone(),
                        last_layout: signature,
                    },
                );
                (area, panels)
            }
        };

        (area, panels)
    }

    /// Mirrors declarative panel attributes into the live panel entities.
    /// Presentation-only: a changed title or flag must not rebuild the
    /// layout, or the user's tab picks would reset behind the edit.
    fn mirror_dock_panels(
        &self,
        specs: &[DockPanelSpec],
        panels: &HashMap<String, Entity<DeclarativePanel>>,
        cx: &mut Context<Self>,
    ) {
        for spec in specs {
            if let Some(entity) = panels.get(&spec.id) {
                if !entity.read(cx).matches(spec) {
                    entity.update(cx, |panel, _cx| panel.apply(spec));
                }
            }
        }
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
        let max_width = props.get("max_width").and_then(|v| v.as_i64());
        let max_height = props.get("max_height").and_then(|v| v.as_i64());
        // A component grows when it has a truthy `flex` (or, for stacks, scrolls).
        let flex_grows = props
            .get("flex")
            .map(crate::components::flex_is_truthy)
            .unwrap_or(false);
        let scroll = props
            .get("scroll")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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
        let background = props.get("background").and_then(|v| v.as_str());

        // Early return if nothing to apply
        if [
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
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
            && !flex_grows
            && !scroll
            && shadow.is_none()
            && rounded.is_none()
            && background.is_none()
            && border_color.is_none()
        {
            return element;
        }

        let is_panel = component.component_type == "panel";

        // Panels own their decoration (`Panel::render` handles padding,
        // border, shadow, rounding, and background), so those properties get
        // skipped in the wrapper below. Only wrap when there is actual
        // geometry (size, margin, flex, scroll) to apply: adding a wrapper
        // for a decoration-only attribute like `border="0"` inserts an
        // intermediate div into the flex chain that an inner scroll stack
        // depends on, which breaks its scrolling.
        // A carousel's `height` sizes its slide *viewport* (consumed inside
        // `Carousel::render`), but the element also renders a pagination row
        // below the viewport and is sized by its content. Clamping the outer
        // wrapper to `height` would cut the pagination row off (it paints
        // past the wrapper into the following sibling), so `height` is
        // consumed by the component and skipped here.
        let height = if component.component_type == "carousel" {
            None
        } else {
            height
        };

        let has_geometry = [
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
            margin,
            margin_x,
            margin_y,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        ]
        .iter()
        .any(|v| v.is_some())
            || flex_grows
            || scroll;
        if is_panel && !has_geometry {
            return element;
        }

        let mut wrapper = div().flex().flex_col().min_h(px(0.));

        // A scrolling stack must grow so the wrapper doesn't break the flex chain
        // that overflow scrolling depends on. (Truthy `flex` is handled below.)
        if component.component_type == "stack" && scroll {
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
        if let Some(mw) = max_width {
            wrapper = wrapper.max_w(px(mw as f32));
        }
        if let Some(mh) = max_height {
            wrapper = wrapper.max_h(px(mh as f32));
        }
        if flex_grows {
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

        // Decoration (padding, border, rounding, shadow, background) is skipped
        // for panels: `Panel::render` owns its own decoration, so applying it
        // here too would double-decorate (a stray outer border/box). The wrapper
        // still handles geometry (size, margin, flex) for panels.
        if !is_panel {
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
            wrapper = apply_rounded(wrapper, rounded, cx);
            if let Some(bg) = background.and_then(|v| crate::components::resolve_color(v, cx)) {
                wrapper = wrapper.bg(bg);
            }
        } // end !is_panel decoration

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
        // `n:if` / `visible` binding: a component whose `visible` property is
        // explicitly `false` renders as an empty element (occupies no space).
        if component
            .properties
            .get("visible")
            .and_then(|v| v.as_bool())
            == Some(false)
        {
            return div().into_any_element();
        }
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
            "carousel" => {
                // Declarative children are the slides, in order; the state
                // entity (selection, axis, looping) lives in the app and is
                // subscribed to `CarouselEvent::Change` for `on-change`.
                let slides = self.render_children(component, components, entity_id, window, cx);
                let carousel_state =
                    self.get_or_create_carousel_state(component, slides.len(), window, cx);
                Carousel::new(component.clone(), carousel_state, slides).into_any_element()
            }
            "calendar" => {
                // The selected date lives in the app's CalendarState (seeded
                // from the value prop) and mirrors back into `value`; the
                // subscription routes selections to `on-change`.
                let calendar_state = self.get_or_create_calendar_state(component, window, cx);
                Calendar::new(component.clone(), calendar_state).into_any_element()
            }
            "command" => {
                // Entries are declarative children; the query/highlight state
                // lives in the app's CommandState (seeded from the query
                // prop) and the palette re-installs handlers per render.
                let (command_state, echo) = self.get_or_create_command_state(component, window, cx);
                Command::new(component.clone(), command_state, echo)
                    .entries(self.collect_command_entries(component, components))
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "checkbox" => Checkbox::new(component.clone())
                .runtime(Arc::clone(&self.runtime))
                .entity_id(entity_id)
                .into_any_element(),
            "input" => {
                let input_state = self.get_or_create_input_state(component, window, cx);
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
                let options = collect_options(component, components);
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
                    .options(options)
                    .selected_value(sel_state)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "progress" => Progress::new(component.clone()).into_any_element(),
            "image" => Image::new(component.clone()).into_any_element(),
            "svg" => Svg::new(component.clone())
                .runtime(Arc::clone(&self.runtime))
                .entity_id(entity_id)
                .into_any_element(),
            "notification" => Notification::new(component.clone()).into_any_element(),
            "tabs" => {
                // Each <tab-item> child becomes one tab; its label comes from the
                // `label` attribute and its own children form the panel body.
                let item_ids: Vec<String> = component
                    .children
                    .iter()
                    .filter(|id| {
                        components
                            .get(*id)
                            .map(|c| c.component_type == "tab_item")
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                let mut items = Vec::new();
                for item_id in &item_ids {
                    let Some(item) = components.get(item_id) else {
                        continue;
                    };
                    let label = item
                        .properties
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let body = self.render_children(item, components, entity_id, window, cx);
                    items.push(TabItemData { label, body });
                }

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
                    .items(items)
                    .selected_index(tab_state)
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "message" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Message::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "message_scroller" => {
                // Declarative children are the rows, in order; the scroll
                // bookkeeping (tail follow, jump button) lives in the app's
                // MessageScrollerState entity, observed so re-renders track
                // the reader leaving/rejoining the live tail.
                let rows = self.render_children(component, components, entity_id, window, cx);
                let scroller_state =
                    self.get_or_create_message_scroller_state(component, rows.len(), cx);
                MessageScroller::new(component.clone(), scroller_state, rows).into_any_element()
            }
            "questionnaire" => {
                // Declarative children form the schema; the flow state is a
                // kit entity created in `get_or_create_questionnaire_state`
                // (observable answers mirror into `current`/`answers` via its
                // subscription).
                let items = self.collect_questionnaire_items(component, components);
                let state = self.get_or_create_questionnaire_state(component, &items, window, cx);
                Questionnaire::new(component.clone(), items, state).into_any_element()
            }
            "settings" => {
                // Declarative children are the whole hierarchy (pages →
                // groups → items → fields); the field value store entity is
                // created in `get_or_create_settings_state`, and field get/set
                // closures read and write it (the store's observer repaints
                // the controls after an edit commits).
                let pages = self.collect_settings_pages(component, components);
                let values = self.get_or_create_settings_state(component, &pages, cx);
                Settings::new(component.clone(), pages)
                    .values(Some(values))
                    .runtime(Arc::clone(&self.runtime))
                    .into_any_element()
            }
            "dock" => {
                // Declarative <dock-panel> children name the panels; each
                // panel's body is its own declarative children, rebuilt every
                // pass into the panel entity's body slot (AnyElement is
                // one-shot, so the entity owns a same-frame stash the app's
                // render pass refills and the panel's paint drains). The area
                // entity and its layout live in `get_or_create_dock_state`.
                let specs = self.collect_dock_panels(component, components);
                if specs.is_empty() {
                    return Dock::new(component.clone()).into_any_element();
                }
                let (area, panels) = self.get_or_create_dock_state(component, &specs, window, cx);
                for spec in &specs {
                    let Some(entity) = panels.get(&spec.id) else {
                        continue;
                    };
                    let Some(panel) = component
                        .children
                        .iter()
                        .filter_map(|id| components.get(id))
                        .find(|child| child.component_type == "dock_panel" && child.id == spec.id)
                    else {
                        continue;
                    };
                    let children = self.render_children(panel, components, entity_id, window, cx);
                    let body = div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h(px(0.))
                        .children(children);
                    entity.update(cx, |panel, _cx| panel.set_body(body.into_any_element()));
                }
                Dock::new(component.clone())
                    .area(Some(area))
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
                // Declared mode: header/row children author the content.
                // Data mode: `data` array via binding, rendered through the
                // virtualized kit table. When both are present, live data
                // wins once it arrives; declared `<table-column>` children
                // feed the data table's column definitions either way.
                let declared_columns = self.extract_declared_columns(component, components);
                if let Some(declared) =
                    self.extract_declared_table(component, components, entity_id, window, cx)
                {
                    Table::new(component.clone())
                        .declared(declared)
                        .into_any_element()
                } else {
                    let table_state =
                        self.get_or_create_table_state(component, &declared_columns, window, cx);
                    Table::new(component.clone())
                        .table_state(table_state)
                        .into_any_element()
                }
            }
            "data_table" => {
                // Columns come from declared `<data-table-column>` children
                // (or are auto-detected from the first data row); the
                // delegate + kit table state live in the component's state.
                let columns = self.collect_data_table_columns(component, components);
                let data_table_state =
                    self.get_or_create_data_table_state(component, &columns, window, cx);
                DataTable::new(component.clone())
                    .table_state(data_table_state)
                    .into_any_element()
            }
            "diff" => {
                // The patch text in `diff` is parsed into files/hunks/paired
                // lines once per change; mode/context/expansion/inline-
                // emphasis configuration rebuilds the state. A malformed
                // patch renders its located parse message instead of the
                // view.
                let parse_error = self.diff_errors.get(&component.id).cloned();
                let diff_state = self.get_or_create_diff_state(component, window, cx);
                Diff::new(component.clone())
                    .diff_state(diff_state)
                    .parse_error(parse_error)
                    .into_any_element()
            }
            "list" => {
                // Each <list-item> child is a row; its own children are the content.
                let item_ids: Vec<String> = component
                    .children
                    .iter()
                    .filter(|id| {
                        components
                            .get(*id)
                            .map(|c| c.component_type == "list_item")
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                let mut items = Vec::new();
                for item_id in &item_ids {
                    let Some(item) = components.get(item_id) else {
                        continue;
                    };
                    items.push(self.render_children(item, components, entity_id, window, cx));
                }

                List::new(component.clone()).items(items).into_any_element()
            }
            "tree" => {
                // Declared mode: <tree-node> children author the structure.
                // Data mode: `items` array via binding. Live items win once
                // delivered (same precedence as the table).
                let has_live_items = match component.properties.get("items") {
                    Some(Value::Array(arr)) => !arr.is_empty(),
                    _ => false,
                };
                let declared_nodes = if has_live_items {
                    None
                } else {
                    self.extract_declared_tree(component, components)
                };

                let tree_state = match declared_nodes.as_ref() {
                    // Structure goes into the TreeState once (created on
                    // first render, reused after so user expansion state
                    // survives); content is rebuilt every frame inside the
                    // row closure.
                    Some(nodes) => self.get_or_create_declared_tree_state(&component.id, nodes, cx),
                    None => self.get_or_create_tree_state(component, window, cx),
                };

                let mut tree = Tree::new(component.clone()).tree_state(tree_state);
                if let Some(nodes) = declared_nodes {
                    tree = tree.declared(nodes);
                }
                tree.into_any_element()
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
                // Each <accordion-item> child becomes one section; the item's
                // own children are rendered as that section's panel body.
                let item_ids: Vec<String> = component
                    .children
                    .iter()
                    .filter(|id| {
                        components
                            .get(*id)
                            .map(|c| c.component_type == "accordion_item")
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                let mut items = Vec::new();
                let mut initial_open = HashSet::new();
                for (ix, item_id) in item_ids.iter().enumerate() {
                    let Some(item) = components.get(item_id) else {
                        continue;
                    };
                    let title = item
                        .properties
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if item
                        .properties
                        .get("open")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        initial_open.insert(ix);
                    }
                    let body = self.render_children(item, components, entity_id, window, cx);
                    items.push(AccordionItemData { title, body });
                }

                let acc_state = self
                    .component_states
                    .get_or_create_accordion_state(&component.id, initial_open);
                Accordion::new(component.clone())
                    .items(items)
                    .open_indices(acc_state)
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "accordion_item" | "tab_item" | "option" | "menu_item" | "list_item"
            | "attachment_media" | "attachment_content" | "attachment_actions"
            | "bubble_reactions" | "menu_label" | "menu_separator" | "menu_submenu"
            | "table_header" | "table_column" | "table_row" | "table_cell" | "tree_node" => {
                // These are rendered by their parent container; standalone fallback.
                div().into_any_element()
            }
            "alert" => Alert::new(component.clone()).into_any_element(),
            "avatar" => Avatar::new(component.clone()).into_any_element(),
            "badge" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Badge::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "bubble" => {
                // Partition the children: content elements go into the
                // bubble's content slot; a `<bubble-reactions>` child is
                // collected as data and attached to the bubble edge.
                let mut children = Vec::new();
                let mut reactions = None;
                for child_id in &component.children {
                    let Some(child) = components.get(child_id) else {
                        continue;
                    };
                    if child.component_type == "bubble_reactions" {
                        let data = BubbleReactionsData {
                            side: child
                                .properties
                                .get("side")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            alignment: child
                                .properties
                                .get("alignment")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            children: self
                                .render_children(child, components, entity_id, window, cx),
                        };
                        reactions = Some(data);
                    } else {
                        children
                            .push(self.render_component(child, components, entity_id, window, cx));
                    }
                }
                let mut bubble = Bubble::new(component.clone()).children(children);
                if let Some(data) = reactions {
                    bubble = bubble.reactions(data);
                }
                bubble.into_any_element()
            }
            "attachment" => {
                // Partition the children into the attachment's typed slots:
                // `<attachment-media>` and `<attachment-content>` collect as
                // data; `<attachment-actions>` renders its children as normal
                // elements (buttons keep their own handlers via the generic
                // child path). Anything else joins the content slot.
                let mut media: Option<AttachmentMediaData> = None;
                let mut content: Option<AttachmentContentData> = None;
                let mut action_children: Vec<AnyElement> = Vec::new();
                for child_id in &component.children {
                    let Some(child) = components.get(child_id) else {
                        continue;
                    };
                    match child.component_type.as_str() {
                        "attachment_media" => {
                            media = Some(AttachmentMediaData {
                                src: child
                                    .properties
                                    .get("src")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                icon: child
                                    .properties
                                    .get("icon")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                children: self
                                    .render_children(child, components, entity_id, window, cx),
                            });
                        }
                        "attachment_content" => {
                            content = Some(AttachmentContentData {
                                title: child
                                    .properties
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                description: child
                                    .properties
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                children: self
                                    .render_children(child, components, entity_id, window, cx),
                            });
                        }
                        "attachment_actions" => action_children
                            .extend(self.render_children(child, components, entity_id, window, cx)),
                        _ => {
                            content
                                .get_or_insert_with(AttachmentContentData::default)
                                .children
                                .push(
                                    self.render_component(child, components, entity_id, window, cx),
                                );
                        }
                    }
                }
                let mut attachment = Attachment::new(component.clone());
                if let Some(media) = media {
                    attachment = attachment.media(media);
                }
                if let Some(content) = content {
                    attachment = attachment.content(content);
                }
                attachment
                    .action_children(action_children)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "bubble_group" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                BubbleGroup::new(component.clone())
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
            "dropdown_button" => {
                let items: Vec<MenuItem> = component
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|c| c.component_type == "menu_item")
                    .map(|c| MenuItem {
                        id: c.id.clone(),
                        label: c
                            .properties
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        on_click: c.handlers.get("click").cloned(),
                    })
                    .collect();
                DropdownButton::new(component.clone())
                    .items(items)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "radio" => {
                let options = collect_options(component, components);
                let initial_val = component.properties.get("value").and_then(|v| v.as_str());
                let initial_ix =
                    initial_val.and_then(|val| options.iter().position(|o| o.value == val));
                let radio_state = self
                    .component_states
                    .get_or_create_selected_index(&component.id, initial_ix);
                Radio::new(component.clone())
                    .options(options)
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
            "app_shell" => {
                // Resolve the three region markers by type.
                let region = |ty: &str| {
                    component
                        .children
                        .iter()
                        .filter_map(|id| components.get(id))
                        .find(|c| c.component_type == ty)
                        .cloned()
                };
                let sidenav_region = region("app_sidenav");
                let content_region = region("app_content");
                let footer_region = region("app_footer");

                // Sidenav items (raw — the shell renders them itself), plus any
                // other children (e.g. <nav-link>, <label>) rendered through the
                // normal path so the shell composes with the router.
                let sidenav_items: Vec<BuiltComponent> = sidenav_region
                    .as_ref()
                    .map(|nav| {
                        nav.children
                            .iter()
                            .filter_map(|id| components.get(id))
                            .filter(|c| c.component_type == "sidenav_item")
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let sidenav_children: Vec<AnyElement> = sidenav_region
                    .as_ref()
                    .map(|nav| {
                        nav.children
                            .iter()
                            .filter_map(|id| components.get(id))
                            .filter(|c| c.component_type != "sidenav_item")
                            .map(|c| self.render_component(c, components, entity_id, window, cx))
                            .collect()
                    })
                    .unwrap_or_default();

                // Content pages; the active one is chosen by shared state.
                let pages: Vec<BuiltComponent> = content_region
                    .as_ref()
                    .map(|content| {
                        content
                            .children
                            .iter()
                            .filter_map(|id| components.get(id))
                            .filter(|c| c.component_type == "page")
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                // With <page> children, drive built-in page switching; otherwise
                // render the content region as-is (e.g. a <router> owns switching).
                let (content_children, active_state) = if pages.is_empty() {
                    let children = content_region
                        .as_ref()
                        .map(|content| {
                            self.render_children(content, components, entity_id, window, cx)
                        })
                        .unwrap_or_default();
                    (children, None)
                } else {
                    let default_target = pages.first().map(|p| p.id.clone()).unwrap_or_default();
                    // `default_page` names the initially-active page (deep-linking
                    // into a section); it must match a `<page id="...">`.
                    let default_target = component
                        .properties
                        .get("default_page")
                        .and_then(|v| v.as_str())
                        .filter(|name| pages.iter().any(|p| &p.id == name))
                        .map(|s| s.to_string())
                        .unwrap_or(default_target);
                    let state = self
                        .component_states
                        .get_or_create_selected_value(&component.id, default_target);
                    let active = state.lock().unwrap().clone();
                    // Render only the active page's body (fall back to the first).
                    let active_page = pages
                        .iter()
                        .find(|p| p.id == active)
                        .or_else(|| pages.first())
                        .cloned();
                    let children = active_page
                        .map(|page| self.render_children(&page, components, entity_id, window, cx))
                        .unwrap_or_default();
                    (children, Some(state))
                };

                let footer_children = footer_region
                    .map(|footer| self.render_children(&footer, components, entity_id, window, cx))
                    .unwrap_or_default();

                let mut shell = AppShell::new(component.clone())
                    .sidenav_items(sidenav_items)
                    .sidenav_children(sidenav_children)
                    .content_children(content_children)
                    .footer_children(footer_children)
                    .entity_id(entity_id)
                    .runtime(Arc::clone(&self.runtime));
                if let Some(state) = active_state {
                    shell = shell.active_state(state);
                }
                shell.into_any_element()
            }
            "router" => {
                let router_id = component.id.clone();
                let default_path = component
                    .properties
                    .get("default")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/")
                    .to_string();

                // Collect <route> children in document order.
                let routes: Vec<BuiltComponent> = component
                    .children
                    .iter()
                    .filter_map(|id| components.get(id))
                    .filter(|c| c.component_type == "route")
                    .cloned()
                    .collect();
                let patterns: Vec<String> = routes
                    .iter()
                    .map(|r| {
                        r.properties
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    })
                    .collect();

                let current = self.runtime.router_current_path(&router_id, &default_path);
                let resolved = crate::containers::router::resolve_route(&patterns, &current);
                let params = resolved
                    .as_ref()
                    .map(|(_, p)| p.clone())
                    .unwrap_or_default();
                // Keep data.route.<id>.* in sync for get_data / bindings.
                self.runtime
                    .sync_route_projection(&router_id, &current, &params);

                // Render only the active route's body (empty if nothing matches).
                let body = resolved
                    .and_then(|(idx, _)| routes.get(idx).cloned())
                    .map(|route| self.render_children(&route, components, entity_id, window, cx))
                    .unwrap_or_default();

                Router::new(body).into_any_element()
            }
            "nav_link" => {
                let route = component
                    .properties
                    .get("route")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let router_ref = component
                    .properties
                    .get("router")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Active when this link's route is the target router's current
                // path. Uses a non-initializing peek so it never creates routers.
                let router_id = router_ref
                    .clone()
                    .or_else(|| self.runtime.primary_router_id());
                let is_active = router_id
                    .and_then(|rid| self.runtime.router_current_path_peek(&rid))
                    .map(|current| current == route)
                    .unwrap_or(false);

                NavLink::new(component)
                    .active(is_active)
                    .entity_id(entity_id)
                    .runtime(Arc::clone(&self.runtime))
                    .into_any_element()
            }
            // Region/leaf markers rendered by their parent (app_shell, router).
            "app_sidenav" | "app_content" | "app_footer" | "sidenav_item" | "page" | "route" => {
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
            "kbd" => Kbd::new(component.clone()).into_any_element(),
            "marker" => {
                let children = self.render_children(component, components, entity_id, window, cx);
                Marker::new(component.clone())
                    .children(children)
                    .into_any_element()
            }
            "skeleton" => Skeleton::new(component.clone()).into_any_element(),
            "stepper" => {
                // Each <stepper-item> child is one step; its children are the
                // step's label content and its icon/disabled attributes carry
                // into the kit item.
                let item_ids: Vec<String> = component
                    .children
                    .iter()
                    .filter(|id| {
                        components
                            .get(*id)
                            .map(|c| c.component_type == "stepper_item")
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                let mut items = Vec::new();
                for item_id in &item_ids {
                    let Some(item) = components.get(item_id) else {
                        continue;
                    };
                    items.push(StepperItemData {
                        label: item
                            .properties
                            .get("label")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        icon: item
                            .properties
                            .get("icon")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        disabled: item
                            .properties
                            .get("disabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        children: self.render_children(item, components, entity_id, window, cx),
                    });
                }

                Stepper::new(component.clone())
                    .items(items)
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "pagination" => {
                // The current page is application-owned: `on-click` fires the
                // handler with the 1-based target page for the app to persist.
                Pagination::new(component.clone())
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "menu" => {
                // Entries are declarative children; collect them pre-render
                // (the dropdown builds its PopupMenu lazily while opening).
                Menu::new(component.clone())
                    .entries(self.collect_menu_entries(component, components))
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
            "rating" => {
                // The kit owns the value in its keyed state (seeded from the
                // value prop); on-click reports the new value to persist.
                Rating::new(component.clone())
                    .runtime(Arc::clone(&self.runtime))
                    .entity_id(entity_id)
                    .into_any_element()
            }
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

/// Collects `<option>` children of a Select/Radio into `OptionData`
/// (`value`, with `label` defaulting to `value`).
fn collect_options(
    component: &BuiltComponent,
    components: &HashMap<String, BuiltComponent>,
) -> Vec<OptionData> {
    component
        .children
        .iter()
        .filter_map(|id| components.get(id))
        .filter(|c| c.component_type == "option")
        .map(|c| {
            let value = c
                .properties
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let label = c
                .properties
                .get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.clone());
            OptionData { value, label }
        })
        .collect()
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Header bar is rendered by AppLayout; App only renders layout content.
        self.render_layout(window, cx)
    }
}
