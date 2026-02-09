use gpui::Entity;
use gpui_component::input::InputState;
use std::collections::HashMap;

pub enum ComponentState {
    Input(Entity<InputState>),
}

pub type ComponentStates = HashMap<String, ComponentState>;
