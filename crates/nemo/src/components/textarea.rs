use gpui::*;
use gpui_component::input::{Input as GpuiInput, InputState};
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct Textarea {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    placeholder: Option<String>,
    #[property]
    default_value: Option<String>,
    #[property]
    rows: Option<i64>,
    #[property]
    auto_grow_min: Option<i64>,
    #[property]
    auto_grow_max: Option<i64>,
    #[property]
    disabled: Option<bool>,
    input_state: Option<Entity<InputState>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Textarea {
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

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.input_state else {
            return div().child("Textarea: missing state").into_any_element();
        };

        let mut input = GpuiInput::new(&state);

        if let Some(true) = self.disabled {
            input = input.disabled(true);
        }

        // Set height directly on the Input element so it doesn't collapse
        // to a single line. GpuiInput multi-line defaults to height: 100%
        // which requires a parent with definite height; using .h() bypasses that.
        let rows = self.rows.or(self.auto_grow_max).unwrap_or(4) as f32;
        let line_height = window.line_height();
        let editor_height = line_height * rows + px(8.);
        input = input.h(editor_height);

        div().w_full().child(input).into_any_element()
    }
}
