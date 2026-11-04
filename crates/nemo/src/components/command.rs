use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, Entity, EntityId, IntoElement};
use gpui_kit::component::command::{
    Command as GpuiCommand, CommandGroup, CommandItem, CommandState,
};
use gpui_kit::component::{Disableable as _, Icon as KitIcon};
use nemo_config::Value;
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;
use crate::runtime::NemoRuntime;

/// One declarative child of a `<command>`, collected pre-render like menu
/// entries. Item confirm handlers are keyed by their kit model coordinates
/// (see `confirm_targets`): ungrouped items share section 0 with a
/// cross-tree row counter; group items sit at their group's section (offset
/// by the presence of ungrouped items) with the row being the item's own
/// index within the group.
#[derive(Clone)]
pub enum CommandEntryData {
    Item {
        id: String,
        label: String,
        icon: Option<String>,
        keywords: Vec<String>,
        disabled: bool,
        checked: Option<bool>,
        on_confirm: Option<String>,
    },
    Group {
        label: Option<String>,
        items: Vec<CommandEntryData>,
    },
    Separator,
}

/// A command palette (gpui-kit `Command`): a search field over a filtered,
/// grouped list. Interaction state (query, focus, highlight) lives in a
/// shared `CommandState`; the palette re-installs its entries and callbacks
/// on every render, so handlers wire through element-level closures:
/// `on-query` receives the query string, `on-select`/`on-confirm` receive
/// the item's `section:row` model coordinates, an item's own `on-confirm`
/// receives `confirm`, and `on-cancel` receives `cancel`. The query mirrors
/// back into the `query` property for scripts to read, and a script-driven
/// re-seed re-filters silently (the echo suppresses the reflexive
/// `on_query` that kit's programmatic `set_query` otherwise fires).
///
/// ```xml
/// <command id="palette" placeholder="Type a command..."
///          on-confirm="handle_confirm">
///   <command-item id="open" label="Open File" icon="file" />
///   <command-separator />
///   <command-group id="sg" label="Settings">
///     <command-item id="profile" label="Profile" icon="user" />
///   </command-group>
/// </command>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `query` | string | Search query; mirrors back while typing, re-seeds on change |
/// | `placeholder` | string | Search-field placeholder |
/// | `searchable` | bool | Show the query field + local filtering (default true) |
/// | `filterable` | bool | Keep the field but skip local filtering (default true) |
/// | `max-height` | integer | List height cap in px (default 300) |
/// | `bordered` | bool | Draw the surrounding frame (default true) |
/// | `on-query` | string | Handler invoked with the query on each change |
/// | `on-select` | string | Handler invoked with `section:row` when the highlight moves |
/// | `on-confirm` | string | Handler invoked with `section:row` of the confirmed item |
/// | `on-cancel` | string | Handler invoked when Escape cancels the palette |
#[derive(IntoElement)]
pub struct Command {
    source: BuiltComponent,
    entries: Vec<CommandEntryData>,
    state: Entity<CommandState>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
    /// Set right before a script-driven `set_query` (which fires `on_query`
    /// even for programmatic writes): the callback sees its own echo and
    /// stays silent. Only the query callback consumes it, so it is `None`
    /// between `get_or_create_command_state` and the clone stored in
    /// `ComponentState::Command`.
    echo: Rc<RefCell<Option<String>>>,
}

impl Command {
    pub fn new(
        source: BuiltComponent,
        state: Entity<CommandState>,
        echo: Rc<RefCell<Option<String>>>,
    ) -> Self {
        Self {
            source,
            entries: Vec::new(),
            state,
            runtime: None,
            entity_id: None,
            echo,
        }
    }

