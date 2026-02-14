mod pid;

use nemo_plugin_api::*;
use pid::PidController;
use std::collections::HashMap;
use std::sync::Mutex;

/// Builds the PID control panel UI template as a PluginValue tree.
fn build_template() -> PluginValue {
    let mut root = HashMap::new();
    root.insert("type".to_string(), PluginValue::String("panel".to_string()));
    root.insert("padding".to_string(), PluginValue::Integer(16));
    root.insert("border".to_string(), PluginValue::Integer(2));
    root.insert(
        "border_color".to_string(),
        PluginValue::String("theme.border".to_string()),
    );
    root.insert("shadow".to_string(), PluginValue::String("md".to_string()));

    // Build child components
    let mut children = HashMap::new();

    // Title
    let mut title = HashMap::new();
    title.insert("type".to_string(), PluginValue::String("label".to_string()));
    title.insert(
        "text".to_string(),
        PluginValue::String("PID Controller".to_string()),
    );
    children.insert("title".to_string(), PluginValue::Object(title));

    // Inner stack for vertical layout
    let mut inner_stack = HashMap::new();
    inner_stack.insert("type".to_string(), PluginValue::String("stack".to_string()));
    inner_stack.insert(
        "direction".to_string(),
        PluginValue::String("vertical".to_string()),
    );
    inner_stack.insert("spacing".to_string(), PluginValue::Integer(8));

    let mut inner_children = HashMap::new();

    // Kp input
    let mut kp_input = HashMap::new();
    kp_input.insert("type".to_string(), PluginValue::String("input".to_string()));
    kp_input.insert(
        "placeholder".to_string(),
        PluginValue::String("Kp".to_string()),
    );
    kp_input.insert("value".to_string(), PluginValue::String("1.0".to_string()));
    kp_input.insert(
        "on_change".to_string(),
        PluginValue::String("on_gain_change".to_string()),
    );
    inner_children.insert("kp_input".to_string(), PluginValue::Object(kp_input));

    // Ki input
    let mut ki_input = HashMap::new();
    ki_input.insert("type".to_string(), PluginValue::String("input".to_string()));
    ki_input.insert(
        "placeholder".to_string(),
        PluginValue::String("Ki".to_string()),
    );
    ki_input.insert("value".to_string(), PluginValue::String("0.0".to_string()));
    ki_input.insert(
        "on_change".to_string(),
        PluginValue::String("on_gain_change".to_string()),
    );
    inner_children.insert("ki_input".to_string(), PluginValue::Object(ki_input));

    // Kd input
    let mut kd_input = HashMap::new();
    kd_input.insert("type".to_string(), PluginValue::String("input".to_string()));
    kd_input.insert(
        "placeholder".to_string(),
        PluginValue::String("Kd".to_string()),
    );
    kd_input.insert("value".to_string(), PluginValue::String("0.0".to_string()));
    kd_input.insert(
        "on_change".to_string(),
        PluginValue::String("on_gain_change".to_string()),
    );
    inner_children.insert("kd_input".to_string(), PluginValue::Object(kd_input));

    // Setpoint input
    let mut sp_input = HashMap::new();
    sp_input.insert("type".to_string(), PluginValue::String("input".to_string()));
    sp_input.insert(
        "placeholder".to_string(),
        PluginValue::String("Setpoint".to_string()),
    );
    sp_input.insert("value".to_string(), PluginValue::String("0.0".to_string()));
    sp_input.insert(
        "on_change".to_string(),
        PluginValue::String("on_setpoint_change".to_string()),
    );
    inner_children.insert("setpoint_input".to_string(), PluginValue::Object(sp_input));

    // Output display
    let mut output_label = HashMap::new();
    output_label.insert("type".to_string(), PluginValue::String("label".to_string()));
    output_label.insert(
        "text".to_string(),
        PluginValue::String("Output: 0.0".to_string()),
    );
    output_label.insert(
        "bind_text".to_string(),
        PluginValue::String("data.${ns}.output_display".to_string()),
    );
    inner_children.insert(
        "output_label".to_string(),
        PluginValue::Object(output_label),
    );

    // Enable switch
    let mut enable_switch = HashMap::new();
    enable_switch.insert(
        "type".to_string(),
        PluginValue::String("switch".to_string()),
    );
    enable_switch.insert(
        "label".to_string(),
        PluginValue::String("Enable".to_string()),
    );
    enable_switch.insert("checked".to_string(), PluginValue::Bool(false));
    enable_switch.insert(
        "on_click".to_string(),
        PluginValue::String("on_enable_toggle".to_string()),
    );
    inner_children.insert(
        "enable_switch".to_string(),
        PluginValue::Object(enable_switch),
    );

    inner_stack.insert("component".to_string(), PluginValue::Object(inner_children));
    children.insert("controls".to_string(), PluginValue::Object(inner_stack));

    root.insert("component".to_string(), PluginValue::Object(children));

    PluginValue::Object(root)
}

