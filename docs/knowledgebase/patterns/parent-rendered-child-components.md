---
type: Pattern
title: Parent-rendered child components
description: How a parent component accesses and renders its typed child components, vs. generic render_children.
tags: [components, gpui, render]
timestamp: 2026-07-11T00:00:00Z
---

A `BuiltComponent` carries its children as **IDs**, not inline structs
(`crates/nemo-layout/src/manager.rs:39`):

```rust
pub struct BuiltComponent {
    pub id: String,
    pub component_type: String,
    pub properties: HashMap<String, Value>,
    pub handlers: HashMap<String, String>,
    pub children: Vec<String>,      // child component IDs
    pub parent: Option<String>,
}
```

The flat `HashMap<String, BuiltComponent>` (keyed by ID) is threaded through
`render_component()`, so a parent can look up each child by ID. There are two
ways parents consume children.

# 1. Generic child rendering

`render_children()` (`crates/nemo/src/app.rs:392`) renders every child into an
opaque `Vec<AnyElement>` and hands it to the component:

```rust
component.children.iter()
    .filter_map(|child_id| components.get(child_id))
    .map(|child| self.render_component(child, components, entity_id, window, cx))
    .collect()
```

Used by container components that don't care about child *types* — `stack`,
`panel`, `tabs` (panels), `collapsible`, `modal`, `tooltip`, `badge`. The
component receives `Vec<AnyElement>` via a `.children(children)` builder.

# 2. Typed child components (the sidenav precedent)

When a parent needs to interpret its children's **properties** (not just place
their rendered output), it receives the raw child `BuiltComponent`s and renders
them itself. `sidenav-bar` / `sidenav-bar-item` is the reference implementation
(`app.rs:825-863`, `crates/nemo/src/components/sidenav_bar.rs:131`):

```rust
let child_components: Vec<BuiltComponent> = component.children.iter()
    .filter_map(|id| components.get(id))
    .filter(|c| c.component_type == "sidenav_bar_item")
    .cloned()
    .collect();
```

The parent then reads each child's properties (e.g.
`SidenavBarItem::from_built_component`) and builds elements. The child type also
gets a standalone match arm that renders nothing, since the parent owns it:

```rust
"sidenav_bar_item" => div().into_any_element(),  // rendered by its parent
```

Note the kebab→snake conversion: the XML tag `sidenav-bar-item` becomes the
`component_type` `sidenav_bar_item` (`kebab_to_snake`, `xml_parser.rs:875`).

# Element text bodies are dropped for component elements

`process_component_element()` (`xml_parser.rs:660`) copies attributes but skips
`__type__`, `__children__`, and **`__cdata__`**. So the text body of a component
element (`<x>body text</x>`) is currently discarded — a component's content must
come from an attribute or from nested child components, not element text. This
matters when designing child-element APIs (see
[declarative children migration](../plans/declarative-children-migration.md)).

# When to use which

* Use **`render_children`** when children are arbitrary UI to place verbatim.
* Use **typed child components** when the parent must read child properties to
  drive a wrapped `gpui_component` widget (accordion items, tabs, menu items,
  options), following the sidenav pattern.