    pub fn entries(mut self, entries: Vec<CommandEntryData>) -> Self {
        self.entries = entries;
        self
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

/// Item-level confirm handlers keyed by the kit's model coordinates
/// (`IndexPath.section`, `IndexPath.row`): ungrouped items occupy section 0
/// with a row counter spanning the whole tree, explicit groups shift by one
/// section when ungrouped items exist (the kit's mixed-entry rule), and the
/// row is the item's own index within its group (counting disabled items).
/// Filtered-out items keep their original coordinates.
fn confirm_targets(data: &[CommandEntryData]) -> HashMap<(usize, usize), (String, String)> {
    let mut map = HashMap::new();
    let mut ungrouped = 0usize;
    let mut groups = 0usize;

    for entry in data {
        match entry {
            CommandEntryData::Item { id, on_confirm, .. } => {
                if let Some(handler) = on_confirm {
                    map.insert((0, ungrouped), (id.clone(), handler.clone()));
                }
                ungrouped += 1;
            }
            CommandEntryData::Group { items, .. } => {
                let section = match ungrouped {
                    0 => groups,
                    _ => groups + 1,
                };
                groups += 1;
                for (row, item) in items.iter().enumerate() {
                    if let CommandEntryData::Item {
                        id,
                        on_confirm: Some(handler),
                        ..
                    } = item
                    {
                        map.insert((section, row), (id.clone(), handler.clone()));
                    }
                }
            }
            CommandEntryData::Separator => {}
        }
    }
    map
}

/// Builds one kit item from collected fields. Confirm handlers were already
/// folded into `confirm_targets`, so the kit item itself stays handler-free
/// (the model-level `on_confirm` routes by coordinates).
fn kit_item(
    label: String,
    icon: Option<String>,
    keywords: Vec<String>,
    disabled: bool,
    checked: Option<bool>,
) -> CommandItem {
    let mut item = CommandItem::new().label(label);
    if let Some(icon) = icon {
        item = item.icon(KitIcon::new(map_icon_name(&icon)));
    }
    if !keywords.is_empty() {
        item = item.keywords(keywords);
    }
    if disabled {
        item = item.disabled(true);
    }
    if let Some(checked) = checked {
        item = item.checked(checked);
    }
    item
}

impl gpui::RenderOnce for Command {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let props = &self.source.properties;
        let component_id = self.source.id.clone();
        let entity_id = self.entity_id;
        let runtime = self.runtime;
        let echo = self.echo;

        let targets = confirm_targets(&self.entries);

        // Textual props before the entries loop consumes `self.entries`
        // (RenderOnce fields move out — clone/consume order matters here).
        let mut search = GpuiCommand::new(&self.state)
            .searchable(
                props
                    .get("searchable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            )
            .filterable(
                props
                    .get("filterable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            )
            .bordered(
                props
                    .get("bordered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            )
            .max_h(px(props
                .get("max_height")
                .and_then(|v| v.as_i64())
                .unwrap_or(300) as f32));
        if let Some(placeholder) = props.get("placeholder").and_then(|v| v.as_str()) {
            search = search.placeholder(placeholder.to_string());
        }

        // Entries: items, groups, and separators in declarative order.
        for entry in self.entries {
            match entry {
                CommandEntryData::Item {
                    label,
                    icon,
                    keywords,
                    disabled,
                    checked,
                    ..
                } => {
                    search = search.item(kit_item(label, icon, keywords, disabled, checked));
                }
                CommandEntryData::Group { label, items } => {
                    let mut group = match label {
                        Some(label) => CommandGroup::new().label(label),
                        None => CommandGroup::new(),
                    };
                    for item in items {
                        if let CommandEntryData::Item {
                            label,
                            icon,
                            keywords,
                            disabled,
                            checked,
                            ..
                        } = item
                        {
                            group = group.item(kit_item(label, icon, keywords, disabled, checked));
                        }
                    }
                    search = search.group(group);
                }
                CommandEntryData::Separator => search = search.separator(),
            }
        }

        // Kit `on_query` fires on each typed change AND after a script-driven
        // `set_query`. The echo consumes the programmatic echo so scripts
        // resetting their own palette don't fire their handler reflexively;
        // user-driven changes fire `on-query` with the query string (the
        // `query` property is written back so scripts can read it).
        let query_handler = self.source.handlers.get("query").cloned();
        let query_runtime = runtime.clone();
        let query_id = component_id.clone();
        search = search.on_query(move |query, _window, cx| {
            {
                let mut echo = echo.borrow_mut();
                if echo.as_deref() == Some(query) {
                    *echo = None;
                    return;
                }
            }
            if let Some(runtime) = &query_runtime {
                if let Ok(mut lm) = runtime.layout_manager.try_write() {
                    let _ = lm.set_property(&query_id, "query", Value::String(query.to_string()));
                }
                if let Some(handler) = &query_handler {
                    runtime.call_handler(handler, &query_id, query);
                }
                if let Some(entity_id) = entity_id {
                    cx.notify(entity_id);
                }
            }
        });

        if let Some(handler) = self.source.handlers.get("select").cloned() {
            let select_runtime = runtime.clone();
            let select_id = component_id.clone();
            search = search.on_select(move |path, _window, cx| {
                if let Some(runtime) = &select_runtime {
                    runtime.call_handler(
                        &handler,
                        &select_id,
                        &format!("{}:{}", path.section, path.row),
                    );
                    if let Some(entity_id) = entity_id {
                        cx.notify(entity_id);
                    }
                }
            });
        }

        if let Some(handler) = self.source.handlers.get("confirm").cloned() {
            let confirm_runtime = runtime.clone();
            let confirm_id = component_id.clone();
            search = search.on_confirm(move |path, _window, cx| {
                if let Some(runtime) = &confirm_runtime {
                    runtime.call_handler(
                        &handler,
                        &confirm_id,
                        &format!("{}:{}", path.section, path.row),
                    );
                    // The item's own `on-confirm` (nice ids over coordinates).
                    if let Some((component_id, item_handler)) =
                        targets.get(&(path.section, path.row))
                    {
                        runtime.call_handler(item_handler, component_id, "confirm");
                    }
                    if let Some(entity_id) = entity_id {
                        cx.notify(entity_id);
                    }
                }
            });
        }

        if let Some(handler) = self.source.handlers.get("cancel").cloned() {
            let cancel_runtime = runtime.clone();
            let cancel_id = component_id.clone();
            search = search.on_cancel(move |_window, cx| {
                if let Some(runtime) = &cancel_runtime {
                    runtime.call_handler(&handler, &cancel_id, "cancel");
                    if let Some(entity_id) = entity_id {
                        cx.notify(entity_id);
                    }
                }
            });
        }

        search
    }
}
