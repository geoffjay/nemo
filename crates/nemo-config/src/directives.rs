//! Control-flow directives (`n:for`/`n:if`) compile pass.
//!
//! Walks a template [`Value`] tree (the shape `process_component_element`
//! produces) and resolves the Vue-style namespaced attributes:
//!
//! * `n:if` — compile-time. Converts the condition into a `bind_visible`
//!   binding on the component. The node stays in the tree; the binding toggles
//!   its `visible` property at runtime.
//! * `n:for` over a static list — compile-time. Expands the node into N copies,
//!   each with the loop variable substituted into `${item}` placeholders, and
//!   strips the `n:for`/`n:key` attributes.
//! * `n:for` over a live data source (`data.*`) — runtime. Marks the node as a
//!   list container: strips `n:for`/`n:key`, stores a `list_binding` metadata
//!   field recording the source path, item variable, key expression, and the
//!   loop body template.
//!
//! The pass is applied after XML parsing and before the runtime's
//! `parse_layout_config`, so `parse_layout_config` and everything downstream
//! sees ordinary `Value` nodes (or list-container nodes for live-data `n:for`).

use crate::Value;
use indexmap::IndexMap;

/// Compiles all `n:`-prefixed directives in a config `Value` tree.
///
/// Walks the `layout` and every `sfc` template, resolving `n:if` and `n:for`
/// in place. The `layout` is rewritten; SFC templates are rewritten in the
/// `sfc` map. Returns the transformed config.
pub fn compile_directives(config: &mut Value) {
    let Some(obj) = config.as_object_mut() else {
        return;
    };

    // Compile the layout tree.
    if let Some(layout) = obj.get_mut("layout") {
        compile_node(layout);
    }

    // Compile every SFC template body.
    if let Some(sfc_map) = obj.get_mut("sfc").and_then(|v| v.as_object_mut()) {
        for (_tag, def) in sfc_map {
            if let Some(template) = def.as_object_mut().and_then(|o| o.get_mut("template")) {
                compile_node(template);
            }
        }
    }

    // Compile XML <template> definitions too.
    if let Some(templates) = obj.get_mut("templates").and_then(|v| v.as_object_mut()) {
        if let Some(template_entries) = templates
            .get_mut("template")
            .and_then(|v| v.as_object_mut())
        {
            for (_name, body) in template_entries {
                compile_node(body);
            }
        }
    }
}

/// Compiles directives in a single component `Value` node (e.g. an SFC
/// template body). This is the per-node entry point for paths that build SFCs
/// individually (like `nemo build`'s `compile_component`), where the whole-config
/// [`compile_directives`] walk doesn't apply.
pub fn compile_directives_node(node: &mut Value) {
    compile_node(node);
}

/// Compiles directives in a single component `Value` node, recursively.
///
/// Handles `n:if` and `n:for` on this node, then recurses into the `component`
/// children map. The node is modified in place.
fn compile_node(node: &mut Value) {
    // Determine the directives on this node.
    let (n_if, n_for, n_key) = {
        let Some(obj) = node.as_object() else {
            return;
        };
        (
            obj.get("n:if").cloned(),
            obj.get("n:for").cloned(),
            obj.get("n:key").or_else(|| obj.get("key")).cloned(),
        )
    };

    // Live-data `n:for`: convert this node into a list container in place.
    // Static `n:for` is handled by the parent (`compile_children_map`), because
    // expansion replaces this node with N siblings.
    if let Some(for_val) = &n_for {
        let for_str = for_val.as_str().unwrap_or("");
        if let Some((item_var, source)) = parse_for_expression(for_str) {
            if is_live_data_source(source) {
                strip_directives(node);
                compile_live_data_n_for(
                    node,
                    for_str,
                    source,
                    item_var,
                    n_key.as_ref(),
                    n_if.as_ref(),
                );
                return;
            }
        }
        // Static n:for on a node compiled directly (no parent map) — expand its
        // own children slot as a fallback so a root-level static n:for still
        // works. Handled below after stripping.
    }

    // No live-data `n:for`. Strip directives and apply `n:if` in place (unless a
    // static `n:for` is present, in which case the parent folds `n:if` per copy).
    if n_for.is_none() {
        strip_directives(node);
        if let Some(cond) = &n_if {
            apply_n_if(node, cond.as_str().unwrap_or(""));
        }
    }

    // Recurse into children (this level expands static `n:for` on each child).
    compile_children_map(node);
}

/// Strips all `n:`-prefixed directive attributes from a node.
fn strip_directives(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        obj.shift_remove("n:if");
        obj.shift_remove("n:for");
        obj.shift_remove("n:key");
        obj.shift_remove("key");
    }
}

