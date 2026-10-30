# Task List

A todo application backed entirely by a JSON file, built from Nemo's XML
configuration plus a small Rhai handler script. The list **starts empty** on
first run, tasks are **added through a modal**, and **every change is written
straight back to `tasks.json`** and reloaded on the next launch.

## Running

```bash
cargo run -- --app-config examples/task-list/app.nemo
```

Run it from the repository root: the handler script persists to
`examples/task-list/tasks.json`, a path resolved relative to the working
directory (the file is git-ignored). If the file doesn't exist yet it is created
as an empty list on startup.

The window opens **maximized** (full screen) using the **nord** dark theme.

## Features

- **Empty on first run, JSON-backed.** There is no default task list baked into
  the XML. On startup the `on_load` hook reads `tasks.json` (creating an empty
  one if missing) and renders it. Uses the
  [rhai-fs](https://crates.io/crates/rhai-fs) package for file I/O, enabled via
  `<script src="./scripts" features="file-io" on-load="on_load" />`.
- **Add tasks via a modal.** The **+ Add Task** button opens a modal with a
  description field and an optional due date. Submitting (the **Add Task** button
  or pressing Enter in either field) appends the task, saves to disk, and closes
  the modal.
- **Data-driven table.** Tasks render in a `<table>` whose rows are supplied as a
  data array by the script — the list grows and shrinks at runtime without any
  fixed row markup. Columns: `#`, `✓` (done), `Task`, `Due`, `Status`.
- **Check off / delete by row number.** Table cells are text-only (the table
  widget can't host live per-row controls), so editing is keyed by the row
  number shown in the `#` column: type a number, then **Toggle Done** or
  **Delete**.
- **Live due-date countdown.** Each due date is formatted from its stored ISO
  date using [rhai-chrono](https://crates.io/crates/rhai-chrono) — `Due in 3d`,
  `Overdue (2d)`, `Due today`, or `No due date`.
- **REST API on the same data.** An `http-server` data source
  (`<data><source name="rest" type="http-server" port="8420"
  on-request="on_request" /></data>`) serves the task list over HTTP while the
  app runs. Every inbound request is handed to the `on_request` handler
  (main thread), which reads/writes the same `tasks.json` and re-renders the
  table, so REST mutations appear in the UI immediately:

  | Method | Path | Body | Result |
  |---|---|---|---|
  | `GET` | `/tasks` | — | `{"tasks": [...]}` |
  | `POST` | `/tasks` | `{"label": "…", "due": "…"}` | `201` created task |
  | `GET` | `/tasks/<n>` | — | that task (`n` = the 1-based `#` column) |
  | `PUT` | `/tasks/<n>` | `{"label"?, "due"?, "done"?}` | updated task |
  | `DELETE` | `/tasks/<n>` | — | `{"removed": "<label>"}` |

  ```sh
  curl http://127.0.0.1:8420/tasks
  curl -X POST -d '{"label":"From curl","due":"2026-12-01"}' http://127.0.0.1:8420/tasks
  curl -X PUT -d '{"done":true}' http://127.0.0.1:8420/tasks/1
  curl -X DELETE http://127.0.0.1:8420/tasks/1
  ```

## How it works

State is persisted to `tasks.json` via the `json_parse` / `json_stringify`
helpers (backed by `serde_json`) and the `rhai-fs` filesystem package. Each
handler is self-contained: it loads the list from disk, edits it, saves it, and
re-renders the table (Rhai script functions are pure, so there is no shared
in-memory state — the file is the single source of truth).

Two framework capabilities make the example possible, both introduced with it:

- **The `on-load` hook** (`<script on-load="on_load" />`) runs a handler once,
  after the layout is built, so the table reflects `tasks.json` on the first
  paint rather than being hydrated lazily on the first interaction.
- **Input value readback** keeps each `<input>`'s typed text in its `value`
  property, so `get_component_property(id, "value")` returns live text a handler
  can read, and a script can clear a field by setting `value` back to `""`.

The REST endpoints ride two more framework pieces (see
[Data flow](../../docs/knowledgebase/concepts/data-flow.md)):
the built-in `http-server` **data source** owns the socket and turns each
request into a repository update, and a runtime **bridge** invokes the script's
`on_request` function on the main thread for each request, sending the returned
`{status, body, content_type?}` map back to the HTTP client. Scripts stay pure
Rhai — no plugin, and no server code beyond the handler function.

## Layout notes / known limitations

- **The table is text-only.** The table widget renders plain-text cells, so
  "done" is shown as a ✓ glyph and toggled via the row-number controls rather
  than an in-row checkbox. This is the trade-off for an unbounded, data-driven
  row list (see the row-rendering discussion in the knowledge base).
- **No percentage sizing.** `width`/`height` are pixels only. The card uses a
  fixed width (`860`) and the table a fixed height (`440`).
- **No `align`/`justify` centering primitive beyond flex.** `center_row` grows
  (`flex="1"`) and `justify="center"` centers the card horizontally; horizontal
  stacks center their children vertically by default.

## Screenshot

<!-- Add screenshot after running: ![Screenshot](screenshot.png) -->
