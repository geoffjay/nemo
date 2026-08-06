//! Runtime list-binding manager for live-data `n:for`.
//!
//! Watches data source paths, diffs arrays, and creates/removes component
//! instances by expanding the loop template per item. Reuses
//! `LayoutManager::insert_component`/`remove_component` (from
//! runtime-component-creation) — it does not reimplement insertion.
//!
//! Keying: when `n:key` is present, items are matched by key value (stable
//! identity — state persists across reorders). Without a key, items are
//! matched by index (destroy/recreate on reorder).

use crate::binding::ComponentProperty;
use crate::manager::LayoutManager;
use crate::node::ListBindingSpec;
use nemo_config::Value;
use std::collections::HashMap;

/// A registered list binding: the container component ID and the spec.
#[derive(Debug, Clone)]
struct ActiveListBinding {
    /// The container component ID (the `n:for` element itself).
    container_id: String,
    /// The list binding spec from the compiled layout.
    spec: ListBindingSpec,
    /// Current instance IDs in order (for diffing).
    instance_ids: Vec<String>,
}

/// Manages live-data list bindings at runtime.
///
/// Owned by `LayoutManager`. On `on_data_changed`, for each list binding whose
/// source matches, diffs the new array against the current instances and
/// creates/removes/updates component instances.
pub struct ListBindingManager {
    /// Active list bindings keyed by source path.
    bindings: HashMap<String, Vec<ActiveListBinding>>,
}

impl ListBindingManager {
    /// Creates a new empty list binding manager.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Registers a list binding for a container component.
    ///
    /// Called by `LayoutManager` when applying a layout node that carries a
    /// `ListBindingSpec`. The container is built with no children initially;
    /// the first `on_data_changed` populates it.
    pub fn register(&mut self, container_id: &str, spec: ListBindingSpec) {
        let source = spec.source.clone();
        let entry = ActiveListBinding {
            container_id: container_id.to_string(),
            spec,
            instance_ids: Vec::new(),
        };
        self.bindings.entry(source).or_default().push(entry);
    }

    /// Removes all list bindings for a container (e.g. on layout clear).
    pub fn unregister_container(&mut self, container_id: &str) {
        for entries in self.bindings.values_mut() {
            entries.retain(|e| e.container_id != container_id);
        }
        self.bindings.retain(|_, v| !v.is_empty());
    }

    /// Processes a data change for list bindings matching `source_path`.
    ///
    /// Diffs the new array against current instances and creates/removes
    /// components via `LayoutManager`. Returns `true` if any structural
    /// changes were made (indicating a re-render is needed).
    pub fn on_data_changed(
        &mut self,
        source_path: &str,
        new_value: &Value,
        manager: &mut LayoutManager,
    ) -> bool {
        let mut any_changes = false;
        // Match list bindings whose source equals `source_path` (value is the
        // array directly) or whose source is nested under `source_path` (a
        // source-level update like `data.api` carrying `{users: [...]}` — we
        // extract the sub-array). This makes live-data `n:for` work with HTTP
        // sources that publish the whole response at `data.<source_id>`.
        for (binding_source, entries) in self.bindings.iter_mut() {
            let sub_value = if binding_source == source_path {
                Some(new_value.clone())
            } else if let Some(suffix) = binding_source
                .strip_prefix(source_path)
                .and_then(|s| s.strip_prefix('.'))
            {
                extract_subpath(new_value, suffix)
            } else {
                None
            };
            if let Some(value) = sub_value {
                for entry in entries.iter_mut() {
                    if diff_and_apply(entry, &value, manager) {
                        any_changes = true;
                    }
                }
            }
        }
        any_changes
    }

    /// Returns the source paths watched by list bindings.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        self.bindings.keys().map(|s| s.as_str())
    }

    /// Returns true if any list bindings are registered.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl Default for ListBindingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigates a dotted sub-path within a `Value`, returning the value at that
/// path (cloned). Used to extract a nested array from a source-level update
/// (e.g. suffix `users` in `{users: [...]}`).
fn extract_subpath(value: &Value, suffix: &str) -> Option<Value> {
    let mut current = value;
    for segment in suffix.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = current.as_object()?.get(segment)?;
    }
    Some(current.clone())
}

