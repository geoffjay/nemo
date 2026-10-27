use gpui::*;
use gpui_kit::component::button::{Button as GpuiButton, ButtonVariants};
use gpui_kit::component::input::{Textarea as GpuiTextarea, TextareaState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme, Sizable};
use nemo_macros::NemoComponent;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

/// A rich text editor component.
///
/// # XML Configuration
///
/// ```xml
/// <text-editor id="content" placeholder="Start writing..." rows="10" default-value="" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `placeholder` | string | Placeholder text shown when empty |
/// | `default-value` | string | Initial text content |
/// | `rows` | int | Number of visible rows |
/// | `disabled` | bool | Whether the editor is read-only |
#[derive(IntoElement, NemoComponent)]
#[allow(dead_code)]
pub struct TextEditor {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    placeholder: Option<String>,
    #[property]
    default_value: Option<String>,
    #[property]
    rows: Option<i64>,
    #[property]
    disabled: Option<bool>,
    input_state: Option<Entity<TextareaState>>,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
}

impl TextEditor {
    pub fn input_state(mut self, state: Entity<TextareaState>) -> Self {
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

    fn render_toolbar(id: &str, state: &Entity<TextareaState>) -> impl IntoElement {
        h_flex()
            .gap(px(2.))
            .p(px(4.))
            .child(
                GpuiButton::new(SharedString::from(format!("{}-bold", id)))
                    .label("B")
                    .xsmall()
                    .ghost()
                    .on_click({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| {
                                state.insert("**", window, cx);
                            });
                        }
                    }),
            )
            .child(
                GpuiButton::new(SharedString::from(format!("{}-italic", id)))
                    .label("I")
                    .xsmall()
                    .ghost()
                    .on_click({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| {
                                state.insert("_", window, cx);
                            });
                        }
                    }),
            )
            .child(
                GpuiButton::new(SharedString::from(format!("{}-underline", id)))
                    .label("U")
                    .xsmall()
                    .ghost()
                    .on_click({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| {
                                state.insert("__", window, cx);
                            });
                        }
                    }),
            )
    }
}

impl RenderOnce for TextEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(state) = self.input_state else {
            return div().child("TextEditor: missing state").into_any_element();
        };

        let disabled = matches!(self.disabled, Some(true));

        // Unique per-editor ids — two editors on the same page get identical
        // view paths (the ViewElement uses the type name as its element id),
        // so nested element ids must differ or a11y node ids collide.
        let editor_id = self.source.id.clone();

        let mut input = GpuiTextarea::new(&state);
        if disabled {
            input = input.disabled(true);
        }
        // Remove default border/bg — the outer container provides them
        input = input.appearance(false);

        // The Textarea element uses height: 100% + flex_grow,
        // requiring a parent with definite height. Compute from rows.
        let rows = self.rows.unwrap_or(4) as f32;
        let line_height = window.line_height();
        let editor_height = line_height * rows + px(8.);

        v_flex()
            .w_full()
            .h(editor_height)
            .flex_shrink_0()
            .border_1()
            .border_color(cx.theme().colors.input)
            .rounded(cx.theme().radius)
            .bg(cx.theme().colors.background)
            .overflow_hidden()
            .child(
                div()
                    .border_b_1()
                    .border_color(cx.theme().colors.border)
                    .child(Self::render_toolbar(&editor_id, &state)),
            )
            .child(input)
            .into_any_element()
    }
}
