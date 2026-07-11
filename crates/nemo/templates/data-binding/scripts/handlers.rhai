// Data Binding Example - RHAI Event Handlers

// Read data from the ticker source and display it
fn on_read_data(component_id, event_data) {
    let ticker = get_data("ticker");
    if ticker != () {
        let msg = "Ticker data: " + ticker.to_string();
        set_component_text("result_display", msg);
        log_info("Read ticker data: " + ticker.to_string());
    } else {
        set_component_text("result_display", "No ticker data available yet");
        log_warn("No ticker data available");
    }
}

// Write data via set_data and observe the binding update
fn on_write_data(component_id, event_data) {
    let timestamp = "Manual update at tick";
    set_data("manual.message", timestamp);
    set_component_text("result_display", "Data written: " + timestamp);
    log_info("Wrote manual data");
}
