use gpui::Entity;
use gpui_kit::component::calendar::CalendarState;
use gpui_kit::component::carousel::CarouselState;
use gpui_kit::component::command::CommandState;
use gpui_kit::component::diff::DiffState;
use gpui_kit::component::dock::DockArea as KitDockArea;
use gpui_kit::component::input::{EditorState, InputState, TextareaState};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::component::questionnaire::QuestionnaireState;
use gpui_kit::component::slider::SliderState;
use gpui_kit::component::table::TableState;
use gpui_kit::component::tree::TreeState;
use nemo_config::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::dock::DeclarativePanel;
use super::settings::SettingValues;
use super::table::NemoTableDelegate;

/// Per-component runtime state.
///
/// # Thread Safety
///
/// Several variants wrap their inner value in `Arc<Mutex<T>>`. This provides
/// shared ownership between the component's render closure (which runs on
/// the main/UI thread) and callback closures (which also run on the main
/// thread in GPUI's single-threaded model).
///
/// ## Why `Arc<Mutex<T>>` in a single-threaded model?
///
/// GPUI's rendering model is single-threaded, so these mutexes are never
/// contended across threads. The `Arc` is needed because render closures
/// and event callbacks each capture their own clone of the `Arc`, and the
/// `Mutex` provides interior mutability behind the shared reference. A
/// `RefCell` would also work but `Mutex` was chosen for `Send` compatibility
/// with future async extensions.
///
/// ## Lock Ordering
///
/// There is no cross-`ComponentState` lock ordering concern because each
/// component's mutex is independent — no code path locks two component
/// state mutexes simultaneously. The only lock ordering rule:
///
/// 1. **Never hold a `ComponentState` mutex while accessing
///    `ComponentStates` (the outer `HashMap`)**. In practice this is not
///    an issue because `ComponentStates` is accessed via `&mut self`
///    (exclusive borrow) during render, while the `Arc<Mutex<T>>` clones
///    are used in closures that run later.
///
/// ## Deadlock Prevention
///
/// Lock scopes are kept minimal (lock, read/write, drop). No `MutexGuard`
/// is held across `.await` points or render calls. This prevents deadlocks
/// in both the current single-threaded model and any future multi-threaded
/// extensions.
///
/// ## `Entity<T>` Variants
///
/// The `Input`, `Slider`, `Table`, and `Tree` variants use GPUI's
/// `Entity<T>` (a reference-counted handle to framework-managed state).
/// These are not wrapped in `Mutex` because GPUI entities have their own
/// internal synchronization.
pub enum ComponentState {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
    Editor(Entity<EditorState>),
    Slider(Entity<SliderState>),
    Carousel {
        state: Entity<CarouselState>,
        last_count: usize,
        last_index: i64,
    },
    /// Active date selection for a calendar, plus the last `value` the
    /// render pass wrote into it (to detect script-driven re-seeds).
    Calendar {
        state: Entity<CalendarState>,
        last_value: String,
    },
    /// Live interaction state for a command palette (query text, focus,
    /// highlighted row), the last `query` the render pass wrote into it,
    /// and the echo flag that suppresses the reflexive `on_query` a
    /// script-driven `set_query` would fire.
    Command {
        state: Entity<CommandState>,
        last_query: String,
        echo: Rc<RefCell<Option<String>>>,
    },
    Table {
        state: Entity<TableState<NemoTableDelegate>>,
        last_data: Vec<Value>,
    },
    /// A data table's virtualized state: the kit entity (columns baked into
    /// its header layout, selection and sort state machine inside), the last
    /// `data` snapshot installed (updates replace the delegate's rows in
    /// place), and the last column/behavior schema (a change rebuilds the
    /// state — its header layout is created at construction).
    DataTable {
        state: Entity<TableState<crate::components::data_table::DataTableDelegate>>,
        last_data: Vec<Value>,
        last_schema: String,
    },
    /// A diff viewer's readonly comparison state: the kit entity (files,
    /// hunks, and the presentation model baked at construction), the last
    /// `diff` patch text installed (updates replace the files in place), and
    /// the last configuration schema (mode/context/expansion/inline — a
    /// change rebuilds the state).
    Diff {
        state: Entity<DiffState>,
        last_data: Option<String>,
        last_schema: String,
    },
    Tree {
        state: Entity<TreeState>,
        last_items: Vec<Value>,
    },
    /// Live virtual-list bookkeeping for a message scroller, plus the last
    /// row count the render pass wrote into it (to detect XML-driven row
    /// changes and re-reset the list).
    MessageScroller {
        state: Entity<MessageScrollerState>,
        last_count: usize,
    },
    /// A questionnaire flow: the shared state entity (None while the declared
    /// schema is invalid; its definitions keep the freeform input entities
    /// alive), the schema identity last built (a change re-creates the state —
    /// the upstream schema is fixed at construction), and the last active item
    /// pushed or reported (guards the property-driven navigation from fighting
    /// user clicks).
    Questionnaire {
        state: Option<Entity<QuestionnaireState>>,
        last_items: String,
        last_current: String,
    },
    /// Field values for a `<settings>` widget: the entity keeps each
    /// declarative field's current value across renders, and its updates
    /// drive the re-render (gets and sets route through the wrapper's
    /// closures). `last_schema` mirrors the field collection so XML edits
    /// re-seed the store from the declared `value` attributes.
    Settings {
        values: Option<Entity<SettingValues>>,
        last_schema: String,
    },
    /// A `<dock>` layout: the live area entity (skinned by the kit's
    /// `DockSkin`), the declarative panel entities keyed by their
    /// `<dock-panel>` id (bodies are re-deposited into them every render
    /// pass), and the layout signature last installed on the area — a
    /// change re-runs `set_center`/`set_dock`; otherwise user drags,
    /// resizes, toggles and tab picks survive untouched.
    Dock {
        area: Entity<KitDockArea>,
        panels: HashMap<String, Entity<DeclarativePanel>>,
        last_layout: String,
    },
    /// Shared open indices for accordion items.
    Accordion(Arc<Mutex<HashSet<usize>>>),
    /// Shared boolean state (collapsible, switch, toggle).
    BoolState(Arc<Mutex<bool>>),
    /// Shared selected value string (select).
    SelectedValue(Arc<Mutex<String>>),
    /// Shared selected index (radio).
    SelectedIndex(Arc<Mutex<Option<usize>>>),
}

