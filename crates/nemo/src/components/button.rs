use gpui::*;
use gpui_component::button::{Button as GpuiButton, ButtonVariants};
use gpui_component::Disableable;
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

#[derive(IntoElement, NemoComponent)]
pub struct Button {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property(default = "Button")]
    label: String,
    #[property(default = "secondary")]
    variant: String,
    #[property]
    disabled: Option<bool>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl Button {
    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let click_handler = self.source.handlers.get("click").cloned();
        let component_id = self.source.id.clone();

        let mut btn = GpuiButton::new(SharedString::from(component_id.clone()))
            .label(SharedString::from(self.label));

        btn = match self.variant.as_str() {
            "primary" => btn.primary(),
            "danger" => btn.danger(),
            "ghost" => btn.ghost(),
            "warning" => btn.warning(),
            "success" => btn.success(),
            "info" => btn.info(),
            _ => btn,
        };

        if let Some(true) = self.disabled {
            btn = btn.disabled(true);
        }

        if let Some(handler) = click_handler {
            if let (Some(runtime), Some(entity_id)) = (self.runtime, self.entity_id) {
                btn = btn.on_click(move |_event, _window, cx| {
                    runtime.call_handler(&handler, &component_id, "click");
                    cx.notify(entity_id);
                });
            }
        }

        // When sizing properties are present, make button fill its wrapper container.
        // GpuiButton sets a fixed height internally (e.g. h_8), so we need to
        // override it via the Styled trait so the button grows with its container.
        let props = &self.source.properties;
        let has_sizing = props.contains_key("width")
            || props.contains_key("height")
            || props.contains_key("min_width")
            || props.contains_key("min_height")
            || props.contains_key("flex");
        if has_sizing {
            btn = btn.w_full().h_full();
        }

        btn
    }
}
