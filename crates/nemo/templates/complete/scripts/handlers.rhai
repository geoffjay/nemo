// Complete Example - UI Event Handlers

// Page panel IDs for navigation
let ALL_PAGES = [
    "page_dashboard",
    "page_monitoring",
    "page_settings"
];

// Navigation handler: hides all pages, shows the one matching the clicked button
fn on_nav(component_id, event_data) {
    set_component_property("page_dashboard", "visible", false);
    set_component_property("page_monitoring", "visible", false);
    set_component_property("page_settings", "visible", false);

    // Derive page ID from nav button ID: "nav_dashboard" -> "page_dashboard"
    let page_id = replace(component_id, "nav_", "page_");
    set_component_property(page_id, "visible", true);
}

// Reset counter button handler
fn on_reset_counter(component_id, event_data) {
    set_data("app.counter", 0);
}

// Toggle auto-refresh
fn on_toggle_refresh(component_id, event_data) {
    let current = get_data("app.auto_refresh");
    let new_val = if current == true { false } else { true };
    set_data("app.auto_refresh", new_val);
}
