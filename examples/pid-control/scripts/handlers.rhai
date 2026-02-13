// PID Control Demo - Event Handlers

// Called when any gain input changes (Kp, Ki, Kd).
// The component_id tells us which gain to update.
fn on_gain_change(component_id, value) {
    let float_val = parse_float(value);
    if float_val == () {
        return;
    }

    if component_id.contains("kp") {
        set_data("pid.kp", float_val);
    } else if component_id.contains("ki") {
        set_data("pid.ki", float_val);
    } else if component_id.contains("kd") {
        set_data("pid.kd", float_val);
    }
}

// Called when the setpoint input changes.
fn on_setpoint_change(component_id, value) {
    let float_val = parse_float(value);
    if float_val != () {
        set_data("pid.setpoint", float_val);
        set_data("pid.sp_display", "Setpoint: " + value);
    }
}

// Called when the enable switch is toggled.
fn on_enable_toggle(component_id, value) {
    let current = get_data("pid.enabled");
    let new_val = if current == true { false } else { true };
    set_data("pid.enabled", new_val);
}

// Called on each plant timer tick.
// Simulates a simple first-order plant: pv += (output - pv) * 0.1
fn on_plant_tick(component_id, value) {
    let enabled = get_data("pid.enabled");
    if enabled != true {
        return;
    }

    let pv = get_data("pid.process_variable");
    if pv == () { pv = 0.0; }

    let output = get_data("pid.output");
    if output == () { output = 0.0; }

    let setpoint = get_data("pid.setpoint");
    if setpoint == () { setpoint = 0.0; }

    // First-order plant dynamics
    let new_pv = pv + (output - pv) * 0.1;
    set_data("pid.process_variable", new_pv);

    // Update display values
    let error = setpoint - new_pv;
    set_data("pid.pv_display", "PV: " + format_float(new_pv));
    set_data("pid.error_display", "Error: " + format_float(error));
    set_data("pid.sp_display", "Setpoint: " + format_float(setpoint));
}

// Helper: format a float to 3 decimal places
fn format_float(val) {
    // RHAI doesn't have format specifiers, use string conversion
    let s = "" + val;
    // Find decimal point
    let dot = s.index_of(".");
    if dot == () || dot < 0 {
        return s + ".000";
    }
    let decimals = s.len() - dot - 1;
    if decimals > 3 {
        return s.sub_string(0, dot + 4);
    }
    while decimals < 3 {
        s += "0";
        decimals += 1;
    }
    s
}