/// Diffs a new array value against the current instances and applies
/// insert/remove/update/reorder operations to the `LayoutManager`.
///
/// * **Keyed** (`n:key` present): each instance's id is `<container>_<key>`,
///   stable across reorders. A reordered item keeps its component instance
///   (and widget state); the container's child order is updated to match.
/// * **Unkeyed**: index-based ids `<container>_<index>`. Reorders update
///   properties in place at each index.
fn diff_and_apply(
    entry: &mut ActiveListBinding,
    new_value: &Value,
    manager: &mut LayoutManager,
) -> bool {
    let container_id = entry.container_id.clone();
    let spec = entry.spec.clone();

    // Extract the new array. Non-array values are treated as empty.
    let new_items: Vec<Value> = match new_value {
        Value::Array(arr) => arr.clone(),
        _ => Vec::new(),
    };

    // Compute the target instance id for each item.
    let new_ids: Vec<String> = new_items
        .iter()
        .enumerate()
        .map(|(i, item)| instance_id_for(&container_id, &spec, i, item))
        .collect();

    let old_ids = entry.instance_ids.clone();

    // Remove old instances whose id is no longer present.
    for old_id in &old_ids {
        if !new_ids.contains(old_id) {
            let _ = manager.remove_component(old_id);
        }
    }

    // Insert new instances / update reused ones.
    for (i, item) in new_items.iter().enumerate() {
        let id = &new_ids[i];
        if old_ids.contains(id) {
            // Reused instance — update properties + rebind for the new index.
            update_instance(
                manager,
                id,
                &spec.source,
                i,
                &spec.item_var,
                item,
                &spec.template,
            );
        } else {
            let _ = insert_instance(manager, &container_id, id, &spec, i, item);
        }
    }

    // Reorder the container's children to match the new array order (keyed
    // reorders move instances without recreating them).
    manager.set_children_order(&container_id, &new_ids);

    let changed = new_ids != entry.instance_ids;
    entry.instance_ids = new_ids;
    changed
}

/// Computes the instance id for an item: `<container>_<key>` when keyed
/// (stable identity), else `<container>_<index>`.
fn instance_id_for(
    container_id: &str,
    spec: &ListBindingSpec,
    index: usize,
    item: &Value,
) -> String {
    match &spec.key {
        Some(key_expr) => {
            if let Some(key_val) = resolve_key(key_expr, &spec.item_var, item) {
                format!("{}_{}", container_id, key_val)
            } else {
                format!("{}_{}", container_id, index)
            }
        }
        None => format!("{}_{}", container_id, index),
    }
}

