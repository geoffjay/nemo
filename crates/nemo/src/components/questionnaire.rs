use gpui::{
    div, AnyElement, App, Entity, IntoElement, ParentElement as _, RenderOnce, Styled as _,
};
use gpui_kit::component::questionnaire::{
    Questionnaire as GpuiQuestionnaire, QuestionnaireActions, QuestionnaireAnswers,
    QuestionnaireChoice, QuestionnaireChoices, QuestionnaireDescription, QuestionnaireError,
    QuestionnaireInput, QuestionnaireItem, QuestionnaireNext, QuestionnairePrevious,
    QuestionnaireProgress, QuestionnaireSkip, QuestionnaireState, QuestionnaireSubmit,
    QuestionnaireTitle,
};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, Size as ComponentSize};
use nemo_layout::BuiltComponent;

/// One declarative answer choice of a `<questionnaire-item>`, collected
/// pre-render like stepper items. `default_selected` seeds the state's initial
/// snapshot; `value` is what the `answers` property reports.
#[derive(Clone)]
pub struct QuestionnaireChoiceData {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub disabled: bool,
    pub default_selected: bool,
}

/// The optional freeform answer of an item. Its `Entity<InputState>` is created
/// when the questionnaire state is built (inputs need a window; see
/// `App::get_or_create_questionnaire_state`), so this only carries the
/// declarative description.
#[derive(Clone)]
pub struct QuestionnaireInputData {
    pub label: String,
    pub placeholder: Option<String>,
    pub disabled: bool,
}

/// One declarative child of a `<questionnaire>`: an ordered question with
/// choice cards and an optional freeform input.
#[derive(Clone)]
pub struct QuestionnaireItemData {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub required: bool,
    pub multiple: bool,
    pub disabled: bool,
    pub choices: Vec<QuestionnaireChoiceData>,
    pub input: Option<QuestionnaireInputData>,
}

