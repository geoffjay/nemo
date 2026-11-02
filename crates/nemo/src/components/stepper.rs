use gpui::*;
use gpui_kit::component::stepper::{Stepper as GpuiStepper, StepperItem as GpuiStepperItem};
use gpui_kit::component::{Sizable, Size as ComponentSize};
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;
use crate::runtime::NemoRuntime;
use std::rc::Rc;
use std::sync::Arc;

/// A step-by-step progress indicator (gpui-kit `Stepper`). Steps come from
/// `<stepper-item>` children; the current step is `selected-index` (0-based,
/// default 0). Steps before the selected index render as completed, so
/// `selected-index="2"` of 4 steps shows two passed check icons.
///
/// ```xml
/// <stepper id="wizard" selected-index="1" on-click="handle_step">
///   <stepper-item icon="user" label="Personal Info" />
///   <stepper-item icon="credit-card" label="Payment" />
///   <stepper-item icon="circle-check" label="Confirmation" />
/// </stepper>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `selected-index` | integer | Current step, 0-based |
/// | `layout` | string | `horizontal` (default) or `vertical` |
/// | `size` | string | `xsmall`, `small`, `medium` (default), or `large` |
/// | `text-center` | bool | Center the text within each step item |
/// | `disabled` | bool | Disable every step |
/// | `on-click` | string | Handler invoked with the clicked step index |
#[derive(IntoElement)]
pub struct Stepper {
    source: BuiltComponent,
    items: Vec<StepperItemData>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

/// One `<stepper-item>` child: `label` (or the element's rendered children)
/// is the step text shown next to the icon/number badge.
#[derive(Default)]
pub struct StepperItemData {
    pub label: Option<String>,
    pub icon: Option<String>,
    pub disabled: bool,
    pub children: Vec<AnyElement>,
}

impl Stepper {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            items: Vec::new(),
            runtime: None,
            entity_id: None,
        }
    }

    pub fn items(mut self, items: Vec<StepperItemData>) -> Self {
        self.items = items;
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

/// Type alias for the kit stepper's step-click callback.
type StepClickFn = dyn Fn(&usize, &mut Window, &mut App);

impl RenderOnce for Stepper {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let selected_index = props
            .get("selected_index")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0) as usize;
        let layout = props
            .get("layout")
            .and_then(|v| v.as_str())
            .unwrap_or("horizontal");
        let disabled = props
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let text_center = props
            .get("text_center")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let component_id = self.source.id.clone();
        let handler = self.source.handlers.get("click").cloned();
        let runtime = self.runtime.clone();
        let entity_id = self.entity_id;
        let on_click: Option<Rc<StepClickFn>> = handler.map(|handler| {
            let click_component_id = component_id.clone();
            let runtime = runtime.expect("runtime present when handler is");
            let entity_id = entity_id.expect("entity_id present when handler is");
            let f: Rc<StepClickFn> =
                Rc::new(move |step: &usize, _window: &mut Window, cx: &mut App| {
                    runtime.call_handler(&handler, &click_component_id, &step.to_string());
                    cx.notify(entity_id);
                });
            f
        });

        let mut stepper = GpuiStepper::new(ElementId::Name(SharedString::from(format!(
            "{}-stepper",
            component_id
        ))))
        .selected_index(selected_index)
        .with_size(Stepper::size_from(
            props.get("size").and_then(|v| v.as_str()),
        ));
        if layout == "vertical" {
            stepper = stepper.vertical();
        }
        if text_center {
            stepper = stepper.text_center(true);
        }
        if disabled {
            stepper = stepper.disabled(true);
        }

        let items: Vec<GpuiStepperItem> = self
            .items
            .into_iter()
            .map(|data| {
                let mut item = GpuiStepperItem::new();
                if let Some(icon) = data.icon {
                    item = item.icon(gpui_kit::component::Icon::new(map_icon_name(&icon)));
                }
                if data.disabled {
                    item = item.disabled(true);
                }
                if let Some(label) = data.label {
                    item = item.child(label);
                }
                if !data.children.is_empty() {
                    item = item.children(data.children);
                }
                item
            })
            .collect();

        let mut stepper = stepper.items(items);
        if let Some(on_click) = on_click {
            stepper = stepper.on_click(move |step, window, cx| on_click(step, window, cx));
        }

        stepper.into_any_element()
    }
}
