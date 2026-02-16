use gpui::*;
use gpui_component::input::{Input as GpuiInput, InputState};
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct CodeEditor {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    language: Option<String>,
    #[property]
    line_number: Option<bool>,
    #[property]
    searchable: Option<bool>,
    #[property]
    default_value: Option<String>,
    #[property]
    multi_line: Option<bool>,
    #[property]
    tab_size: Option<i64>,
    #[property]
    hard_tabs: Option<bool>,
    #[property]
    disabled: Option<bool>,
    #[property]
    rows: Option<i64>,
    input_state: Option<Entity<InputState>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl CodeEditor {
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

impl RenderOnce for CodeEditor {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.input_state else {
            return div().child("CodeEditor: missing state").into_any_element();
        };

        let mut input = GpuiInput::new(&state);

        if let Some(true) = self.disabled {
            input = input.disabled(true);
        }

        // Set height directly on the Input element so it doesn't collapse
        // to a single line. GpuiInput multi-line defaults to height: 100%
        // which requires a parent with definite height; using .h() bypasses that.
        let rows = self.rows.unwrap_or(4) as f32;
        let line_height = window.line_height();
        let editor_height = line_height * rows + px(8.);
        input = input.h(editor_height);

        div().w_full().child(input).into_any_element()
    }
}
