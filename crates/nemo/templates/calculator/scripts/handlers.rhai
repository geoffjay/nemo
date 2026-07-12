// Calculator Event Handlers
// State is stored as properties on the display component (display_result)

// Helper to get calculator state
fn get_state(key) {
    let val = get_component_property("display_result", key);
    if val == () {
        // Return defaults
        if key == "current_value" { return "0"; }
        if key == "stored_value" { return "0"; }
        if key == "pending_operator" { return ""; }
        if key == "start_new_input" { return "true"; }
    }
    val
}

// Helper to set calculator state
fn set_state(key, value) {
    set_component_property("display_result", key, value);
}

// Update the display with the current value
fn update_display(value) {
    set_component_text("display_result", value);
}

// Handler for digit button clicks (0-9)
fn on_digit(component_id, event_data) {
    let digit = get_component_label(component_id);
    let current_value = get_state("current_value");
    let start_new_input = get_state("start_new_input");

    if start_new_input == "true" {
        current_value = digit;
        set_state("start_new_input", "false");
    } else {
        if current_value == "0" {
            current_value = digit;
        } else {
            current_value = current_value + digit;
        }
    }

    set_state("current_value", current_value);
    update_display(current_value);
    log_debug("Digit pressed: " + digit + ", display: " + current_value);
}

// Handler for decimal point
fn on_decimal(component_id, event_data) {
    let current_value = get_state("current_value");
    let start_new_input = get_state("start_new_input");

    if start_new_input == "true" {
        current_value = "0.";
        set_state("start_new_input", "false");
    } else if !current_value.contains(".") {
        current_value = current_value + ".";
    }

    set_state("current_value", current_value);
    update_display(current_value);
    log_debug("Decimal pressed, display: " + current_value);
}

// Handler for operator buttons (+, -, *, /)
fn on_operator(component_id, event_data) {
    let operator = get_component_label(component_id);
    let current_value = get_state("current_value");
    let pending_operator = get_state("pending_operator");
    let start_new_input = get_state("start_new_input");

    // If there's a pending operation, calculate it first
    if pending_operator != "" && start_new_input != "true" {
        do_calculate();
        current_value = get_state("current_value");
    }

    set_state("stored_value", current_value);
    set_state("pending_operator", operator);
    set_state("start_new_input", "true");

    log_info("Operator set: " + operator + ", stored: " + current_value);
}

// Handler for equals button
fn on_equals(component_id, event_data) {
    let pending_operator = get_state("pending_operator");

    if pending_operator != "" {
        do_calculate();
        set_state("pending_operator", "");
    }
    set_state("start_new_input", "true");

    let current_value = get_state("current_value");
    log_info("Result: " + current_value);
}

// Perform the calculation based on pending operator
fn do_calculate() {
    let current_value = get_state("current_value");
    let stored_value = get_state("stored_value");
    let pending_operator = get_state("pending_operator");

    let input_value = parse_float(current_value);
    let stored = parse_float(stored_value);
    let result = 0.0;

    if pending_operator == "+" {
        result = stored + input_value;
    } else if pending_operator == "-" {
        result = stored - input_value;
    } else if pending_operator == "*" {
        result = stored * input_value;
    } else if pending_operator == "/" {
        if input_value == 0.0 {
            set_state("current_value", "Error");
            update_display("Error");
            log_warn("Division by zero attempted");
            return;
        }
        result = stored / input_value;
    }

    // Format result using to_string (handles whole numbers nicely)
    let result_str = to_string(result);

    set_state("current_value", result_str);
    set_state("stored_value", result_str);
    update_display(result_str);
    log_debug("Calculated: " + stored_value + " " + pending_operator + " " + current_value + " = " + result_str);
}

// Handler for clear button
fn on_clear(component_id, event_data) {
    set_state("current_value", "0");
    set_state("stored_value", "0");
    set_state("pending_operator", "");
    set_state("start_new_input", "true");

    update_display("0");
    log_info("Calculator cleared");
}

// Handler for negate button (+/-)
fn on_negate(component_id, event_data) {
    let current_value = get_state("current_value");

    if current_value != "0" && current_value != "Error" {
        let value = parse_float(current_value);
        value = -value;
        let result = to_string(value);

        set_state("current_value", result);
        update_display(result);
        log_debug("Negated: " + result);
    }
}

// Handler for percent button
fn on_percent(component_id, event_data) {
    let current_value = get_state("current_value");

    if current_value != "Error" {
        let value = parse_float(current_value);
        value = value / 100.0;
        let result = to_string(value);

        set_state("current_value", result);
        update_display(result);
        log_debug("Percent: " + result);
    }
}
