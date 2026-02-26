use gpui::*;
use gpui_component::checkbox::Checkbox as GpuiCheckbox;
use gpui_component::Disableable;
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

/// A checkbox toggle component.
///
/// # XML Configuration
///
/// ```xml
/// <checkbox id="agree" label="I agree to the terms" checked="false" on-change="handleCheck" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `label` | string | Text label displayed next to the checkbox |
/// | `checked` | bool | Whether the checkbox is checked |
/// | `disabled` | bool | Whether the checkbox is disabled |
/// | `on-change` | string | Event handler invoked when toggled |
#[derive(IntoElement, NemoComponent)]
pub struct Checkbox {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property(default = "")]
    label: String,
    #[property]
    checked: Option<bool>,
    #[property]
    disabled: Option<bool>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Checkbox {
    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let change_handler = self.source.handlers.get("change").cloned();
        let component_id = self.source.id.clone();

        let mut cb = GpuiCheckbox::new(SharedString::from(component_id.clone()))
            .checked(self.checked.unwrap_or(false));

        if !self.label.is_empty() {
            cb = cb.label(self.label);
        }

        if let Some(true) = self.disabled {
            cb = cb.disabled(true);
        }

        if let Some(handler) = change_handler {
            if let (Some(runtime), Some(entity_id)) = (self.runtime, self.entity_id) {
                cb = cb.on_click(move |new_checked, _window, cx| {
                    let data = if *new_checked { "true" } else { "false" };
                    runtime.call_handler(&handler, &component_id, data);
                    cx.notify(entity_id);
                });
            }
        }

        cb
    }
}