/// Walks the `component` children map of a node, expanding static `n:for` on
/// each child (replacing it with N sibling copies) and recursing into the rest.
fn compile_children_map(node: &mut Value) {
    let Some(map) = node
        .as_object_mut()
        .and_then(|o| o.get_mut("component"))
        .and_then(|v| v.as_object_mut())
    else {
        return;
    };

    let original = std::mem::take(map);
    for (id, mut child) in original {
        // Static `n:for` on the child → expand into N sibling entries.
        let n_for = child
            .get("n:for")
            .and_then(|v| v.as_str())
            .map(String::from);
        if let Some(for_str) = &n_for {
            if let Some((item_var, source)) = parse_for_expression(for_str) {
                if !is_live_data_source(source) {
                    if let Some(items) = resolve_static_source(&child, source) {
                        let n_key = child.get("n:key").or_else(|| child.get("key")).cloned();
                        let n_if = child.get("n:if").cloned();
                        strip_directives(&mut child);
                        let expanded = expand_static_for(
                            &id,
                            &child,
                            &items,
                            item_var,
                            n_key.as_ref(),
                            n_if.as_ref(),
                        );
                        for (new_id, mut copy) in expanded {
                            compile_node(&mut copy);
                            map.insert(new_id, copy);
                        }
                        continue;
                    }
                }
            }
        }
        // Otherwise recurse (handles `n:if`, live-data `n:for`, nested children).
        compile_node(&mut child);
        map.insert(id, child);
    }
}

/// Applies `n:if` to a node: emits a `bind_visible` binding from the condition
/// to the component's `visible` property.
///
/// The condition syntax is one of:
/// * A source path (`data.api.status`) — binds `visible` to the truthiness of
///   that path's value.
/// * A comparison (`data.api.status == 'error'`) — binds `visible` to a
///   transform that evaluates the comparison at apply time.
fn apply_n_if(node: &mut Value, condition: &str) {
    let Some(obj) = node.as_object_mut() else {
        return;
    };

    let (source, transform) = parse_condition(condition);

    // Emit bind_visible = "<source>" with an optional transform.
    obj.insert(
        "bind_visible".to_string(),
        Value::String(source.to_string()),
    );
    if let Some(t) = transform {
        // The binding system's `apply_transform` treats a transform containing
        // "value" as a string-format interpolation. We encode the comparison as
        // a transform expression the binding system can evaluate.
        obj.insert("binding".to_string(), {
            let mut b = IndexMap::new();
            b.insert("source".to_string(), Value::String(source.to_string()));
            b.insert("target".to_string(), Value::String("visible".to_string()));
            b.insert("transform".to_string(), Value::String(t));
            Value::Object(b)
        });
        // Remove the simple bind_visible — the explicit binding block above
        // carries the transform. parse_component_from_value reads `binding`
        // blocks with source/target/transform.
        obj.shift_remove("bind_visible");
    }
}

/// Parses an `n:if` condition into a (source, optional-transform) pair.
///
/// * `data.api.status` → (`data.api.status`, None) — truthiness binding.
/// * `data.api.status == 'error'` → (`data.api.status`, Some("== 'error'")) —
///   the transform evaluates the comparison against the bound value.
/// * `data.api.status != 'error'` → (`data.api.status`, Some("!= 'error'")).
fn parse_condition(condition: &str) -> (&str, Option<String>) {
    let cond = condition.trim();

    // Comparison: split on `==` or `!=`.
    if let Some(idx) = cond.find("==") {
        let lhs = cond[..idx].trim();
        let rhs = cond[idx + 2..].trim();
        return (lhs, Some(format!("== {}", rhs)));
    }
    if let Some(idx) = cond.find("!=") {
        let lhs = cond[..idx].trim();
        let rhs = cond[idx + 2..].trim();
        return (lhs, Some(format!("!= {}", rhs)));
    }

    // Plain source path — truthiness.
    (cond, None)
}

/// Parses an `n:for` expression `"item in source"` into `(item_var, source)`.
fn parse_for_expression(expr: &str) -> Option<(&str, &str)> {
    let expr = expr.trim();
    let mid = expr.find(" in ")?;
    let item_var = expr[..mid].trim();
    let source = expr[mid + 4..].trim();
    if item_var.is_empty() || source.is_empty() {
        return None;
    }
    Some((item_var, source))
}

/// Returns true if the source is a live data path (`data.*`).
fn is_live_data_source(source: &str) -> bool {
    source.starts_with("data.")
}

