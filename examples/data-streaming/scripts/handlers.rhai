// Data Streaming Dashboard Navigation Handlers

fn on_nav(component_id, event_data) {
    // Hide all pages
    set_component_property("page_charts", "visible", false);
    set_component_property("page_tables", "visible", false);

    // Derive page ID from nav item ID: "nav_charts" -> "page_charts"
    let page_id = replace(component_id, "nav_", "page_");
    set_component_property(page_id, "visible", true);
}
