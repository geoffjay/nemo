# Task List

A todo application built entirely from Nemo's XML configuration plus a small
Rhai handler script. It demonstrates interactive, handler-driven UI: checking
tasks off, showing optional due dates, and changing a task's category icon by
clicking it and picking one from a popup.

## Running

```bash
cargo run -- --config examples/task-list/app.xml
```

The window opens **maximized** (full screen) using the **nord** dark theme.

## Features

- **Full-screen, nord theme.** The `<window>` omits `width`/`height`, which
  makes Nemo open the window maximized. The theme is `<theme name="nord" mode="dark" />`.
- **Centered, scrolling task panel.** A fixed-width card is centered
  horizontally and scrolls its task list internally while the page itself never
  scrolls.
- **Check tasks off.** Each row has a checkbox. Its `on-change` handler
  (`toggle_task`) persists the checked state and flips the row's status label
  between `Pending` and `✓ Completed`.
- **Optional due dates.** Each row shows a due-date label; tasks without a
  deadline show `No due date`.
- **Clickable category icon with a popup picker.** The emoji on the far left of
  each row is a ghost button. Clicking it opens a shared modal
  (`open_icon_picker`); choosing an emoji copies it back onto that row's icon
  and closes the popup (`choose_icon`).

## How it works

State lives on the components themselves. Handlers call
`set_component_property` / `set_component_text` / `set_component_label`, and the
next render reflects the change — the same pattern used by the `calculator`
example. The shared icon picker remembers which row is being edited by storing
the clicked button's id in the modal's custom `editing` property.

Repeated task rows are defined with `<template>`s (`task_row`, `task_icon`,
`task_check`, `due`, `status`) so each row is a few lines of XML.

## Layout notes / known limitations

Nemo's layout engine intentionally exposes a small surface, and this example
works within it rather than around it:

- **No percentage sizing.** `width`/`height` are pixels only. The card uses a
  fixed width (`820`) and a fixed scroll height (`520`), chosen to approximate
  ~50% width / ~80% height on a typical laptop display. On a much larger monitor
  the card occupies a smaller fraction; adjust the two numbers to taste.
- **No `align`/`justify` attributes.** Centering is done with flex spacers:
  `<stack>` is always `flex-grow:1`, so the two empty stacks on either side of
  the card push it to the horizontal center, while `<panel>` keeps its fixed
  width and does not grow.
- **Vertical position is only approximate.** Because there is no
  vertical-centering primitive (and the only content-height container, `<panel>`,
  always paints a background), the card is placed near the top with a
  `margin-y` gap rather than pixel-perfectly centered. It reads as a tall,
  centered card on common displays.

## Screenshot

<!-- Add screenshot after running: ![Screenshot](screenshot.png) -->
