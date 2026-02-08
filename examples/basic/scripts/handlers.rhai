// Nemo Event Handlers
// This script is loaded automatically from the scripts directory

// Handler for button click events
// Parameters:
//   component_id: The ID of the component that triggered the event
//   event_data: Additional event data (e.g., "click")
fn on_button_click(component_id, event_data) {
    // Log the event
    log_info("Button clicked: " + component_id);
    log_info("Event data: " + event_data);

    // You can access configuration and data through the context API:
    // let value = get_data("some.path");
    // let config = get_config("app.title");
}

// Example of a more complex handler
fn on_form_submit(component_id, event_data) {
    log_info("Form submitted from: " + component_id);

    // Parse event data if it's JSON
    // let data = parse_json(event_data);

    // Perform validation
    // if data.name == "" {
    //     log_warn("Name is required");
    //     return false;
    // }

    log_info("Form data: " + event_data);
}

// Handler for input change events
fn on_input_change(component_id, event_data) {
    log_debug("Input changed: " + component_id + " = " + event_data);
}
