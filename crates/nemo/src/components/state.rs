use gpui::Entity;
use gpui_component::input::InputState;
use gpui_component::table::TableState;
use gpui_component::tree::TreeState;
use nemo_config::Value;
use std::collections::HashMap;

use super::table::NemoTableDelegate;

pub enum ComponentState {
    Input(Entity<InputState>),
    Table {
        state: Entity<TableState<NemoTableDelegate>>,
        last_data: Vec<Value>,
    },
    Tree {
        state: Entity<TreeState>,
        last_items: Vec<Value>,
    },
}

pub type ComponentStates = HashMap<String, ComponentState>;
