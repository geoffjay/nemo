// Task List event handlers with on-disk persistence.
//
// IMPORTANT: Rhai script functions are PURE. They cannot read or mutate
// script-level (module) `let`/`var` state, and a function called from another
// function gets a fresh scope. So this script keeps NO mutable module state.
// Instead:
//   * per-task state lives on disk in tasks.json (via rhai-fs), and
//   * the one-shot "have we applied saved state to the UI yet?" flag lives in
//     host data storage (set_data / get_data), which every function can reach.
//
// Uses rhai-fs (gated by the `file-io` feature in app.xml) for file I/O,
// rhai-chrono for due-date arithmetic, and the json_parse / json_stringify
// helpers for (de)serialization.
//
// File layout (tasks.json):
//   { "tasks": [ {"icon":"📋","label":"...","done":false,"due":"2026-07-15"}, ... ] }

// Shared constants are exposed as zero-arg functions, NOT top-level `const`.
// A top-level `const`/`let` is not visible inside a function called from
// another handler (script functions are pure and only see their parameters
// plus other functions), so a function is the reliable way to share a value.

// Path to the persistence file, resolved relative to the working directory.
fn data_file() { "examples/task-list/tasks.json" }

// Host-data key for the one-shot "applied to UI" flag. Not bound to any
// component, so writing it has no visible side effect.
fn applied_key() { "task_list.applied" }

// ── Persistence ─────────────────────────────────────────────────────────────
// Every handler is self-contained: it loads the task array from disk (or the
// defaults), works with it as a local, and writes it back. No shared cache.

fn load_tasks() {
    if !path(data_file()).exists {
        log_info("No tasks.json found; starting with defaults");
        return default_tasks();
    }

    let file = open_file(data_file(), "r");
    let contents = file.read_string();
    let parsed = json_parse(contents);

    if parsed == () || parsed.tasks == () {
        log_warn("tasks.json is not valid; starting with defaults");
        return default_tasks();
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

// ── Defaults ─────────────────────────────────────────────────────────────────
// Used when no tasks.json exists yet. Matches the 12 rows in app.xml.

fn default_tasks() {
    [
        #{ icon: "📋", label: "Finish the quarterly report", done: false, due: "2026-07-15" },
        #{ icon: "🛒", label: "Buy groceries for the week",    done: false, due: "2026-07-13" },
        #{ icon: "💪", label: "Go for a morning run",          done: false, due: "" },
        #{ icon: "🏠", label: "Fix the leaking kitchen sink", done: false, due: "2026-07-20" },
        #{ icon: "⭐", label: "Prepare the client presentation", done: false, due: "2026-07-14" },
        #{ icon: "📋", label: "Reply to outstanding client emails", done: false, due: "" },
        #{ icon: "🎯", label: "Read 20 pages of the current book", done: false, due: "2026-07-12" },
        #{ icon: "🏠", label: "Water the plants",              done: false, due: "" },
        #{ icon: "🛒", label: "Order a birthday gift",        done: false, due: "2026-07-18" },
        #{ icon: "📋", label: "Plan the team offsite",        done: false, due: "2026-07-25" },
        #{ icon: "💪", label: "Book a dentist appointment",   done: false, due: "" },
        #{ icon: "⭐", label: "Submit the tax documents",     done: false, due: "2026-07-30" },
    ]
}

// ── Component sync ──────────────────────────────────────────────────────────
// Apply the persisted task state to the XML components exactly once per run,
// so the UI reflects the saved state on the first interaction. The "already
// applied" flag lives in host data storage because a plain module variable
// would not survive across handler calls.

fn apply_saved_state() {
    if get_data(applied_key()) == true { return; }
    set_data(applied_key(), true);

    let tasks = load_tasks();
    let n = len(tasks);
    let i = 1;
    while i <= n {
        let task = tasks[i - 1];

        set_component_label("icon_" + i, task.icon);
        set_component_property("check_" + i, "checked", task.done);
        set_component_label("check_" + i, task.label);
        set_component_text("due_" + i, format_due(task.due));
        set_component_text("status_" + i, if task.done { "✓ Completed" } else { "Pending" });

        i += 1;
    }
    log_info("Applied " + n + " saved tasks to the UI");
}

// Format a due date for display using rhai-chrono. Empty string → "No due
// date"; past dates show "Overdue"; today/tomorrow are special-cased; future
// dates show "Due in N days".

fn format_due(due) {
    if due == "" || due == () { return "No due date"; }

    // datetime_parse wraps NaiveDateTime::parse_from_str, which requires a
    // full date-time — a bare "%Y-%m-%d" would fail — so append midnight.
    let dt = datetime_parse(due + " 00:00:00", "%Y-%m-%d %H:%M:%S");
    let days = dt.diff(datetime_local()).days;

    if days < 0 { return "Overdue (" + (-days) + "d)"; }
    if days == 0 { return "Due today"; }
    if days == 1 { return "Due tomorrow"; }
    "Due in " + days + "d (" + due + ")"
}

// ── Event handlers ──────────────────────────────────────────────────────────

fn toggle_task(component_id, event_data) {
    apply_saved_state();

    let done = event_data == "true";
    set_component_property(component_id, "checked", done);

    let status_id = component_id.replace("check_", "status_");
    set_component_text(status_id, if done { "✓ Completed" } else { "Pending" });

    let idx = parse_index(component_id, "check_");
    let tasks = load_tasks();
    if idx >= 0 && idx < len(tasks) {
        tasks[idx].done = done;
        save_tasks(tasks);
    }

    log_info("Task " + component_id + " completed=" + event_data);
}

fn open_icon_picker(component_id, event_data) {
    apply_saved_state();

    set_component_property("icon_picker", "editing", component_id);
    set_component_property("icon_picker", "open", true);
    log_info("Opening icon picker for " + component_id);
}

fn choose_icon(component_id, event_data) {
    let target = get_component_property("icon_picker", "editing");
    let emoji = get_component_label(component_id);

    if target != () && target != "" {
        set_component_label(target, emoji);

        let idx = parse_index(target, "icon_");
        let tasks = load_tasks();
        if idx >= 0 && idx < len(tasks) {
            tasks[idx].icon = emoji;
            save_tasks(tasks);
        }

        log_info("Set " + target + " icon to " + emoji);
    } else {
        log_warn("choose_icon called with no active target");
    }

    set_component_property("icon_picker", "open", false);
}

fn close_icon_picker(component_id, event_data) {
    set_component_property("icon_picker", "open", false);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

// Extract the numeric suffix from an id like "check_7" → 0-based index 6.
fn parse_index(id, prefix) {
    let num_str = id.replace(prefix, "");
    parse_int(num_str) - 1
}
