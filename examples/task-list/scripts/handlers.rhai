// Task List event handlers with on-disk persistence
//
// State is persisted to `tasks.json` next to the app config. On first
// interaction the saved state is loaded and applied to the components;
// every toggle / icon change writes back. Uses rhai-fs (gated by the
// `file-io` feature in app.xml) for file I/O, rhai-chrono for due-date
// arithmetic, and the json_parse / json_stringify helpers for (de)serialization.
//
// File layout (tasks.json):
//   { "tasks": [ {"icon":"📋","label":"...","done":false,"due":"2026-07-15"}, ... ] }

// Path to the persistence file, resolved relative to the script location.
const DATA_FILE = "examples/task-list/tasks.json";

// In-memory cache of the task list (a rhai array of maps). Loaded once
// from disk on first use, then kept in sync with the components.
let tasks_cache = ();
let tasks_loaded = false;

// ── Persistence ─────────────────────────────────────────────────────────────

fn load_tasks() {
    if tasks_loaded { return; }
    tasks_loaded = true;

    if !exists(path(DATA_FILE)) {
        log_info("No tasks.json found; starting with defaults");
        tasks_cache = default_tasks();
        return;
    }

    let file = open_file(DATA_FILE, "r");
    let contents = file.read_string();
    let parsed = json_parse(contents);

    if parsed == () {
        log_warn("tasks.json is not valid JSON; starting with defaults");
        tasks_cache = default_tasks();
        return;
    }

    tasks_cache = parsed.tasks;
    log_info("Loaded " + len(tasks_cache) + " tasks from " + DATA_FILE);
}

fn save_tasks() {
    let obj = #{ tasks: tasks_cache };
    let json = json_stringify(obj);
    let file = open_file(DATA_FILE, "w");
    file.write(json);
    log_debug("Saved tasks to " + DATA_FILE);
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
// Apply the cached task state to the XML components. Called once on first
// interaction so the UI reflects the persisted state.

fn apply_tasks_to_components() {
    let n = len(tasks_cache);
    let i = 1;
    while i <= n {
        let task = tasks_cache[i - 1];

        set_component_label("icon_" + i, task.icon);
        set_component_property("check_" + i, "checked", task.done);
        set_component_label("check_" + i, task.label);
        set_component_text("due_" + i, format_due(task.due));
        set_component_text("status_" + i, if task.done { "✓ Completed" } else { "Pending" });

        i += 1;
    }
}

// Format a due date for display using rhai-chrono. Empty string → "No due
// date"; past dates show "Overdue"; future dates show "Due in N days" or
// the ISO date if parsing fails.

fn format_due(due) {
    if due == "" || due == () { return "No due date"; }

    let dt = datetime_parse(due, "%Y-%m-%d");
    if dt == () { return "Due " + due; }

    let now = datetime_local();
    let diff = dt.diff(now);
    let days = diff.days;

    if days < 0 { return "Overdue (" + (-days) + "d)"; }
    if days == 0 { return "Due today"; }
    if days == 1 { return "Due tomorrow"; }
    "Due in " + days + "d (" + due + ")"
}

// ── Event handlers ──────────────────────────────────────────────────────────

fn toggle_task(component_id, event_data) {
    if !tasks_loaded { load_tasks(); apply_tasks_to_components(); }

    let done = event_data == "true";
    let idx = parse_index(component_id, "check_");

    set_component_property(component_id, "checked", done);

    let status_id = component_id.replace("check_", "status_");
    if done {
        set_component_text(status_id, "✓ Completed");
    } else {
        set_component_text(status_id, "Pending");
    }

    tasks_cache[idx].done = done;
    save_tasks();

    log_info("Task " + component_id + " completed=" + event_data);
}

fn open_icon_picker(component_id, event_data) {
    if !tasks_loaded { load_tasks(); apply_tasks_to_components(); }

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
        tasks_cache[idx].icon = emoji;
        save_tasks();

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