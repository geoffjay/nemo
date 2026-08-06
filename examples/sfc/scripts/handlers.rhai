// Runtime component creation handlers.
//
// These handlers demonstrate create_component / remove_component: the
// "Add label" button inserts a label under the c4_dynamic stack at runtime,
// and "Remove last" tears down the most recently added one. No pre-declaration
// in XML — the components are created and removed structurally.

// Track the IDs of dynamically created labels so remove_component can target
// the most recent one (stack discipline).
let created_ids = [];

fn on_add_label(component_id, event_data) {
    let count = created_ids.len() + 1;
    let props = #{text: "Label #" + count};
    let id = create_component("c4_dynamic", "label", props);
    if id != "" {
        created_ids.push(id);
    }
}

fn on_remove_label(component_id, event_data) {
    if created_ids.len() > 0 {
        let id = created_ids.pop();
        remove_component(id);
    }
}