/// Registry mapping component IDs to their runtime state.
///
/// Accessed exclusively via `&mut self` during the render pass. The
/// `get_or_create_*` methods return `Arc<Mutex<T>>` clones that outlive
/// the borrow — these clones are captured by event-handler closures and
/// used later (still on the main thread) to mutate state.
pub struct ComponentStates(HashMap<String, ComponentState>);

impl ComponentStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, id: &str) -> Option<&ComponentState> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ComponentState> {
        self.0.get_mut(id)
    }

    pub fn insert(&mut self, id: String, state: ComponentState) {
        self.0.insert(id, state);
    }

    /// Drops the entry for `id` (used when a questionnaire's declared schema
    /// changed and its state must be rebuilt from scratch).
    pub fn remove(&mut self, id: &str) -> Option<ComponentState> {
        self.0.remove(id)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets or creates shared boolean state (collapsible, switch, toggle).
    pub fn get_or_create_bool_state(&mut self, id: &str, initial: bool) -> Arc<Mutex<bool>> {
        if let Some(ComponentState::BoolState(state)) = self.0.get(id) {
            return Arc::clone(state);
        }
        let state = Arc::new(Mutex::new(initial));
        self.0.insert(
            id.to_string(),
            ComponentState::BoolState(Arc::clone(&state)),
        );
        state
    }

    /// Gets or creates shared selected value state for a select component.
    pub fn get_or_create_selected_value(
        &mut self,
        id: &str,
        initial: String,
    ) -> Arc<Mutex<String>> {
        if let Some(ComponentState::SelectedValue(state)) = self.0.get(id) {
            return Arc::clone(state);
        }
        let state = Arc::new(Mutex::new(initial));
        self.0.insert(
            id.to_string(),
            ComponentState::SelectedValue(Arc::clone(&state)),
        );
        state
    }

    /// Gets or creates shared selected index state for a radio component.
    pub fn get_or_create_selected_index(
        &mut self,
        id: &str,
        initial: Option<usize>,
    ) -> Arc<Mutex<Option<usize>>> {
        if let Some(ComponentState::SelectedIndex(state)) = self.0.get(id) {
            return Arc::clone(state);
        }
        let state = Arc::new(Mutex::new(initial));
        self.0.insert(
            id.to_string(),
            ComponentState::SelectedIndex(Arc::clone(&state)),
        );
        state
    }

    /// Gets or creates shared accordion open-indices state.
    ///
    /// `initial_open` is the set of item indices that should start expanded,
    /// computed by the caller from the `open` attribute of each
    /// `<accordion-item>` child.
    pub fn get_or_create_accordion_state(
        &mut self,
        id: &str,
        initial_open: HashSet<usize>,
    ) -> Arc<Mutex<HashSet<usize>>> {
        if let Some(ComponentState::Accordion(state)) = self.0.get(id) {
            return Arc::clone(state);
        }

        let state = Arc::new(Mutex::new(initial_open));
        self.0.insert(
            id.to_string(),
            ComponentState::Accordion(Arc::clone(&state)),
        );
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_states_new_is_empty() {
        let states: ComponentStates = ComponentStates::new();
        assert!(states.is_empty());
    }

    // ── BoolState ─────────────────────────────────────────────────────

    #[test]
    fn test_bool_state_insert_and_retrieve() {
        let mut states = ComponentStates::new();
        let val = Arc::new(Mutex::new(false));
        states.insert(
            "switch1".into(),
            ComponentState::BoolState(Arc::clone(&val)),
        );

        if let Some(ComponentState::BoolState(s)) = states.get("switch1") {
            assert!(!*s.lock().unwrap());
        } else {
            panic!("Expected BoolState");
        }
    }

    #[test]
    fn test_bool_state_shared_mutation() {
        let val = Arc::new(Mutex::new(false));
        let clone = Arc::clone(&val);

        // Simulate toggling from one reference
        *clone.lock().unwrap() = true;

        // Original should see the change
        assert!(*val.lock().unwrap());
    }

    #[test]
    fn test_bool_state_returns_existing() {
        let mut states = ComponentStates::new();
        let original = Arc::new(Mutex::new(true));
        states.insert(
            "tog".into(),
            ComponentState::BoolState(Arc::clone(&original)),
        );

        // Simulating get_or_create pattern: should return the existing one
        if let Some(ComponentState::BoolState(existing)) = states.get("tog") {
            let retrieved = Arc::clone(existing);
            assert!(Arc::ptr_eq(&retrieved, &original));
        } else {
            panic!("Expected BoolState");
        }
    }

    // ── SelectedValue ─────────────────────────────────────────────────

    #[test]
    fn test_selected_value_insert_and_retrieve() {
        let mut states = ComponentStates::new();
        let val = Arc::new(Mutex::new("option_a".to_string()));
        states.insert(
            "select1".into(),
            ComponentState::SelectedValue(Arc::clone(&val)),
        );

        if let Some(ComponentState::SelectedValue(s)) = states.get("select1") {
            assert_eq!(*s.lock().unwrap(), "option_a");
        } else {
            panic!("Expected SelectedValue");
        }
    }

    #[test]
    fn test_selected_value_mutation() {
        let val = Arc::new(Mutex::new("a".to_string()));
        let clone = Arc::clone(&val);
        *clone.lock().unwrap() = "b".to_string();
        assert_eq!(*val.lock().unwrap(), "b");
    }

    // ── SelectedIndex ─────────────────────────────────────────────────

    #[test]
    fn test_selected_index_none_initial() {
        let mut states = ComponentStates::new();
        let val = Arc::new(Mutex::new(None::<usize>));
        states.insert(
            "radio1".into(),
            ComponentState::SelectedIndex(Arc::clone(&val)),
        );

        if let Some(ComponentState::SelectedIndex(s)) = states.get("radio1") {
            assert_eq!(*s.lock().unwrap(), None);
        } else {
            panic!("Expected SelectedIndex");
        }
    }

    #[test]
    fn test_selected_index_with_value() {
        let val: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(Some(2)));
        assert_eq!(*val.lock().unwrap(), Some(2));

        *val.lock().unwrap() = Some(0);
        assert_eq!(*val.lock().unwrap(), Some(0));
    }

    // ── Accordion ─────────────────────────────────────────────────────

    #[test]
    fn test_accordion_empty_initial() {
        let val = Arc::new(Mutex::new(HashSet::<usize>::new()));
        assert!(val.lock().unwrap().is_empty());
    }

    #[test]
    fn test_accordion_with_initial_open() {
        let mut initial = HashSet::new();
        initial.insert(0);
        initial.insert(2);
        let val = Arc::new(Mutex::new(initial));

        let locked = val.lock().unwrap();
        assert!(locked.contains(&0));
        assert!(!locked.contains(&1));
        assert!(locked.contains(&2));
    }

    #[test]
    fn test_accordion_toggle() {
        let val = Arc::new(Mutex::new(HashSet::new()));
        let clone = Arc::clone(&val);

        // Open index 1
        clone.lock().unwrap().insert(1);
        assert!(val.lock().unwrap().contains(&1));

        // Close index 1
        clone.lock().unwrap().remove(&1);
        assert!(!val.lock().unwrap().contains(&1));
    }

    #[test]
    fn test_accordion_init_from_items() {
        // Simulate the logic from App::get_or_create_accordion_state
        let items = [
            Value::Object({
                let mut m = indexmap::IndexMap::new();
                m.insert("title".to_string(), Value::String("First".into()));
                m.insert("open".to_string(), Value::Bool(true));
                m
            }),
            Value::Object({
                let mut m = indexmap::IndexMap::new();
                m.insert("title".to_string(), Value::String("Second".into()));
                m
            }),
            Value::Object({
                let mut m = indexmap::IndexMap::new();
                m.insert("title".to_string(), Value::String("Third".into()));
                m.insert("open".to_string(), Value::Bool(true));
                m
            }),
            Value::Object({
                let mut m = indexmap::IndexMap::new();
                m.insert("title".to_string(), Value::String("Fourth".into()));
                m.insert("open".to_string(), Value::Bool(false));
                m
            }),
        ];

        let mut initial = HashSet::new();
        for (ix, item_val) in items.iter().enumerate() {
            if let Some(obj) = item_val.as_object() {
                if obj.get("open").and_then(|v| v.as_bool()).unwrap_or(false) {
                    initial.insert(ix);
                }
            }
        }

        assert_eq!(initial.len(), 2);
        assert!(initial.contains(&0)); // First: open=true
        assert!(!initial.contains(&1)); // Second: no open key
        assert!(initial.contains(&2)); // Third: open=true
        assert!(!initial.contains(&3)); // Fourth: open=false
    }

    // ── Type discrimination ───────────────────────────────────────────

    #[test]
    fn test_different_state_types_coexist() {
        let mut states = ComponentStates::new();
        states.insert(
            "sw".into(),
            ComponentState::BoolState(Arc::new(Mutex::new(true))),
        );
        states.insert(
            "sel".into(),
            ComponentState::SelectedValue(Arc::new(Mutex::new("x".into()))),
        );
        states.insert(
            "rad".into(),
            ComponentState::SelectedIndex(Arc::new(Mutex::new(Some(1)))),
        );
        states.insert(
            "acc".into(),
            ComponentState::Accordion(Arc::new(Mutex::new(HashSet::new()))),
        );

        assert_eq!(states.len(), 4);
        assert!(matches!(
            states.get("sw"),
            Some(ComponentState::BoolState(_))
        ));
        assert!(matches!(
            states.get("sel"),
            Some(ComponentState::SelectedValue(_))
        ));
        assert!(matches!(
            states.get("rad"),
            Some(ComponentState::SelectedIndex(_))
        ));
        assert!(matches!(
            states.get("acc"),
            Some(ComponentState::Accordion(_))
        ));
    }

    #[test]
    fn test_overwrite_state_with_same_id() {
        let mut states = ComponentStates::new();
        states.insert(
            "x".into(),
            ComponentState::BoolState(Arc::new(Mutex::new(false))),
        );
        states.insert(
            "x".into(),
            ComponentState::BoolState(Arc::new(Mutex::new(true))),
        );

        if let Some(ComponentState::BoolState(s)) = states.get("x") {
            assert!(*s.lock().unwrap()); // Should be the new value
        } else {
            panic!("Expected BoolState");
        }
    }
}
