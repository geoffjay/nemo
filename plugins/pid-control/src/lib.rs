mod pid;

use indexmap::IndexMap;
use nemo_plugin_api::*;
use pid::PidController;
use std::sync::Mutex;

/// Helper to build a PluginValue::Object from a list of key-value pairs.
fn pv_obj(pairs: &[(&str, PluginValue)]) -> PluginValue {
    let mut map = IndexMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    PluginValue::Object(map)
}

fn pv_str(s: &str) -> PluginValue {
    PluginValue::String(s.to_string())
}

/// Builds a horizontal row with a label and an input side by side.
fn input_row(label: &str, value: &str, on_change: &str) -> (PluginValue, PluginValue) {
    let label_component = pv_obj(&[("type", pv_str("label")), ("text", pv_str(label))]);
    let input_component = pv_obj(&[
        ("type", pv_str("input")),
        ("value", pv_str(value)),
        ("on_change", pv_str(on_change)),
    ]);
    (label_component, input_component)
}

/// Builds the PID control panel UI template as a PluginValue tree.
fn build_template() -> PluginValue {
    // Each gain/setpoint gets a horizontal row: label + input
    let (kp_label, kp_input) = input_row("Kp", "1.0", "on_gain_change");
    let (ki_label, ki_input) = input_row("Ki", "0.0", "on_gain_change");
    let (kd_label, kd_input) = input_row("Kd", "0.0", "on_gain_change");
    let (sp_label, sp_input) = input_row("Setpoint", "0.0", "on_setpoint_change");

    let mut rows = IndexMap::new();

    // Kp row
    let mut kp_children = IndexMap::new();
    kp_children.insert("kp_label".to_string(), kp_label);
    kp_children.insert("kp_input".to_string(), kp_input);
    rows.insert(
        "kp_row".to_string(),
        pv_obj(&[
            ("type", pv_str("stack")),
            ("direction", pv_str("horizontal")),
            ("spacing", PluginValue::Integer(8)),
            ("component", PluginValue::Object(kp_children)),
        ]),
    );

    // Ki row
    let mut ki_children = IndexMap::new();
    ki_children.insert("ki_label".to_string(), ki_label);
    ki_children.insert("ki_input".to_string(), ki_input);
    rows.insert(
        "ki_row".to_string(),
        pv_obj(&[
            ("type", pv_str("stack")),
            ("direction", pv_str("horizontal")),
            ("spacing", PluginValue::Integer(8)),
            ("component", PluginValue::Object(ki_children)),
        ]),
    );

    // Kd row
    let mut kd_children = IndexMap::new();
    kd_children.insert("kd_label".to_string(), kd_label);
    kd_children.insert("kd_input".to_string(), kd_input);
    rows.insert(
        "kd_row".to_string(),
        pv_obj(&[
            ("type", pv_str("stack")),
            ("direction", pv_str("horizontal")),
            ("spacing", PluginValue::Integer(8)),
            ("component", PluginValue::Object(kd_children)),
        ]),
    );

    // Setpoint row
    let mut sp_children = IndexMap::new();
    sp_children.insert("setpoint_label".to_string(), sp_label);
    sp_children.insert("setpoint_input".to_string(), sp_input);
    rows.insert(
        "setpoint_row".to_string(),
        pv_obj(&[
            ("type", pv_str("stack")),
            ("direction", pv_str("horizontal")),
            ("spacing", PluginValue::Integer(8)),
            ("component", PluginValue::Object(sp_children)),
        ]),
    );

    // Output display
    rows.insert(
        "output_label".to_string(),
        pv_obj(&[
            ("type", pv_str("label")),
            ("text", pv_str("Output: 0.0")),
            ("bind_text", pv_str("data.${ns}.output_display")),
        ]),
    );

    // Enable switch
    rows.insert(
        "enable_switch".to_string(),
        pv_obj(&[
            ("type", pv_str("switch")),
            ("label", pv_str("Enable")),
            ("checked", PluginValue::Bool(false)),
            ("on_click", pv_str("on_enable_toggle")),
        ]),
    );

    // Controls vertical stack containing all rows
    let controls = pv_obj(&[
        ("type", pv_str("stack")),
        ("direction", pv_str("vertical")),
        ("spacing", PluginValue::Integer(8)),
        ("component", PluginValue::Object(rows)),
    ]);

    // Title
    let title = pv_obj(&[("type", pv_str("label")), ("text", pv_str("PID Controller"))]);

    // Root panel
    let mut children = IndexMap::new();
    children.insert("title".to_string(), title);
    children.insert("controls".to_string(), controls);

    pv_obj(&[
        ("type", pv_str("panel")),
        ("padding", PluginValue::Integer(16)),
        ("border", PluginValue::Integer(2)),
        ("border_color", pv_str("theme.border")),
        ("shadow", pv_str("md")),
        ("component", PluginValue::Object(children)),
    ])
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
