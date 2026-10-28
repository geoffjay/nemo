use gpui::*;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{Tree as GpuiTree, TreeEntry, TreeItem, TreeState};
use gpui_kit::component::{h_flex, Icon as KitIcon};
use nemo_config::Value;
use nemo_layout::BuiltComponent;
use std::sync::Arc;

use crate::components::icon::map_icon_name;

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

/// A declared tree node extracted from `<tree-node>` children by the
/// dispatcher.
///
/// Node content is plain data (icons + text), not pre-rendered elements:
/// the kit tree renders each visible row through `uniform_list`, which
/// invokes the row renderer once per layout pass and once per paint pass —
/// elements are consumed per invocation, so they must be built fresh from
/// data inside the row closure on every call.
pub struct DeclaredTreeNode {
    /// Stable node id (the `id` attribute, or derived from the label).
    pub id: String,
    /// Node label, used for plain-text rows and accessibility.
    pub label: String,
    /// Optional icons flanking the label: `leading` renders before the
    /// text, `trailing` after it.
    pub leading_icon: Option<String>,
    pub trailing_icon: Option<String>,
    /// Nested nodes.
    pub children: Vec<DeclaredTreeNode>,
    /// Initial expanded state (the `expanded` attribute).
    pub expanded: bool,
    /// Disabled nodes cannot be selected or expanded.
    pub disabled: bool,
}

/// Recursively builds kit `TreeItem`s (structure only) from declared nodes.
pub fn declared_to_tree_items(nodes: &[DeclaredTreeNode]) -> Vec<TreeItem> {
    nodes
        .iter()
        .map(|node| {
            TreeItem::new(node.id.clone(), node.label.clone())
                .expanded(node.expanded)
                .disabled(node.disabled)
                .children(declared_to_tree_items(&node.children))
        })
        .collect()
}

/// Nemo Tree wrapper component.
/// A tree view component for hierarchical data.
///
/// Two authoring modes, picked automatically:
///
/// * **Data mode** — an `items` array (usually via `<binding>`), rendered
///   through gpui-kit's virtualized tree.
/// * **Declared mode** — nodes authored as `<tree-node>` children:
///
/// ```xml
/// <tree id="files" height="300">
///   <tree-node id="src" expanded="true">
///     <icon name="folder" /><text content="src" />
///     <tree-node><icon name="file-code" /><text content="main.rs" /></tree-node>
///   </tree-node>
/// </tree>
/// ```
///
/// A node's `tree-node` children are its nested nodes; every other child
/// is its label content. Text content (`<tree-node>Label</tree-node>`)
/// becomes the label directly.
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `height` | int | Fixed tree height in pixels |
/// | `items` | JSON array | Node objects with `id`, `label`, optional `children` (data mode) |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Tree {
    source: BuiltComponent,
    tree_state: Option<Entity<TreeState>>,
    /// Declared-mode nodes; when set, rendered through the kit tree with
    /// per-node pre-rendered content. Owned (not Arc) because the content
    /// elements are consumed exactly once at paint.
    declared: Option<Vec<DeclaredTreeNode>>,
}

impl Tree {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            tree_state: None,
            declared: None,
        }
    }

    pub fn tree_state(mut self, state: Entity<TreeState>) -> Self {
        self.tree_state = Some(state);
        self
    }

    /// Switch to declared mode with the extracted node structure.
    pub fn declared(mut self, nodes: Vec<DeclaredTreeNode>) -> Self {
        self.declared = Some(nodes);
        self
    }
}

/// One row's label content, built fresh from node data on every call
/// (`uniform_list` renders each visible row once for layout and once for
/// paint; elements are consumed per call, so they can't be shared).
fn render_node_label(node: Option<&DeclaredTreeNode>, fallback: &str) -> gpui::AnyElement {
    let Some(node) = node else {
        // Data-mode item or a node without extracted content.
        return div().child(fallback.to_string()).into_any_element();
    };
    let mut row = h_flex().gap_2().items_center();
    if let Some(name) = &node.leading_icon {
        row = row.child(KitIcon::new(map_icon_name(name)).size_4().flex_shrink_0());
    }
    row = row.child(node.label.clone());
    if let Some(name) = &node.trailing_icon {
        row = row.child(KitIcon::new(map_icon_name(name)).size_4().flex_shrink_0());
    }
    row.into_any_element()
}

/// Renders one visible tree row: indent, disclosure chevron for folders,
/// then the node's label content.
fn render_tree_row(
    ix: usize,
    entry: &TreeEntry,
    nodes: Option<&Arc<Vec<DeclaredTreeNode>>>,
) -> ListItem {
    let item = entry.item();
    let depth = entry.depth();
    let is_folder = entry.is_folder();
    let is_expanded = entry.is_expanded();

    // Chevron reflecting expansion; leaves keep a spacer for alignment.
    let disclosure: gpui::AnyElement = if is_folder {
        let name = if is_expanded {
            map_icon_name("chevron-down")
        } else {
            map_icon_name("chevron-right")
        };
        KitIcon::new(name)
            .size_4()
            .flex_shrink_0()
            .into_any_element()
    } else {
        div().w_4().flex_shrink_0().into_any_element()
    };

    // Declared-mode content is looked up by node id; data-mode rows have no
    // entry and render their plain label.
    let node = nodes.and_then(|n| find_node_by_id(n, item.id.as_ref()));
    let rendered = render_node_label(node, item.label.as_ref());

    ListItem::new(ix).pl(px(4. + 16. * depth as f32)).child(
        h_flex()
            .gap_1()
            .items_center()
            .child(disclosure)
            .child(rendered),
    )
}

/// Finds a declared node by id in a nested structure.
fn find_node_by_id<'a>(nodes: &'a [DeclaredTreeNode], id: &str) -> Option<&'a DeclaredTreeNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node_by_id(&node.children, id) {
            return Some(found);
        }
    }
    None
}

impl RenderOnce for Tree {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.tree_state else {
            return div().child("Tree: missing state").into_any_element();
        };

        let declared: Option<Arc<Vec<DeclaredTreeNode>>> = self.declared.map(Arc::new);

        let tree = GpuiTree::new(&state, move |ix, entry, _selected, _window, _cx| {
            render_tree_row(ix, entry, declared.as_ref())
        });

        // Tree internally uses size_full() with uniform_list in Auto sizing mode,
        // so it needs a parent with a definite height to render items.
        let height = self
            .source
            .properties
            .get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(300) as f32;

        div().w_full().h(px(height)).child(tree).into_any_element()
    }
}
