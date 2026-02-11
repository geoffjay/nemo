// Component Gallery Navigation Handlers

// All page panel IDs
let ALL_PAGES = [
    "page_button",
    "page_label",
    "page_icon",
    "page_checkbox",
    "page_input",
    "page_select",
    "page_progress",
    "page_notification",
    "page_modal",
    "page_text",
    "page_list",
    "page_panel",
    "page_table",
    "page_tree",
    "page_image",
    "page_charts"
];

// Navigation handler: hides all pages, shows the one matching the clicked button
fn on_nav(component_id, event_data) {
    // Hide all pages
    set_component_property("page_button", "visible", false);
    set_component_property("page_label", "visible", false);
    set_component_property("page_icon", "visible", false);
    set_component_property("page_checkbox", "visible", false);
    set_component_property("page_input", "visible", false);
    set_component_property("page_select", "visible", false);
    set_component_property("page_progress", "visible", false);
    set_component_property("page_notification", "visible", false);
    set_component_property("page_modal", "visible", false);
    set_component_property("page_text", "visible", false);
    set_component_property("page_list", "visible", false);
    set_component_property("page_panel", "visible", false);
    set_component_property("page_table", "visible", false);
    set_component_property("page_tree", "visible", false);
    set_component_property("page_image", "visible", false);
    set_component_property("page_charts", "visible", false);

    // Derive page ID from nav button ID: "nav_button" -> "page_button"
    let page_id = replace(component_id, "nav_", "page_");
    set_component_property(page_id, "visible", true);
}

// Modal toggle handler
fn on_toggle_modal(component_id, event_data) {
    let current = get_component_property("demo_modal", "open");
    if current == true {
        set_component_property("demo_modal", "open", false);
    } else {
        set_component_property("demo_modal", "open", true);
    }
}
