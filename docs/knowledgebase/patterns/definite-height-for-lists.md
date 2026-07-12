---
type: Pattern
title: Definite height for uniform_list widgets
description: Table and Tree collapse to 0px without a parent of definite height.
tags: [components, layout, gotcha]
timestamp: 2026-07-11T00:00:00Z
---

Nemo's `Table` and `Tree` components wrap gpui-component widgets that render
their bodies with `uniform_list` using `ListSizingBehavior::Auto` + `.size_full()`.
Such a list **needs a parent with a definite height** — inside an
auto/flex-sized parent its body computes to 0px and no rows render.

# Symptom

The header row still shows (it has a fixed height and `flex_shrink_0`), but the
data rows are invisible. This is misleading: the widget looks "present but
empty" rather than obviously broken.

# Fix

Wrap the widget in a container with a definite height:

```rust
div().w_full().h(px(height)) /* … Table/Tree … */
```

Nemo does this with a default of **300px**, configurable via the `height`
property on the component. When adding or embedding a list-like widget, give it a
definite height rather than relying on flex sizing. See `crates/nemo/src/components/table.rs`
and `tree.rs`.