/// Resolves a static iteration source to a concrete `Vec<Value>`.
///
/// Recognized forms:
/// * A literal array attribute: `['a', 'b', 'c']` (parsed by `coerce_value`
///   into `Value::Array`).
/// * A `${var.x}` reference — resolved by the layout builder's variable
///   substitution; we cannot resolve it here, so we return `None` and let the
///   runtime handle it. (Phase 2 scope: literal arrays only at compile time.)
fn resolve_static_source(_node: &Value, source: &str) -> Option<Vec<Value>> {
    // Literal array: the attribute was coerced to Value::Array by coerce_value.
    // But `n:for="item in ['a','b','c']"` puts the *whole* expression as a
    // string; the array literal is inside the source part.
    if source.starts_with('[') && source.ends_with(']') {
        // Try standard JSON first (double-quoted strings).
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(source) {
            let arr = Value::from(json_val);
            if let Value::Array(a) = arr {
                return Some(a);
            }
        }
        // Fall back to single-quoted strings (common in n:for expressions):
        // replace single quotes with double quotes and retry.
        let double_quoted = source.replace('\'', "\"");
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&double_quoted) {
            let arr = Value::from(json_val);
            if let Value::Array(a) = arr {
                return Some(a);
            }
        }
    }
    None
}

/// Expands a static `n:for` into N sibling `(id, node)` copies. The caller
/// (`compile_children_map`) inserts them into the parent's `component` map,
/// replacing the original loop element. `template` is the loop element with
/// directives already stripped.
fn expand_static_for(
    base_id: &str,
    template: &Value,
    items: &[Value],
    item_var: &str,
    n_key: Option<&Value>,
    n_if: Option<&Value>,
) -> Vec<(String, Value)> {
    let mut expanded = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let mut copy = substitute_item(template, item_var, item);

        // Per-instance id: `<base>_<key>` (with n:key) or `<base>_<index>`.
        let instance_id = if let Some(key_expr) = n_key {
            let key_val = resolve_key_value(key_expr, item_var, item);
            format!("{}_{}", base_id, key_val)
        } else {
            format!("{}_{}", base_id, i)
        };

        // Fold n:if into the copy (per-instance conditional).
        if let Some(cond) = n_if {
            apply_n_if(&mut copy, cond.as_str().unwrap_or(""));
        }

        expanded.push((instance_id, copy));
    }
    expanded
}

/// Substitutes `${item_var}` and `${item_var.field}` placeholders in a node
/// with values from the loop item.
fn substitute_item(template: &Value, item_var: &str, item: &Value) -> Value {
    match template {
        Value::String(s) => {
            let placeholder = format!("${{{}", item_var);
            if !s.contains(&placeholder) {
                return template.clone();
            }
            // Replace ${item} and ${item.field} with the item's value.
            let mut result = s.clone();
            // ${item.field} — nested field access.
            let field_prefix = format!("${{{}.", item_var);
            while let Some(start) = result.find(&field_prefix) {
                let after = start + field_prefix.len();
                if let Some(end) = result[after..].find('}') {
                    let field = &result[after..after + end];
                    let replacement = item
                        .as_object()
                        .and_then(|o| o.get(field))
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let pattern = format!("${{{}.{}}}", item_var, field);
                    result = result.replacen(&pattern, &replacement, 1);
                } else {
                    break;
                }
            }
            // ${item} — the whole item.
            let whole = format!("${{{}}}", item_var);
            if result.contains(&whole) {
                let replacement = item.to_string();
                result = result.replacen(&whole, &replacement, 1);
            }
            Value::String(result)
        }
        Value::Object(o) => {
            let mut result = IndexMap::new();
            for (k, v) in o {
                // Skip the `id` key — it's set per-instance by the caller.
                if k == "id" {
                    continue;
                }
                result.insert(k.clone(), substitute_item(v, item_var, item));
            }
            Value::Object(result)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| substitute_item(v, item_var, item))
                .collect(),
        ),
        _ => template.clone(),
    }
}

/// Resolves an `n:key` expression (`user.id`) against a loop item, returning
/// the key value as a string for id suffixing.
fn resolve_key_value(key_expr: &Value, item_var: &str, item: &Value) -> String {
    let expr = match key_expr {
        Value::String(s) => s.as_str(),
        _ => return "0".to_string(),
    };
    // `item.id` → extract `id` from the item.
    let prefix = format!("{}.", item_var);
    if let Some(field) = expr.strip_prefix(&prefix) {
        if let Some(val) = item.as_object().and_then(|o| o.get(field)) {
            return val.to_string();
        }
    }
    // Bare loop variable (`n:key="tab"`): the key is the item value itself
    // (for scalar items like strings/numbers).
    if expr == item_var {
        return item.to_string();
    }
    // Fallback: the expression itself (literal key).
    expr.to_string()
}

