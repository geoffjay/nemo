---
type: Pattern
title: Layout sizing and centering (flexbox-native stacks)
description: How the XML layout engine sizes, grows, aligns, and centers, and how panels own their decoration.
tags: [layout, configuration, gotcha]
timestamp: 2026-07-14T00:00:00Z
---

Nemo's XML layout engine is **flexbox-native**: containers size to their content
by default and grow only when asked. The surface lives in
`apply_layout_styles` (`crates/nemo/src/app.rs`) plus the `stack` and `panel`
components (`crates/nemo/src/components/{stack,panel}.rs`) and the shared helpers
`flex_is_truthy` / `container_grows` (`crates/nemo/src/components/mod.rs`).

## Sizing / growing

* **Content-sized by default.** A `<stack>` no longer grows automatically. It
  grows along its main axis only when it opts in:
  * `flex="1"` (or any positive number, or `flex="true"`) — truthy `flex`.
  * `scroll="true"` — a scroll container must grow to establish the bounded
    flex chain overflow scrolling needs (see
    [definite height for lists](definite-height-for-lists.md)).
  * being the **layout root** — `render_layout` injects `flex=1` on the root so
    the top-level `<layout>` fills the viewport.
* **Panels can grow too.** `<panel flex="1">` fills its parent — needed when a
  panel sits between a growing stack and an inner `scroll` stack (it must be
  bounded so the scroll child is bounded).
* **No percentage sizing.** `width`/`height`/`min-*`/`max-*` are integer pixels.
* `max-width` / `max-height` are supported (clamp a growing/content box).

## Alignment (no more spacer hacks)

* **Cross-axis default:** horizontal stacks center their children
  (`items_center`); vertical stacks stretch (`items_stretch`).
* `align` overrides the cross axis: `start | center | end | stretch`.
* `justify` sets the main axis: `start | center | end | between | around`.

Centering a fixed-width panel no longer needs empty spacer stacks:

```xml
<stack direction="horizontal" flex="1" justify="center">   <!-- fills window -->
  <panel width="820"> ... </panel>   <!-- centered H (justify) + V (default) -->
</stack>
```

`justify="between"` spreads a header's title/hint or a row's left/right groups
to opposite edges. To keep equal-height children in a row (e.g. cards, calculator
buttons), set `align="stretch"` — the horizontal default is now center, not
stretch.

## Panels own their decoration

`Panel::render` is the single source of truth for a panel's `padding`, `border`
(+`border-color`), `rounded`, `shadow`, and background (`theme.secondary`).
`apply_layout_styles` **skips** those decoration props for panels (it still
applies geometry: size, margin, flex) — otherwise the panel would be
double-decorated, drawing a stray outer border/box. Panels support single
`padding` only (no per-side padding).

Worked example: `examples/task-list/` — a maximized, nord-themed card centered
both axes with `justify`/`align` (no spacer stacks), rows using
`justify="between"` + `min-height`/`max-height`, and an internally-scrolling
task list.
