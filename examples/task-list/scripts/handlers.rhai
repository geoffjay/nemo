// Task List event handlers with on-disk persistence.
//
// The task list lives entirely in tasks.json — there is NO default list baked
// into the XML. On startup the `on_load` hook (wired via `<script
// on-load="on_load" />`) reads the file (creating an empty one if missing) and
// renders it into the table before the first paint. Every mutation writes the
// whole list straight back to disk.
//
// IMPORTANT: Rhai script functions are PURE. They cannot read or mutate
// script-level (module) `let`/`const` state, and a function called from another
// function gets a fresh scope. So this script keeps NO mutable module state:
// each handler loads the list from disk, edits it, saves it, and re-renders.
//
// Shared constants are therefore exposed as zero-arg functions, not `const`.
//
// File layout (tasks.json):
//   { "tasks": [ {"label":"…","done":false,"due":"2026-07-15"}, … ] }

// Path to the persistence file, resolved relative to the working directory.
fn data_file() { "examples/task-list/tasks.json" }

// ── Persistence ─────────────────────────────────────────────────────────────

fn load_tasks() {
    // First run (or after the file was deleted): create an empty list on disk
    // so the app always has a file to read, and start empty.
    if !path(data_file()).exists {
        log_info("No tasks.json found; creating an empty task list");
        save_tasks([]);
        return [];
    }

    let file = open_file(data_file(), "r");
    let contents = file.read_string();
    let parsed = json_parse(contents);

    if parsed == () || parsed.tasks == () {
        log_warn("tasks.json is not valid; starting empty");
        return [];
    }

    parsed.tasks
}

fn save_tasks(tasks) {
    let json = json_stringify(#{ tasks: tasks });
    let file = open_file(data_file(), "w");
    // "w" opens without truncating, so a shorter payload would leave stale
    // trailing bytes and corrupt the JSON. Truncate to zero first.
    file.truncate_file();
    file.write(json);
    log_debug("Saved " + len(tasks) + " tasks to " + data_file());
}

// ── Rendering ─────────────────────────────────────────────────────────────
// Build the table's row array from the task list and push it (plus the header
// count) to the components. Table cells are text-only, so "done" is a ✓ glyph.

fn render_tasks(tasks) {
    let rows = [];
    let i = 0;
    while i < len(tasks) {
        let t = tasks[i];
        // Build each field as a local first: inline `if`/expressions inside a
        // map literal push Rhai past its expression-complexity limit.
        let num = "" + (i + 1);
        let done = "";
        if t.done { done = "✓"; }
        let status = "Pending";
        if t.done { status = "Completed"; }
        let due = format_due(t.due);
        rows.push(#{
            num: num,
            done: done,
            label: t.label,
            due: due,
            status: status,
        });
        i += 1;
    }

    set_component_property("tasks_table", "data", rows);

    let n = len(tasks);
    set_component_text("task_count", "" + n + (if n == 1 { " task" } else { " tasks" }));
}

// Format a due date for display using rhai-chrono. Empty → "No due date"; past
// dates show "Overdue"; today/tomorrow are special-cased; future dates count
// down. An unparseable string is shown verbatim rather than erroring.

fn format_due(due) {
    if due == "" || due == () { return "No due date"; }

    // datetime_parse wraps NaiveDateTime::parse_from_str, which requires a full
    // date-time — a bare "%Y-%m-%d" would fail — so append midnight.
    let days = ();
    try {
        let dt = datetime_parse(due + " 00:00:00", "%Y-%m-%d %H:%M:%S");
        days = dt.diff(datetime_local()).days;
    } catch (err) {
        return due;
    }

    if days < 0 { return "Overdue (" + (-days) + "d)"; }
    if days == 0 { return "Due today"; }
    if days == 1 { return "Due tomorrow"; }
    "Due in " + days + "d"
}

// ── Startup hook ─────────────────────────────────────────────────────────────
// Called once, after the layout is built, via `<script on-load="on_load" />`.

fn on_load(component_id, event_data) {
    let tasks = load_tasks();
    render_tasks(tasks);
    log_info("Loaded " + len(tasks) + " tasks from disk");
}

// ── Add-task modal ────────────────────────────────────────────────────────

fn open_add_modal(component_id, event_data) {
    // Clear the fields from any previous use, then open.
    set_component_property("new_task_input", "value", "");
    set_component_property("new_due_input", "value", "");
    set_component_property("add_modal", "open", true);
}

fn close_add_modal(component_id, event_data) {
    set_component_property("add_modal", "open", false);
}

