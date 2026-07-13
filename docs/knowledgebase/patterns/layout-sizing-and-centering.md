---
type: Pattern
title: Layout sizing and centering with a minimal style surface
description: What the XML layout engine does and does not support, and how to size/center within it.
tags: [layout, configuration, gotcha]
timestamp: 2026-07-12T00:00:00Z
---

Nemo's XML layout engine exposes a deliberately small style surface
(`apply_layout_styles` in `crates/nemo/src/app.rs`, plus the `stack` and `panel`
components). Several of its behaviours are non-obvious and easy to fight:

* **`flex` ignores its numeric value.** `apply_layout_styles` only does
  `if flex.is_some() { wrapper.flex_1() }`, so `flex="8"` is identical to
  `flex="1"`. Flex siblings always split their axis **equally** — you cannot
  express an 80/20 ratio with flex weights.
* **No percentage sizing.** `width`/`height` are integer pixels only; a value
  like `"50%"` is stored as an unparsed string and has no effect.
* **No alignment attributes.** There is no `align`/`justify`/`items`/`self`
  attribute; the only hardcoded centering in the codebase is the error view.
* **`<stack>` always grows** — `stack.rs` renders `div().flex().flex_1()`, so a
  stack always fills its axis. **`<panel>` does not grow** (it is content-sized
  on the main axis) and **always paints `theme.secondary`**, ignoring any
  `background` property.
* **Scrolling** is `<stack scroll="true">` only, along the stack's direction,
  and needs a bounded height (see
  [definite height for lists](definite-height-for-lists.md)).

## Centering technique

To center a fixed-width panel horizontally, place it between two empty
`<stack>` spacers: the stacks grow equally and push the panel to the middle,
while the panel keeps its pixel `width`.

```xml
<stack direction="horizontal">
  <stack />                                  <!-- grows -->
  <panel width="820"> ... </panel>           <!-- fixed, centered -->
  <stack />                                  <!-- grows -->
</stack>
```

True **vertical** centering of a tall card is not cleanly achievable: the only
content-sized (non-growing) container is `<panel>`, which always paints a
background, so using one as a vertical-centering wrapper draws a full-width
band. Approximate with `margin-y`, or accept the band. Because the window is
usually maximized (omit `width`/`height` on `<window>`), pick fixed pixel sizes
that approximate the desired fraction and document the choice.

Worked example: `examples/task-list/` (a maximized, nord-themed card centered
horizontally with an internally-scrolling task list).