/// The questionnaire schema's identity, used by the app to detect XML edits
/// that require rebuilding the state (the upstream schema is fixed at
/// construction; items and choices cannot be swapped on a live entity).
pub fn questionnaire_schema_signature(items: &[QuestionnaireItemData]) -> String {
    items
        .iter()
        .map(|item| {
            let choices = item
                .choices
                .iter()
                .map(|choice| {
                    format!(
                        "{}/{}/{}/{}/{}",
                        choice.value,
                        choice.label,
                        choice.description.clone().unwrap_or_default(),
                        choice.disabled,
                        choice.default_selected
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}/{}/{}/{}/{}/{}/{}/{}",
                item.name,
                item.label,
                item.description.clone().unwrap_or_default(),
                item.required,
                item.multiple,
                item.disabled,
                item.input.as_ref().map_or("-".to_string(), |input| format!(
                    "{}/{}/{}",
                    input.label,
                    input.placeholder.clone().unwrap_or_default(),
                    input.disabled
                ),),
                choices
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Mirrors the kit's `QuestionnaireState::validate_schema` over the collected
/// declarative data: duplicate item names, duplicate choice values within an
/// item, and more than one default on a single-choice item are rejected before
/// the state entity is built (the kit constructor asserts the same invariants).
pub fn validate_questionnaire_schema(items: &[QuestionnaireItemData]) -> Result<(), String> {
    let mut item_names = std::collections::HashSet::new();
    for item in items {
        if !item_names.insert(item.name.clone()) {
            return Err(format!("duplicate questionnaire item `{}`", item.name));
        }
        let mut choices = std::collections::HashSet::new();
        let mut defaults = 0;
        for choice in &item.choices {
            if !choices.insert(choice.value.clone()) {
                return Err(format!(
                    "duplicate choice `{}` in item `{}`",
                    choice.value, item.name
                ));
            }
            defaults += usize::from(choice.default_selected);
        }
        if !item.multiple && defaults > 1 {
            return Err(format!(
                "single-choice item `{}` has more than one default answer",
                item.name
            ));
        }
    }
    Ok(())
}

/// Serializes the answer snapshot for the `answers` property and handler
/// payloads: `item=value1,value2|freeform text`, items joined with `;` in the
/// enabled schema order.
pub fn serialize_questionnaire_answers(answers: &QuestionnaireAnswers) -> String {
    answers
        .iter()
        .map(|(name, answer)| {
            let mut rendered = answer
                .choices()
                .iter()
                .map(|choice| choice.as_ref().to_string())
                .collect::<Vec<_>>()
                .join(",");
            if let Some(freeform) = answer.freeform() {
                if !rendered.is_empty() {
                    rendered.push('|');
                }
                rendered.push_str(freeform.as_ref());
            }
            format!("{}={}", name, rendered)
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// A guided, one-question-at-a-time form (gpui-kit `Questionnaire`). The
/// upstream kit is a compound of twelve part types over one shared
/// `QuestionnaireState` entity; this wrapper renders that recommended
/// composition as one widget: progress line, the active item (title,
/// description, choice cards, optional freeform input, and validation error),
/// and the navigation buttons (Previous/Skip/Next/Submit — only the visible
/// ones enter the tree).
///
/// The schema is declarative: `<questionnaire-item>` children (with
/// `<questionnaire-choice>` and `<questionnaire-input>` sub-children) are
/// collected pre-render into a `QuestionnaireItemData` list the app turns into
/// a `QuestionnaireState`. Titles and choice labels come from the collected
/// definitions — the kit's part fallbacks render them, so the wrapper passes
/// no children into `QuestionnaireChoice`.
///
/// ```xml
/// <questionnaire id="survey" on-submit="handle_survey">
///   <questionnaire-item name="direction" label="What should we prototype next?" required="true">
///     <questionnaire-choice value="delegation" label="Delegation" default-selected="true" />
///     <questionnaire-choice value="questions" label="Question prompts" />
///   </questionnaire-item>
///   <questionnaire-item name="detail" label="How much detail?">
///     <questionnaire-choice value="focused" label="Focused" />
///     <questionnaire-choice value="complete" label="Complete flow" />
///     <questionnaire-input label="Something else" placeholder="Type another answer…" />
///   </questionnaire-item>
/// </questionnaire>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `size` | string | `xsmall`, `small`, `medium` (default), or `large` for the whole composition |
/// | `show-progress` | bool | Render the "Question X of Y" line (default true) |
/// | `show-actions` | bool | Render the navigation buttons (default true) |
/// | `current-item` | string | Script-driven navigation: the name of the now-active item |
/// | `current` | read | Active item name, mirrored back on every change |
/// | `answers` | read | Serialized answer snapshot of all enabled items |
///
/// # Out of scope
///
/// The kit's richer composition seams are Rust-closure or host-owned APIs the
/// declarative system cannot express, and are not silently dropped:
///
/// * Custom part layouts (rendering a subset of items, custom action rows) and
///   the choice seams (`indicator_style`/`content_style`/`shortcut_style`,
///   `render_indicator`, `render_shortcut`) — XML cannot hold render closures.
/// * Validators (`with_validator`) and external errors
///   (`set_external_error`/`clear_external_error`) — closures and host
///   lifecycle; required-item validation stays available.
/// * Host-driven conditional items (`set_item_disabled` for branching) — items
///   with `disabled="true"` are fixed at construction.
#[derive(IntoElement)]
pub struct Questionnaire {
    source: BuiltComponent,
    items: Vec<QuestionnaireItemData>,
    state: Option<Entity<QuestionnaireState>>,
}

impl Questionnaire {
    pub fn new(
        source: BuiltComponent,
        items: Vec<QuestionnaireItemData>,
        state: Option<Entity<QuestionnaireState>>,
    ) -> Self {
        Self {
            source,
            items,
            state,
        }
    }

    fn size_of(value: Option<&str>) -> ComponentSize {
        match value {
            Some("xsmall") => ComponentSize::XSmall,
            Some("small") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        }
    }
}

impl RenderOnce for Questionnaire {
    fn render(self, _window: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let show_progress = props
            .get("show_progress")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let show_actions = props
            .get("show_actions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let size = Self::size_of(props.get("size").and_then(|v| v.as_str()));

        let Some(state) = self.state else {
            // The declared schema failed validation (duplicate names, multiple
            // defaults on a single-choice item, …); the reason is logged once
            // at creation.
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Questionnaire: invalid schema")
                .into_any_element();
        };

        // One item per composition slot, in declaration order. Inactive items
        // render nothing themselves, so it is safe to mount every declared
        // item; the active one is the only one that enters the layout.
        let mut root = GpuiQuestionnaire::new(&state);
        if show_progress {
            root = root.child(QuestionnaireProgress::new(&state));
        }
        for item in &self.items {
            root = root.child(render_item(&state, item));
        }
        if show_actions {
            root = root.child(
                QuestionnaireActions::new(&state)
                    .child(QuestionnairePrevious::new(&state))
                    .child(QuestionnaireSkip::new(&state))
                    .child(QuestionnaireNext::new(&state))
                    .child(QuestionnaireSubmit::new(&state)),
            );
        }

        root.with_size(size).into_any_element()
    }
}

/// The active item's composition: title, description, the choice cards with
/// the optional freeform input inside the choices group, and the validation
/// error. Title/description/label text all come from the definitions, so none
/// of the parts receive renderable children here.
fn render_item(state: &Entity<QuestionnaireState>, item: &QuestionnaireItemData) -> AnyElement {
    let name = item.name.clone();
    let mut item_element = QuestionnaireItem::new(state, name.clone())
        .child(QuestionnaireTitle::new(state, name.clone()));
    if item.description.is_some() {
        item_element = item_element.child(QuestionnaireDescription::new(state, name.clone()));
    }

    let mut choices = QuestionnaireChoices::new(state, name.clone());
    for choice in &item.choices {
        choices = choices.child(QuestionnaireChoice::new(
            state,
            name.clone(),
            choice.value.clone(),
        ));
    }
    if item.input.is_some() {
        choices = choices.child(QuestionnaireInput::new(state, name.clone()));
    }

    item_element
        .child(choices)
        .child(QuestionnaireError::new(state, name))
        .into_any_element()
}

#[cfg(test)]
mod probe {
    use gpui::{AppContext as _, TestAppContext};
    use gpui_kit::component::questionnaire::{
        QuestionnaireChoiceDefinition, QuestionnaireItemDefinition, QuestionnaireState,
    };

    use crate::components::questionnaire::{
        questionnaire_schema_signature, serialize_questionnaire_answers,
        validate_questionnaire_schema, QuestionnaireChoiceData, QuestionnaireItemData,
    };

    fn items() -> Vec<QuestionnaireItemData> {
        vec![
            QuestionnaireItemData {
                name: "direction".to_string(),
                label: "What should we prototype next?".to_string(),
                description: Some("Choose a direction or write your own.".to_string()),
                required: true,
                multiple: false,
                disabled: false,
                choices: vec![
                    QuestionnaireChoiceData {
                        value: "delegation".to_string(),
                        label: "Delegation".to_string(),
                        description: None,
                        disabled: false,
                        default_selected: false,
                    },
                    QuestionnaireChoiceData {
                        value: "questions".to_string(),
                        label: "Question prompts".to_string(),
                        description: None,
                        disabled: false,
                        default_selected: false,
                    },
                ],
                input: None,
            },
            QuestionnaireItemData {
                name: "detail".to_string(),
                label: "How much detail?".to_string(),
                description: None,
                required: false,
                multiple: true,
                disabled: false,
                choices: vec![QuestionnaireChoiceData {
                    value: "focused".to_string(),
                    label: "Focused".to_string(),
                    description: None,
                    disabled: false,
                    default_selected: true,
                }],
                input: None,
            },
        ]
    }

    fn to_definitions(items: &[QuestionnaireItemData]) -> Vec<QuestionnaireItemDefinition> {
        items
            .iter()
            .map(|item| {
                QuestionnaireItemDefinition::new(item.name.clone(), item.label.clone())
                    .with_required(item.required)
                    .with_multiple(item.multiple)
                    .with_disabled(item.disabled)
                    .with_choices(item.choices.iter().map(|choice| {
                        let base = QuestionnaireChoiceDefinition::new(
                            choice.value.clone(),
                            choice.label.clone(),
                        )
                        .with_disabled(choice.disabled);
                        if choice.default_selected {
                            base.with_default_selected(true)
                        } else {
                            base
                        }
                    }))
            })
            .collect()
    }

    #[test]
    fn schema_validation_rejects_duplicates_and_double_defaults() {
        let its = items();
        assert!(validate_questionnaire_schema(&its).is_ok());

        let mut duplicate_item = its.clone();
        duplicate_item.push(its[0].clone());
        let reason = validate_questionnaire_schema(&duplicate_item).unwrap_err();
        assert!(reason.contains("duplicate questionnaire item"), "{reason}");

        let mut multiple_defaults = its.clone();
        multiple_defaults[0].choices[0].default_selected = true;
        multiple_defaults[0].choices.push(QuestionnaireChoiceData {
            value: "both".to_string(),
            label: "Both together".to_string(),
            description: None,
            disabled: false,
            default_selected: true,
        });
        let reason = validate_questionnaire_schema(&multiple_defaults).unwrap_err();
        assert!(reason.contains("more than one default"), "{reason}");
    }

    /// The signature must change whenever anything the definitions encode
    /// changes, or an XML edit would silently keep the stale schema.
    #[test]
    fn schema_signature_tracks_definitions() {
        let before = questionnaire_schema_signature(&items());
        // A stable schema is byte-stable.
        assert_eq!(before, questionnaire_schema_signature(&items()));

        let mut grown = items();
        grown[0].choices.push(QuestionnaireChoiceData {
            value: "both".to_string(),
            label: "Both together".to_string(),
            description: None,
            disabled: false,
            default_selected: false,
        });
        assert_ne!(
            before,
            questionnaire_schema_signature(&grown),
            "adding a choice changes the schema"
        );

        let mut renamed = items();
        renamed[1].label = "Different question".to_string();
        assert_ne!(
            before,
            questionnaire_schema_signature(&renamed),
            "renaming an item label changes the schema"
        );
    }

    #[gpui::test]
    fn answer_serialization_reports_schema_order(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let its = items();
        let state =
            cx.update(|cx| cx.new(|cx| QuestionnaireState::new(to_definitions(&its), cx).unwrap()));
        cx.update(|cx| {
            // The multi item's only choice is `default_selected`, the single
            // item is unanswered: the snapshot reflects both initial states.
            let answers = state.read(cx).answers();
            assert_eq!(
                serialize_questionnaire_answers(&answers),
                "direction=;detail=focused"
            );
        });
    }
}
