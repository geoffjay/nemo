# Task List

A todo application built entirely from Nemo's XML configuration plus a small
Rhai handler script. It demonstrates interactive, handler-driven UI with
**on-disk persistence**: checking tasks off, changing category icons, and
optional due dates are all saved to a JSON file and restored on the next
launch.

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
- **Persistent state.** Task completion, icons, and due dates are written to
  `tasks.json` (next to the app config) on every change and loaded on startup.
  Uses the [rhai-fs](https://crates.io/crates/rhai-fs) package for file I/O,
  enabled via `<script src="./scripts" features="file-io" />`.
- **Check tasks off.** Each row has a checkbox. Its `on-change` handler
  (`toggle_task`) persists the checked state, flips the row's status label
  between `Pending` and `✓ Completed`, and writes back to disk.
- **Optional due dates with live countdown.** Each row shows a due-date label
  computed from the stored ISO date using
  [rhai-chrono](https://crates.io/crates/rhai-chrono) — `Due in 3d`, `Overdue
  (2d)`, `Due today`, or `No due date`.
- **Clickable category icon with a popup picker.** The emoji on the far left of
  each row is a ghost button. Clicking it opens a shared modal
  (`open_icon_picker`); choosing an emoji copies it back onto that row's icon,
  persists it, and closes the popup (`choose_icon`).

## How it works

State is persisted to `tasks.json` via the `json_parse` / `json_stringify`
helpers (backed by `serde_json`) and the `rhai-fs` filesystem package. The
handler script keeps an in-memory cache of the task list (a rhai array of
maps) that is loaded from disk on first interaction and written back on every
change.

The `file-io` feature is opt-in: `<script src="./scripts" features="file-io" />`
in `app.xml` tells the runtime to register the `rhai-fs` package with the Rhai
engine. Without it, scripts are sandboxed with no host I/O.

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