/// Inserts a new component instance for `data.source[index]`, expanding the
/// loop template with the item's data.
fn insert_instance(
    manager: &mut LayoutManager,
    container_id: &str,
    instance_id: &str,
    spec: &ListBindingSpec,
    index: usize,
    item: &Value,
) -> Result<(), crate::error::LayoutError> {
    let source = &spec.source;
    let item_var = &spec.item_var;
    let template = &spec.template;
    // Expand the template: substitute `${item_var}` placeholders with per-index
    // binding source paths. The instance's component type comes from the
    // template's `type` field.
    let expanded = expand_template_for_index(template, source, index, item_var, item);

    let component_type = expanded
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("panel")
        .to_string();

    // Extract properties (excluding structural keys).
    let mut properties: HashMap<String, Value> = HashMap::new();
    if let Some(obj) = expanded.as_object() {
        for (key, val) in obj {
            match key.as_str() {
                "type" | "id" | "component" | "binding" | "slot" | "vars" | "list_binding" => {
                    continue;
                }
                _ => {
                    properties.insert(key.clone(), val.clone());
                }
            }
        }
    }

    // Extract handlers.
    let mut handlers: HashMap<String, String> = HashMap::new();
    if let Some(obj) = expanded.as_object() {
        for (key, val) in obj {
            if let Some(event) = key.strip_prefix("on_") {
                if let Some(h) = val.as_str() {
                    handlers.insert(event.to_string(), h.to_string());
                }
            }
        }
    }

    manager.insert_component(
        instance_id,
        &component_type,
        Some(container_id),
        properties,
        handlers,
    )?;

    // Set up per-instance bindings: for each `${item_var.field}` placeholder
    // in the template, bind `data.source[index].field` to the child property.
    setup_instance_bindings(manager, instance_id, source, index, item_var, template);

    // Recursively insert children from the expanded template's `component` map.
    if let Some(children) = expanded.get("component").and_then(|v| v.as_object()) {
        for (child_id, child_val) in children {
            let full_child_id = format!("{}_{}", instance_id, child_id);
            let child_type = child_val
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("panel")
                .to_string();
            let mut child_props: HashMap<String, Value> = HashMap::new();
            if let Some(child_obj) = child_val.as_object() {
                for (k, v) in child_obj {
                    match k.as_str() {
                        "type" | "id" | "component" | "binding" | "slot" | "vars"
                        | "list_binding" => continue,
                        _ => {
                            child_props.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            let _ = manager.insert_component(
                &full_child_id,
                &child_type,
                Some(instance_id),
                child_props,
                HashMap::new(),
            );
        }
    }

    Ok(())
}

/// Expands a loop template for a specific index, substituting `${item_var}`
/// and `${item_var.field}` placeholders with per-index binding source paths
/// or literal values.
fn expand_template_for_index(
    template: &Value,
    source: &str,
    index: usize,
    item_var: &str,
    item: &Value,
) -> Value {
    match template {
        Value::String(s) => {
            let placeholder = format!("${{{}", item_var);
            if !s.contains(&placeholder) {
                return template.clone();
            }
            let mut result = s.clone();
            // ${item.field} → bind to data.source[index].field
            let field_prefix = format!("${{{}.", item_var);
            while let Some(start) = result.find(&field_prefix) {
                let after = start + field_prefix.len();
                if let Some(end) = result[after..].find('}') {
                    let field = &result[after..after + end];
                    // Use the literal value from the item if available; otherwise
                    // leave a binding expression for runtime resolution.
                    let replacement = item
                        .as_object()
                        .and_then(|o| o.get(field))
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| format!("${{{}.{}.{}", source, index, field));
                    let pattern = format!("${{{}.{}}}", item_var, field);
                    result = result.replacen(&pattern, &replacement, 1);
                } else {
                    break;
                }
            }
            // ${item} → the whole item (stringified).
            let whole = format!("${{{}}}", item_var);
            if result.contains(&whole) {
                let replacement = item.to_string();
                result = result.replacen(&whole, &replacement, 1);
            }
            Value::String(result)
        }
        Value::Object(o) => {
            let mut result = indexmap::IndexMap::new();
            for (k, v) in o {
                if k == "id" {
                    continue;
                }
                result.insert(
                    k.clone(),
                    expand_template_for_index(v, source, index, item_var, item),
                );
            }
            Value::Object(result)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| expand_template_for_index(v, source, index, item_var, item))
                .collect(),
        ),
        _ => template.clone(),
    }
}

/// Sets up per-instance bindings for a newly inserted instance. For each
/// `${item_var.field}` placeholder in the template, binds
/// `data.source[index].field` to the corresponding child property.
fn setup_instance_bindings(
    manager: &mut LayoutManager,
    instance_id: &str,
    source: &str,
    index: usize,
    item_var: &str,
    template: &Value,
) {
    let Some(obj) = template.as_object() else {
        return;
    };
    for (key, val) in obj {
        match key.as_str() {
            "type" | "id" | "component" | "binding" | "slot" | "vars" | "list_binding" => continue,
            _ => {}
        }
        // Check if this property value contains a ${item_var.field} placeholder.
        if let Value::String(s) = val {
            let field_prefix = format!("${{{}.", item_var);
            if let Some(start) = s.find(&field_prefix) {
                let after = start + field_prefix.len();
                if let Some(end) = s[after..].find('}') {
                    let field = &s[after..after + end];
                    let binding_source = format!("{}.{}.{}", source, index, field);
                    // Bind the property to the per-index data path.
                    manager.bindings_mut().bind(
                        binding_source,
                        ComponentProperty::new(instance_id, key),
                        crate::node::BindingMode::OneWay,
                        None,
                    );
                }
            }
        }
    }
}

/// Updates an existing instance when the array changes: re-substitutes the
/// template with the new item data, updates the instance's properties (and its
/// children's) in place, and refreshes per-index bindings. The instance keeps
/// its component ID so widget state (input focus/caret) is preserved.
fn update_instance(
    manager: &mut LayoutManager,
    instance_id: &str,
    source: &str,
    index: usize,
    item_var: &str,
    item: &Value,
    template: &Value,
) {
    let expanded = expand_template_for_index(template, source, index, item_var, item);

    // Update the instance's own properties.
    let mut properties: HashMap<String, Value> = HashMap::new();
    if let Some(obj) = expanded.as_object() {
        for (key, val) in obj {
            match key.as_str() {
                "type" | "id" | "component" | "binding" | "slot" | "vars" | "list_binding" => {
                    continue;
                }
                _ => {
                    properties.insert(key.clone(), val.clone());
                }
            }
        }
    }
    let _ = manager.set_properties(instance_id, properties);

    // Update children's properties too.
    if let Some(children) = expanded.get("component").and_then(|v| v.as_object()) {
        for (child_id, child_val) in children {
            let full_child_id = format!("{}_{}", instance_id, child_id);
            let mut child_props: HashMap<String, Value> = HashMap::new();
            if let Some(child_obj) = child_val.as_object() {
                for (k, v) in child_obj {
                    match k.as_str() {
                        "type" | "id" | "component" | "binding" | "slot" | "vars"
                        | "list_binding" => continue,
                        _ => {
                            child_props.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            let _ = manager.set_properties(&full_child_id, child_props);
        }
    }

    // Refresh per-index bindings for the new index.
    manager.bindings_mut().unbind_component(instance_id);
    setup_instance_bindings(manager, instance_id, source, index, item_var, template);
}

/// Resolves a key expression (`user.id`) against a loop item, returning the
/// key value as a string.
fn resolve_key(key_expr: &str, item_var: &str, item: &Value) -> Option<String> {
    let prefix = format!("{}.", item_var);
    let field = key_expr.strip_prefix(&prefix)?;
    item.as_object()
        .and_then(|o| o.get(field))
        .map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::LayoutManager;
    use nemo_config::Value;
    use nemo_registry::{register_all_builtins, ComponentRegistry};
    use std::sync::Arc;

    fn make_manager() -> LayoutManager {
        let registry = Arc::new(ComponentRegistry::new());
        register_all_builtins(&registry);
        LayoutManager::new(registry)
    }

    fn user_obj(name: &str) -> Value {
        let mut o = indexmap::IndexMap::new();
        o.insert("name".to_string(), Value::String(name.to_string()));
        Value::Object(o)
    }
    fn make_spec(source: &str, key: Option<&str>) -> ListBindingSpec {
        let mut o = indexmap::IndexMap::new();
        o.insert("type".to_string(), Value::String("label".to_string()));
        o.insert(
            "text".to_string(),
            Value::String("${user.name}".to_string()),
        );
        let template = Value::Object(o);
        ListBindingSpec {
            source: source.to_string(),
            item_var: "user".to_string(),
            key: key.map(|s| s.to_string()),
            template,
            n_if: None,
        }
    }

    #[test]
    fn test_list_binding_grow_from_empty() {
        let mut manager = make_manager();
        // Create a container (stack) to hold the list.
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));
        let users = Value::Array(vec![user_obj("Alice"), user_obj("Bob")]);

        let changed = lbm.on_data_changed("data.users", &users, &mut manager);
        assert!(changed, "should report changes");
        // Two instances inserted as children of "list".
        let list = manager.get_component("list").unwrap();
        assert_eq!(list.children.len(), 2, "two children inserted");
        assert!(manager.get_component("list_0").is_some());
        assert!(manager.get_component("list_1").is_some());
    }

    #[test]
    fn test_list_binding_shrink() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        // Grow to 3.
        let users3 = Value::Array(vec![user_obj("A"), user_obj("B"), user_obj("C")]);
        lbm.on_data_changed("data.users", &users3, &mut manager);
        assert_eq!(manager.get_component("list").unwrap().children.len(), 3);

        // Shrink to 1.
        let users1 = Value::Array(vec![user_obj("A")]);
        lbm.on_data_changed("data.users", &users1, &mut manager);
        let list = manager.get_component("list").unwrap();
        assert_eq!(list.children.len(), 1, "shrunk to 1 child");
        assert!(manager.get_component("list_0").is_some());
        assert!(manager.get_component("list_1").is_none(), "removed");
        assert!(manager.get_component("list_2").is_none(), "removed");
    }

    #[test]
    fn test_list_binding_empty_array() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        let empty = Value::Array(vec![]);
        lbm.on_data_changed("data.users", &empty, &mut manager);
        let list = manager.get_component("list").unwrap();
        assert_eq!(list.children.len(), 0, "empty array → zero children");
    }

    #[test]
    fn test_list_binding_no_match_source() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        // Non-matching source path → no changes.
        let changed = lbm.on_data_changed("data.other", &Value::Array(vec![]), &mut manager);
        assert!(!changed);
    }

    #[test]
    fn test_list_binding_non_array_value() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        // Non-array value treated as empty.
        lbm.on_data_changed(
            "data.users",
            &Value::String("not an array".to_string()),
            &mut manager,
        );
        let list = manager.get_component("list").unwrap();
        assert_eq!(list.children.len(), 0);
    }

    /// Source-level update (`data.api`) carrying `{users: [...]}` populates a
    /// binding whose source is the nested `data.api.users` — the HTTP-source
    /// case where the whole response is published at the source root.
    #[test]
    fn test_list_binding_prefix_match_extracts_subarray() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.api.users", None));

        // The source-id loop publishes the whole `data.api` object.
        let api = Value::Object({
            let mut o = indexmap::IndexMap::new();
            o.insert(
                "users".to_string(),
                Value::Array(vec![user_obj("Alice"), user_obj("Bob")]),
            );
            o.insert("status".to_string(), Value::String("ok".to_string()));
            o
        });
        let changed = lbm.on_data_changed("data.api", &api, &mut manager);
        assert!(changed, "prefix match extracts users array");
        assert_eq!(manager.get_component("list").unwrap().children.len(), 2);
    }

    /// Item property values are substituted from the item data into the
    /// inserted component (`${user.name}` → the item's name).
    #[test]
    fn test_list_binding_substitutes_item_values() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        let users = Value::Array(vec![user_obj("Alice")]);
        lbm.on_data_changed("data.users", &users, &mut manager);

        let instance = manager.get_component("list_0").unwrap();
        assert_eq!(
            instance.properties.get("text").and_then(|v| v.as_str()),
            Some("Alice"),
            "item value substituted into the instance property"
        );
    }

    /// Reordering the array updates instance property values in place (index
    /// matching). The container's child IDs are stable across the update.
    #[test]
    fn test_list_binding_reorder_updates_values() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", None));

        let users = Value::Array(vec![user_obj("Alice"), user_obj("Bob")]);
        lbm.on_data_changed("data.users", &users, &mut manager);
        assert_eq!(
            manager
                .get_component("list_0")
                .unwrap()
                .properties
                .get("text")
                .and_then(|v| v.as_str()),
            Some("Alice")
        );

        // Reorder: Bob first, Alice second.
        let reordered = Value::Array(vec![user_obj("Bob"), user_obj("Alice")]);
        lbm.on_data_changed("data.users", &reordered, &mut manager);
        // Same child IDs (state preserved), updated values.
        assert_eq!(
            manager
                .get_component("list_0")
                .unwrap()
                .properties
                .get("text")
                .and_then(|v| v.as_str()),
            Some("Bob"),
            "index-0 instance now shows Bob"
        );
        assert_eq!(manager.get_component("list").unwrap().children.len(), 2);
    }

    fn keyed_user(id: &str, name: &str) -> Value {
        let mut o = indexmap::IndexMap::new();
        o.insert("id".to_string(), Value::String(id.to_string()));
        o.insert("name".to_string(), Value::String(name.to_string()));
        Value::Object(o)
    }

    /// With `n:key`, a reordered item keeps its component instance (stable
    /// key-based id), so widget state keyed by component id is preserved.
    #[test]
    fn test_list_binding_keyed_reorder_preserves_identity() {
        let mut manager = make_manager();
        manager
            .insert_component("list", "stack", None, HashMap::new(), HashMap::new())
            .unwrap();

        let mut lbm = ListBindingManager::new();
        lbm.register("list", make_spec("data.users", Some("user.id")));

        // Two keyed users → ids list_a and list_b.
        let users = Value::Array(vec![keyed_user("a", "Alice"), keyed_user("b", "Bob")]);
        lbm.on_data_changed("data.users", &users, &mut manager);
        assert!(manager.get_component("list_a").is_some());
        assert!(manager.get_component("list_b").is_some());
        assert_eq!(
            manager.get_component("list").unwrap().children,
            vec!["list_a".to_string(), "list_b".to_string()]
        );

        // Reorder: Bob first. Both instances persist (same ids); only the
        // container's child order changes.
        let reordered = Value::Array(vec![keyed_user("b", "Bob"), keyed_user("a", "Alice")]);
        lbm.on_data_changed("data.users", &reordered, &mut manager);
        assert!(
            manager.get_component("list_a").is_some(),
            "Alice instance persists"
        );
        assert!(
            manager.get_component("list_b").is_some(),
            "Bob instance persists"
        );
        assert_eq!(
            manager.get_component("list").unwrap().children,
            vec!["list_b".to_string(), "list_a".to_string()],
            "children reordered to match data, instances preserved"
        );
        // Bob's label still shows Bob (its own data), proving identity followed
        // the key, not the index. The template is a label, so `list_b` is it.
        assert_eq!(
            manager
                .get_component("list_b")
                .unwrap()
                .properties
                .get("text")
                .and_then(|v| v.as_str()),
            Some("Bob"),
        );
    }
}