fn init(registrar: &mut dyn PluginRegistrar) {
    let ctx = registrar.context_arc();

    // Register the UI template
    registrar.register_template("pid_control", build_template());

    // Set initial data values
    let _ = ctx.set_data("pid.kp", PluginValue::Float(1.0));
    let _ = ctx.set_data("pid.ki", PluginValue::Float(0.0));
    let _ = ctx.set_data("pid.kd", PluginValue::Float(0.0));
    let _ = ctx.set_data("pid.setpoint", PluginValue::Float(0.0));
    let _ = ctx.set_data("pid.process_variable", PluginValue::Float(0.0));
    let _ = ctx.set_data("pid.output", PluginValue::Float(0.0));
    let _ = ctx.set_data("pid.enabled", PluginValue::Bool(false));
    let _ = ctx.set_data(
        "pid.output_display",
        PluginValue::String("Output: 0.0".to_string()),
    );

    // Read configurable I/O paths
    let input_path = ctx
        .get_config("plugins.pid.input_path")
        .and_then(|v| match v {
            PluginValue::String(s) => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| "pid.process_variable".to_string());

    let output_path = ctx
        .get_config("plugins.pid.output_path")
        .and_then(|v| match v {
            PluginValue::String(s) => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| "pid.output".to_string());

    let interval_ms = ctx
        .get_config("plugins.pid.interval_ms")
        .and_then(|v| match v {
            PluginValue::Integer(i) => Some(i as u64),
            _ => None,
        })
        .unwrap_or(100);

    ctx.log(LogLevel::Info, "PID control plugin initialized");

    // Spawn the PID control loop
    let controller = std::sync::Arc::new(Mutex::new(PidController::new(1.0, 0.0, 0.0)));

    std::thread::spawn(move || {
        let dt = interval_ms as f64 / 1000.0;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));

            // Check if enabled
            let enabled = ctx
                .get_data("pid.enabled")
                .map(|v| matches!(v, PluginValue::Bool(true)))
                .unwrap_or(false);

            if !enabled {
                continue;
            }

            // Read gains and update controller
            let kp = get_float(ctx.as_ref(), "pid.kp").unwrap_or(1.0);
            let ki = get_float(ctx.as_ref(), "pid.ki").unwrap_or(0.0);
            let kd = get_float(ctx.as_ref(), "pid.kd").unwrap_or(0.0);

            let mut pid = controller.lock().unwrap();
            pid.set_gains(kp, ki, kd);

            // Read setpoint and process variable
            let setpoint = get_float(ctx.as_ref(), "pid.setpoint").unwrap_or(0.0);
            let pv = get_float(ctx.as_ref(), &input_path).unwrap_or(0.0);

            // Compute
            let output = pid.compute(setpoint, pv, dt);
            drop(pid);

            // Write output
            let _ = ctx.set_data(&output_path, PluginValue::Float(output));
            let _ = ctx.set_data(
                "pid.output_display",
                PluginValue::String(format!("Output: {:.3}", output)),
            );
        }
    });
}

/// Helper to extract a float from plugin data.
fn get_float(ctx: &dyn PluginContext, path: &str) -> Option<f64> {
    ctx.get_data(path).and_then(|v| match v {
        PluginValue::Float(f) => Some(f),
        PluginValue::Integer(i) => Some(i as f64),
        PluginValue::String(s) => s.parse().ok(),
        _ => None,
    })
}

declare_plugin!(
    PluginManifest::new(
        "pid-control",
        "PID Controller",
        semver::Version::new(0, 1, 0)
    )
    .with_description("PID controller plugin with configurable I/O and live-tunable gains")
    .with_capability(Capability::DataSource("pid".to_string())),
    init
);