// Reads the two inputs' live `value` properties (kept current by the input
// readback wiring), appends a task, persists, and closes the modal. Wired to
// both the "Add Task" button and the inputs' on-change (Enter submits).
fn submit_add(component_id, event_data) {
    let label = field_value("new_task_input");
    if label == "" {
        log_warn("submit_add ignored: empty task description");
        return;
    }

    let due = field_value("new_due_input");

    let tasks = load_tasks();
    tasks.push(#{ label: label, done: false, due: due });
    save_tasks(tasks);
    render_tasks(tasks);

    set_component_property("new_task_input", "value", "");
    set_component_property("new_due_input", "value", "");
    set_component_property("add_modal", "open", false);
    log_info("Added task: " + label);
}

// ── Row actions (keyed by the "#" column) ───────────────────────────────────

fn toggle_done(component_id, event_data) {
    let idx = selected_index();
    let tasks = load_tasks();
    if idx >= 0 && idx < len(tasks) {
        tasks[idx].done = !tasks[idx].done;
        save_tasks(tasks);
        render_tasks(tasks);
        log_info("Toggled task " + (idx + 1) + " to done=" + tasks[idx].done);
    } else {
        log_warn("toggle_done: no valid row selected");
    }
}

fn delete_task(component_id, event_data) {
    let idx = selected_index();
    let tasks = load_tasks();
    if idx >= 0 && idx < len(tasks) {
        let removed = tasks[idx].label;
        tasks.remove(idx);
        save_tasks(tasks);
        render_tasks(tasks);
        set_component_property("row_num", "value", "");
        log_info("Deleted task: " + removed);
    } else {
        log_warn("delete_task: no valid row selected");
    }
}

// ── REST API (http-server data source) ──────────────────────────────────────
// Wired via `<source name="rest" type="http-server" port="8420"
// on-request="on_request" />`. The runtime hands each inbound HTTP request to
// `on_request` on the main thread as a map:
//   { id, method, path, headers, body }  plus `query` when the URL had one;
//     body is a parsed JSON map when the request body was valid JSON, else a
//     string.
// The handler returns #{ status: <int>, body: <string>, content_type?: <str> }.
// Routes (row numbers are the 1-based `#` column):
//   GET    /tasks      → {"tasks": [...]}
//   POST   /tasks      body {"label", "due"?} → 201 created task
//   GET    /tasks/<n>  → that task
//   PUT    /tasks/<n>  body {"label"?, "due"?, "done"?} → updated task
//   DELETE /tasks/<n>  → {"removed": <label>} and the UI re-renders

fn on_request(req) {
    log_info("REST " + req.method + " " + req.path);

    let tasks = load_tasks();
    // Leading slash first (rhai keeps the empty string when splitting
    // "/a/b"): "/tasks" → "tasks", "/tasks/3" → "tasks/3". Then split.
    let route = if req.path.starts_with("/") { req.path.sub_string(1) } else { req.path };
    let segs = route.split("/");

    // Only /tasks and /tasks/<n> exist.
    if len(segs) == 0 || segs[0] != "tasks" || len(segs) > 2 {
        return rest_error(404, "unknown route: " + req.path);
    }
    let row = if len(segs) == 2 { segs[1] } else { "" };

    // Collection level: GET lists, POST appends.
    if row == "" {
        if req.method == "GET" {
            return rest_json(200, #{ tasks: tasks });
        }
        if req.method == "POST" {
            let body = req.body;
            let label_missing = body == () || body.label == ();
            let label_blank = !label_missing && ("" + body.label).trim() == "";
            if label_missing || label_blank {
                return rest_error(400, "POST body must include a non-empty \"label\"");
            }
            let label = ("" + body.label).trim();
            let due = field_or_blank(body.due);
            let task = #{ label: label, done: false, due: due };
            tasks.push(task);
            save_tasks(tasks);
            render_tasks(tasks);
            return rest_json(201, task);
        }
        return rest_error(405, "method " + req.method + " not supported on /tasks");
    }

    // Row level: everything below addresses one task.
    let idx = parse_int_safe(row);
    if idx < 1 || idx > len(tasks) {
        return rest_error(404, "no task " + row);
    }
    let i = idx - 1;

    if req.method == "GET" {
        return rest_json(200, tasks[i]);
    }

    if req.method == "PUT" {
        if req.body != () {
            if req.body.label != () { tasks[i].label = "" + req.body.label; }
            if req.body.due != () { tasks[i].due = "" + req.body.due; }
            if req.body.done != () { tasks[i].done = req.body.done; }
        }
        save_tasks(tasks);
        render_tasks(tasks);
        return rest_json(200, tasks[i]);
    }

    if req.method == "DELETE" {
        let removed = tasks[i];
        tasks.remove(i);
        save_tasks(tasks);
        render_tasks(tasks);
        return rest_json(200, #{ removed: removed.label });
    }

    rest_error(405, "method " + req.method + " not supported");
}

// ── REST helpers ─────────────────────────────────────────────────────────────

// Serialize a reply map as JSON with the default 200 status.
fn rest_json(status, value) {
    #{ status: status, body: json_stringify(value) }
}

// Plain-text error reply.
fn rest_error(status, message) {
    #{ status: status, body: message, content_type: "text/plain" }
}

// Optional JSON string field → "" when absent.
fn field_or_blank(value) {
    if value == () { "" } else { "" + value }
}

// parse_int that returns -1 instead of raising on non-numeric input.
fn parse_int_safe(raw) {
    let n = -1;
    try {
        n = parse_int(raw);
    } catch (err) {
        return -1;
    }
    n
}

// ── Helpers ──────────────────────────────────────────────────────────────────

// Read a component's `value` property as a trimmed string ("" if unset).
fn field_value(id) {
    let v = get_component_property(id, "value");
    if v == () { return ""; }
    ("" + v).trim()
}

// The 0-based task index the user typed into the "Row #" field (-1 if blank or
// not a positive integer). Rows are displayed 1-based in the "#" column.
fn selected_index() {
    let raw = field_value("row_num");
    if raw == "" { return -1; }
    // parse_int raises on a non-numeric string, so guard it.
    let n = -1;
    try {
        n = parse_int(raw) - 1;
    } catch (err) {
        return -1;
    }
    n
}
