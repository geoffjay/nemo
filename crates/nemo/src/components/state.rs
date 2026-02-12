use gpui::Entity;
use gpui_component::input::InputState;
use gpui_component::slider::SliderState;
use gpui_component::table::TableState;
use gpui_component::tree::TreeState;
use nemo_config::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::table::NemoTableDelegate;

pub enum ComponentState {
    Input(Entity<InputState>),
    Slider(Entity<SliderState>),
    Table {
        state: Entity<TableState<NemoTableDelegate>>,
        last_data: Vec<Value>,
    },
    Tree {
        state: Entity<TreeState>,
        last_items: Vec<Value>,
    },
    /// Shared open indices for accordion items.
    Accordion(Arc<Mutex<HashSet<usize>>>),
    /// Shared boolean state (collapsible, switch, toggle).
    BoolState(Arc<Mutex<bool>>),
    /// Shared selected value string (select).
    SelectedValue(Arc<Mutex<String>>),
    /// Shared selected index (radio).
    SelectedIndex(Arc<Mutex<Option<usize>>>),
}

pub type ComponentStates = HashMap<String, ComponentState>;
