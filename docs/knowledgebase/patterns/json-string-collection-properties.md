---
type: Pattern
title: Collection properties as JSON-string attributes
description: Several components take arrays/objects as a single JSON-string XML attribute, coerced by coerce_value.
tags: [components, config, xml]
timestamp: 2026-07-11T00:00:00Z
---

Several built-in components receive their collection data as a **single XML
attribute holding a JSON string** rather than as nested child elements:

```xml
<accordion items='[{"title":"Q1","content":"A1"},{"title":"Q2","content":"A2"}]' />
```

# How the coercion works

Attribute values are converted to `Value` by `coerce_value()`
(`crates/nemo-config/src/xml_parser.rs:880`) during parsing. The relevant rule:

```rust
// Check for JSON arrays in attributes (e.g., columns='[{"key":"a"}]')
if s.starts_with('[') && s.ends_with(']') {
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(s) {
        return Value::from(json_val);
    }
}
```

Key consequences:

* Only values that **start with `[` and end with `]`** are attempted as JSON.
  A bare object attribute (`{...}`) is **not** coerced — it stays a
  `Value::String`.
* If `serde_json` fails to parse, the value silently falls back to a plain
  string. There is no error surfaced for malformed JSON in an attribute.
* Coercion is applied uniformly to every attribute, so any attribute shaped like
  `[...]` becomes an array regardless of the property's intended type.

# Inventory of JSON-string collection properties

Most collection components have **migrated to child elements** (see
[declarative children migration](../plans/declarative-children-migration.md));
these no longer take a JSON-string property: `accordion` (`<accordion-item>`),
`tabs` (`<tab-item>`), `select`/`radio` (`<option>`), `dropdown-button`
(`<menu-item>`), and `list` (`<list-item>`).

The remaining JSON-string collection properties are:

| Component | Property | Shape the code reads |
|---|---|---|
| `table` | `columns` | array of `{key, label, width?}` objects |
| `table` | `data` | array of row objects (usually data-bound) |
| `tree` | `items` | array of `{id, label, expanded?, disabled?, children[]}` (recursive) |
| charts | `data` | array of row objects (via `chart_utils::extract_data_array`) |
| stacked/clustered charts | `x_fields` etc. | array of strings (via `chart_utils::get_string_array`) |

Charts’ `data` is the case where JSON-in-attribute is least awkward, since it is
normally bound to a data source rather than hand-authored. `tree`'s recursive
shape and `table`'s data are the main remaining candidates, but both are out of
scope for the initial migration.

# Extraction idioms (not yet unified)

Two spellings of the same extraction coexist across components:

```rust
// (a)
match props.get("options") { Some(Value::Array(arr)) => …, _ => … }
// (b)
props.get("options").and_then(|v| v.as_array()).map(…)
```

`crates/nemo/src/components/chart_utils.rs` provides reusable helpers
(`get_string_array`, `get_string_field`, `get_f64_field`, `extract_data_array`),
but they are `pub(crate)` and chart-scoped; non-chart components re-implement the
string-array extraction inline.

# Direction

Moving these collections from JSON-string attributes to nested child elements is
tracked in
[declarative children migration](../plans/declarative-children-migration.md),
which builds on
[parent-rendered child components](parent-rendered-child-components.md).
