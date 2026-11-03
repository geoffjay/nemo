use gpui::*;
use gpui_kit::component::button::{Button as GpuiButton, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Disableable, Icon as KitIcon, Sizable, Size as ComponentSize};
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;
use crate::runtime::NemoRuntime;
use std::sync::Arc;

/// One declarative child of a `<menu>`, collected pre-render like stepper
/// items. Items may be positioned by a `checked` state and carry an optional
/// external `href` (opens in the system browser); a submenu nests further
/// entries.
#[derive(Clone)]
pub enum MenuEntryData {
    Item {
        id: String,
        label: String,
        icon: Option<String>,
        disabled: bool,
        checked: Option<bool>,
        href: Option<String>,
        on_click: Option<String>,
    },
    Label(String),
    Separator,
    Submenu {
        label: String,
        icon: Option<String>,
        items: Vec<MenuEntryData>,
    },
}

/// A button trigger that opens a popup menu (gpui-kit `PopupMenu`). Entries
/// come from `<menu-item>`, `<menu-label>`, `<menu-separator>`, and
/// `<menu-submenu>` children; actions are GPUI Action-based upstream, so item
/// behavior here goes through hander wiring instead (keyboard shortcut hints
/// need the action system and are out of scope).
///
/// ```xml
/// <menu id="actions" label="Actions" variant="outline">
///   <menu-item id="open" icon="folder-open" label="Open" on-click="open" />
///   <menu-separator />
///   <menu-item id="delete" label="Delete" disabled="true" />
/// </menu>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `label` | string | Trigger button text |
/// | `variant` | string | `primary`, `secondary`, `danger`, or `ghost` |
/// | `size` | string | `xsmall`, `small`, `medium` (default), or `large` |
/// | `anchor` | string | `top-left` (default), `top-right`, `bottom-left`, or `bottom-right` |
/// | `scrollable` | bool | Scroll the menu when it exceeds `max-height` |
/// | `max-height` | integer | Menu height cap in px |
/// | `disabled` | bool | Disable the trigger button |
#[derive(IntoElement)]
pub struct Menu {
    source: BuiltComponent,
    entries: Vec<MenuEntryData>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Menu {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            entries: Vec::new(),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn entries(mut self, entries: Vec<MenuEntryData>) -> Self {
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

    fn size_from(value: Option<&str>) -> ComponentSize {
        match value {
            Some("xsmall") => ComponentSize::XSmall,
            Some("small") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        }
    }
}

fn anchor_from(value: Option<&str>) -> Anchor {
    match value {
        Some("top-right") => Anchor::TopRight,
        Some("bottom-left") => Anchor::BottomLeft,
        Some("bottom-right") => Anchor::BottomRight,
        _ => Anchor::TopLeft,
    }
}

/// Interpret one collected `<menu-item>` entry (the checked state, the
/// external link target, and the click handler wire in through the kit item).
fn item_from(
    entry: &MenuEntryData,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
) -> PopupMenuItem {
    let MenuEntryData::Item {
        id,
        label,
        icon,
        disabled,
        checked,
        href,
        on_click,
    } = entry
    else {
        unreachable!("item_from is only called on MenuEntryData::Item");
    };

    let mut item = match href {
        Some(href) => PopupMenuItem::link(label.clone(), href.clone()),
        None => PopupMenuItem::new(label.clone()),
    };
    if let Some(icon) = icon {
        item = item.icon(KitIcon::new(map_icon_name(icon)));
    }
    if *disabled {
        item = item.disabled(true);
    }
    if let Some(checked) = checked {
        item = item.checked(*checked);
    }
    if let (Some(handler), Some(runtime), Some(entity_id)) = (on_click.clone(), runtime, entity_id)
    {
        let item_id = id.clone();
        item = item.on_click(move |_event, _window, cx| {
            runtime.call_handler(&handler, &item_id, "click");
            cx.notify(entity_id);
        });
    }
    item
}

/// Build the kit menu recursively from collected entries. Lives outside the
/// struct so the dropdown builder closure (an `Fn`, rebuilt per render) can
/// call it repeatedly.
fn apply_entries(
    menu: PopupMenu,
    entries: &[MenuEntryData],
    runtime: &Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
    window: &mut Window,
    cx: &mut Context<'_, PopupMenu>,
) -> PopupMenu {
    let mut menu = menu;
    for entry in entries {
        menu = match entry {
            MenuEntryData::Item { .. } => menu.item(item_from(entry, runtime.clone(), entity_id)),
            MenuEntryData::Label(label) => menu.label(label.clone()),
            MenuEntryData::Separator => menu.separator(),
            MenuEntryData::Submenu { label, icon, items } => {
                let submenu_runtime = runtime.clone();
                menu.submenu_with_icon(
                    icon.as_ref().map(|icon| KitIcon::new(map_icon_name(icon))),
                    label.clone(),
                    window,
                    cx,
                    {
                        let items = items.clone();
                        move |submenu, window, cx| {
                            apply_entries(submenu, &items, &submenu_runtime, entity_id, window, cx)
                        }
                    },
                )
            }
        };
    }
    menu
}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let label = props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Menu")
            .to_string();
        let variant = props.get("variant").and_then(|v| v.as_str()).unwrap_or("");
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let scrollable = props
            .get("scrollable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_height = props.get("max_height").and_then(|v| v.as_i64());

        let entries = self.entries;
        let runtime = self.runtime;
        let entity_id = self.entity_id;

        let mut button = GpuiButton::new(SharedString::from(format!("{}-trigger", self.source.id)))
            .label(SharedString::from(label))
            .with_size(Self::size_from(props.get("size").and_then(|v| v.as_str())));
        button = match variant {
            "primary" => button.primary(),
            "danger" => button.danger(),
            "ghost" => button.ghost(),
            _ => button,
        };
        if disabled {
            button = button.disabled(true);
        }

        button.dropdown_menu_with_anchor(
            anchor_from(props.get("anchor").and_then(|v| v.as_str())),
            move |menu, window, cx| {
                let mut menu = apply_entries(menu, &entries, &runtime, entity_id, window, cx);
                if scrollable {
                    menu = menu.scrollable(true);
                }
                if let Some(max_height) = max_height {
                    menu = menu.max_h(px(max_height as f32));
                    if !scrollable {
                        menu = menu.scrollable(true);
                    }
                }
                menu
            },
        )
    }
}