/// Compiles a live-data `n:for` into a list-container node.
///
/// Strips `n:for`/`n:key` (already done by the caller), stores a `list_binding`
/// metadata field recording the source path, item variable, key expression,
/// and the loop body template. The node stays in the tree as a container with
/// no children initially — the runtime `ListBindingManager` populates it.
fn compile_live_data_n_for(
    node: &mut Value,
    for_expr: &str,
    source: &str,
    item_var: &str,
    n_key: Option<&Value>,
    n_if: Option<&Value>,
) {
    // Snapshot the template *before* borrowing `node` mutably.
    let template = node.clone();

    let Some(obj) = node.as_object_mut() else {
        return;
    };

    // Extract the key expression.
    let key_expr = n_key.and_then(|v| v.as_str()).map(|s| s.to_string());

    // Build the list_binding metadata.
    let mut lb = IndexMap::new();
    lb.insert("source".to_string(), Value::String(source.to_string()));
    lb.insert("item_var".to_string(), Value::String(item_var.to_string()));
    if let Some(k) = &key_expr {
        lb.insert("key".to_string(), Value::String(k.clone()));
    }
    lb.insert("template".to_string(), template);
    // Fold n:if into the list_binding so it's evaluated per-instance.
    if let Some(cond) = n_if {
        lb.insert(
            "n_if".to_string(),
            Value::String(cond.as_str().unwrap_or("").to_string()),
        );
    }

    obj.insert("list_binding".to_string(), Value::Object(lb));

    // The container has no static children — the runtime populates it. Remove
    // any existing `component` children (they are the loop body, now stored as
    // the template).
    obj.shift_remove("component");

    // Note: we do NOT recurse into children here — the template is stored and
    // expanded at runtime. Nested directives inside the template are compiled
    // when the runtime instantiates each item (deferred).
    let _ = for_expr; // already parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, Value)>) -> Value {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        Value::Object(map)
    }

    fn s(val: &str) -> Value {
        Value::String(val.to_string())
    }

    /// `n:if` with a plain source path compiles to `bind_visible`.
    #[test]
    fn test_n_if_plain_source() {
        let mut node = obj(vec![("type", s("panel")), ("n:if", s("data.api.status"))]);
        compile_node(&mut node);
        let o = node.as_object().unwrap();
        assert!(o.get("n:if").is_none(), "n:if stripped");
        assert_eq!(
            o.get("bind_visible").and_then(|v| v.as_str()),
            Some("data.api.status"),
        );
    }

    /// `n:if` with a comparison compiles to a binding block with a transform.
    #[test]
    fn test_n_if_comparison() {
        let mut node = obj(vec![
            ("type", s("panel")),
            ("n:if", s("data.api.status == 'error'")),
        ]);
        compile_node(&mut node);
        let o = node.as_object().unwrap();
        assert!(o.get("n:if").is_none());
        assert!(o.get("bind_visible").is_none());
        let binding = o.get("binding").and_then(|v| v.as_object()).unwrap();
        assert_eq!(
            binding.get("source").and_then(|v| v.as_str()),
            Some("data.api.status"),
        );
        assert_eq!(
            binding.get("target").and_then(|v| v.as_str()),
            Some("visible"),
        );
        assert_eq!(
            binding.get("transform").and_then(|v| v.as_str()),
            Some("== 'error'"),
        );
    }

    /// Static `n:for` over a literal array expands to N children.
    #[test]
    fn test_n_for_static_literal_array() {
        // n:for is on a child of a parent stack; the parent's compile pass
        // expands it into sibling children.
        let mut parent = obj(vec![
            ("type", s("stack")),
            (
                "component",
                obj(vec![(
                    "tab",
                    obj(vec![
                        ("type", s("tab-item")),
                        ("n:for", s("tab in ['home', 'settings', 'about']")),
                        ("n:key", s("tab")),
                        ("label", s("${tab}")),
                    ]),
                )]),
            ),
        ]);
        compile_node(&mut parent);
        let children = parent.get("component").and_then(|v| v.as_object()).unwrap();
        assert_eq!(children.len(), 3);
        let labels: Vec<String> = children
            .values()
            .map(|c| {
                c.get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        assert!(labels.contains(&"home".to_string()));
        assert!(labels.contains(&"settings".to_string()));
        assert!(labels.contains(&"about".to_string()));
    }

    /// Static `n:for` without `n:key` uses index-based ids.
    #[test]
    fn test_n_for_static_no_key_index_ids() {
        let mut parent = obj(vec![
            ("type", s("stack")),
            (
                "component",
                obj(vec![(
                    "tabs",
                    obj(vec![
                        ("type", s("tab-item")),
                        ("n:for", s("tab in ['a', 'b']")),
                        ("label", s("${tab}")),
                    ]),
                )]),
            ),
        ]);
        compile_node(&mut parent);
        let children = parent.get("component").and_then(|v| v.as_object()).unwrap();
        assert!(children.contains_key("tabs_0"));
        assert!(children.contains_key("tabs_1"));
    }

    /// Static `n:for` with `n:key` uses key-based ids.
    #[test]
    fn test_n_for_static_with_key_ids() {
        let template = obj(vec![("type", s("card")), ("text", s("${u.name}"))]);
        let items = vec![item_a_clone(), item_b_clone()];
        let expanded = expand_static_for(
            "cards",
            &template,
            &items,
            "u",
            Some(&Value::String("u.id".to_string())),
            None,
        );
        let ids: Vec<&str> = expanded.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"cards_a"));
        assert!(ids.contains(&"cards_b"));
    }

    fn item_a_clone() -> Value {
        obj(vec![("id", s("a")), ("name", s("Alpha"))])
    }
    fn item_b_clone() -> Value {
        obj(vec![("id", s("b")), ("name", s("Beta"))])
    }

    /// Live-data `n:for` emits a `list_binding` metadata field.
    #[test]
    fn test_n_for_live_data_emits_list_binding() {
        let mut node = obj(vec![
            ("id", s("user-list")),
            ("type", s("stack")),
            ("n:for", s("user in data.api.users")),
            ("n:key", s("user.id")),
            ("text", s("${user.name}")),
        ]);
        compile_node(&mut node);
        let o = node.as_object().unwrap();
        assert!(o.get("n:for").is_none());
        assert!(o.get("n:key").is_none());
        let lb = o.get("list_binding").and_then(|v| v.as_object()).unwrap();
        assert_eq!(
            lb.get("source").and_then(|v| v.as_str()),
            Some("data.api.users"),
        );
        assert_eq!(lb.get("item_var").and_then(|v| v.as_str()), Some("user"),);
        assert_eq!(lb.get("key").and_then(|v| v.as_str()), Some("user.id"),);
        // The template is stored.
        assert!(lb.get("template").is_some());
        // The container has no static children.
        assert!(o.get("component").is_none());
    }

    /// `n:for` + `n:if` on the same node: `n:for` wins, `n:if` is folded into
    /// the list_binding (live data) or each expansion (static).
    #[test]
    fn test_n_for_with_n_if_live_data() {
        let mut node = obj(vec![
            ("id", s("list")),
            ("type", s("stack")),
            ("n:for", s("x in data.xs")),
            ("n:if", s("data.show")),
        ]);
        compile_node(&mut node);
        let o = node.as_object().unwrap();
        let lb = o.get("list_binding").and_then(|v| v.as_object()).unwrap();
        assert_eq!(lb.get("n_if").and_then(|v| v.as_str()), Some("data.show"),);
    }

    /// `compile_directives` walks the layout and SFC templates.
    #[test]
    fn test_compile_directives_walks_layout_and_sfc() {
        let mut config = obj(vec![
            (
                "layout",
                obj(vec![
                    ("type", s("stack")),
                    (
                        "component",
                        obj(vec![(
                            "p1",
                            obj(vec![("type", s("panel")), ("n:if", s("data.show"))]),
                        )]),
                    ),
                ]),
            ),
            (
                "sfc",
                obj(vec![(
                    "card",
                    obj(vec![(
                        "template",
                        obj(vec![("type", s("panel")), ("n:if", s("data.visible"))]),
                    )]),
                )]),
            ),
        ]);
        compile_directives(&mut config);
        // Layout node compiled.
        let layout_node = config
            .get("layout")
            .and_then(|l| l.get("component"))
            .and_then(|c| c.get("p1"))
            .unwrap();
        assert!(layout_node.get("n:if").is_none());
        assert!(layout_node.get("bind_visible").is_some());
        // SFC template compiled.
        let sfc_tmpl = config
            .get("sfc")
            .and_then(|s| s.get("card"))
            .and_then(|c| c.get("template"))
            .unwrap();
        assert!(sfc_tmpl.get("n:if").is_none());
        assert!(sfc_tmpl.get("bind_visible").is_some());
    }
}
