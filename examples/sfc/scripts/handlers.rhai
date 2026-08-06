// Handlers for the single-file-components demo.
//
// This file drives two runtime features:
//
//  1. Runtime component creation (create_component / remove_component) — the
//     "Add label" / "Remove last" buttons in card 4.
//  2. Control-flow directives (n:if / n:for over live data) — the toggle in
//     card 6 and the task list in card 7. The directives themselves are
//     declared in app.xml; these handlers only mutate the backing data, and the
//     runtime reacts (toggling `visible`, creating/removing list instances).
//
// `on_load` runs once at startup (wired via <script on-load="on_load">) to seed
// the data the directives read, so the demo opens in a meaningful state.

// ── Startup seed ─────────────────────────────────────────────────────────────

fn on_load(component_id, event_data) {
    // n:if in card 6 binds `visible` to `data.ui.show_details == 'yes'`.
    set_data("ui.show_details", "yes");
    // n:for in card 7 renders one row per item in `data.tasks`.
    set_data("tasks", [
        #{ id: "t1", name: "Write the parser" },
        #{ id: "t2", name: "Wire the runtime" },
    ]);
}

// ── Card 4: runtime component creation ───────────────────────────────────────
//
// The "Add label" button inserts a label under the c4_dynamic stack at runtime,
// and "Remove last" tears down the most recently added one. The created ids are
// tracked in the data repository (not module state) so they survive across
// handler calls.

fn on_add_label(component_id, event_data) {
    let ids = get_data("dyn_label_ids");
    if ids == () { ids = []; }
    let count = ids.len() + 1;
    let props = #{text: "Label #" + count};
    let id = create_component("c4_dynamic", "label", props);
    if id != "" {
        ids.push(id);
        set_data("dyn_label_ids", ids);
    }
}

fn on_remove_label(component_id, event_data) {
    let ids = get_data("dyn_label_ids");
    if ids == () { return; }
    if ids.len() > 0 {
        let id = ids.pop();
        remove_component(id);
        set_data("dyn_label_ids", ids);
    }
}

// ── Card 6: n:if conditional visibility ──────────────────────────────────────
//
// Flips `data.ui.show_details` between "yes" and "no". The panel's compiled
// `bind-visible` binding evaluates `== 'yes'` and toggles `visible`.

fn on_toggle_details(component_id, event_data) {
    let cur = get_data("ui.show_details");
    if cur == "yes" {
        set_data("ui.show_details", "no");
    } else {
        set_data("ui.show_details", "yes");
    }
}

// ── Card 7: live-data n:for (add / remove / reorder) ─────────────────────────
//
// The n:for over `data.tasks` creates/removes a row per task as the array
// changes. `n:key="task.id"` gives each row a stable identity, so reordering
// moves the existing instance instead of recreating it.

fn on_add_task(component_id, event_data) {
    let tasks = get_data("tasks");
    if tasks == () { tasks = []; }
    // A monotonic id keeps keys stable even after removals.
    let next = get_data("task_seq");
    if next == () { next = tasks.len(); }
    next = next + 1;
    tasks.push(#{ id: "t" + next, name: "Task #" + next });
    set_data("task_seq", next);
    set_data("tasks", tasks);
}

fn on_remove_task(component_id, event_data) {
    let tasks = get_data("tasks");
    if tasks == () { return; }
    if tasks.len() > 0 {
        tasks.pop();
        set_data("tasks", tasks);
    }
}

fn on_reverse_tasks(component_id, event_data) {
    let tasks = get_data("tasks");
    if tasks == () { return; }
    tasks.reverse();
    set_data("tasks", tasks);
}
