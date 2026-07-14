// Task List event handlers
//
// State lives on the components themselves (via set_component_property), which
// is the same pattern the calculator example uses: a handler mutates a stored
// property and the next render reflects it.

// ── Completing tasks ────────────────────────────────────────────────────────
// A checkbox is stateless — it reads its `checked` property on every render and
// reverts unless the handler writes the new value back. `event_data` is the new
// state as a string ("true" / "false").
fn toggle_task(component_id, event_data) {
    let done = event_data == "true";

    // Persist the checked state so the box stays checked across re-renders.
    set_component_property(component_id, "checked", done);

    // Each checkbox "check_N" has a sibling status label "status_N".
    let status_id = component_id.replace("check_", "status_");
    if done {
        set_component_text(status_id, "✓ Completed");
    } else {
        set_component_text(status_id, "Pending");
    }

    log_info("Task " + component_id + " completed=" + event_data);
}

// ── Changing a task's icon via the popup ────────────────────────────────────
// The far-left icon of each row is a ghost button whose label is an emoji.
// Clicking it opens the shared icon-picker modal and records which button is
// being edited in the modal's `editing` property.
fn open_icon_picker(component_id, event_data) {
    set_component_property("icon_picker", "editing", component_id);
    set_component_property("icon_picker", "open", true);
    log_info("Opening icon picker for " + component_id);
}

// A picker option was clicked: copy its emoji onto the task's icon button
// (recorded in `editing`) and close the popup.
fn choose_icon(component_id, event_data) {
    let target = get_component_property("icon_picker", "editing");
    let emoji = get_component_label(component_id);

    if target != () && target != "" {
        set_component_label(target, emoji);
        log_info("Set " + target + " icon to " + emoji);
    } else {
        log_warn("choose_icon called with no active target");
    }

    set_component_property("icon_picker", "open", false);
}

// Cancel button in the popup.
fn close_icon_picker(component_id, event_data) {
    set_component_property("icon_picker", "open", false);
}
