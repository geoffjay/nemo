use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::tree::{Tree as GpuiTree, TreeItem, TreeState};
use nemo_config::Value;
use nemo_layout::BuiltComponent;

/// Converts a `Value` object to a `TreeItem`.
fn value_to_tree_item(value: &Value) -> Option<TreeItem> {
    let obj = value.as_object()?;

    let id = obj.get("id").and_then(|v| v.as_str())?.to_string();
    let label = obj
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let expanded = obj
        .get("expanded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let disabled = obj
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let children = obj
        .get("children")
        .and_then(|v| v.as_array())
        .map(|arr| values_to_tree_items(arr))
        .unwrap_or_default();

    Some(
        TreeItem::new(id, label)
            .expanded(expanded)
            .disabled(disabled)
            .children(children),
    )
}

/// Converts a slice of `Value` items to `TreeItem` list.
pub fn values_to_tree_items(values: &[Value]) -> Vec<TreeItem> {
    values.iter().filter_map(value_to_tree_item).collect()
}

/// Nemo Tree wrapper component.
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Tree {
    source: BuiltComponent,
    tree_state: Option<Entity<TreeState>>,
}

impl Tree {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source: source,
            tree_state: None,
        }
    }

    pub fn tree_state(mut self, state: Entity<TreeState>) -> Self {
        self.tree_state = Some(state);
        self
    }
}

impl RenderOnce for Tree {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.tree_state else {
            return div().child("Tree: missing state").into_any_element();
        };

        let tree = GpuiTree::new(&state, |ix, entry, _selected, _window, _cx| {
            let item = entry.item();
            ListItem::new(ix)
                .pl(px(16. * entry.depth() as f32))
                .child(item.label.clone())
        });

        // Tree internally uses size_full() with uniform_list in Auto sizing mode,
        // so it needs a parent with a definite height to render items.
        let height = self
            .source
            .properties
            .get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(300) as f32;

        div()
            .w_full()
            .h(px(height))
            .child(tree)
            .into_any_element()
    }
}
