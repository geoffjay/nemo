use gpui::*;
use gpui_component::input::{Input as GpuiInput, InputState};
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

/// A single-line text input component.
///
/// # XML Configuration
///
/// ```xml
/// <input id="email" placeholder="Enter your email" value="" on-change="handleEmail" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `placeholder` | string | Placeholder text shown when empty |
/// | `value` | string | Current input value |
/// | `disabled` | bool | Whether the input is disabled |
/// | `on-change` | string | Event handler invoked when value changes |
#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct Input {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    placeholder: Option<String>,
    #[property]
    value: Option<String>,
    #[property]
    disabled: Option<bool>,
    input_state: Option<Entity<InputState>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Input {
    pub fn input_state(mut self, state: Entity<InputState>) -> Self {
        self.input_state = Some(state);
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

impl RenderOnce for Input {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.input_state else {
            return div().child("Input: missing state").into_any_element();
        };

        let mut input = GpuiInput::new(&state);

        if let Some(true) = self.disabled {
            input = input.disabled(true);
        }

        input.into_any_element()
    }
}
