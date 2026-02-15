mod pid;

use nemo_plugin::prelude::*;
use pid::PidController;
use std::sync::Mutex;

/// Builds the PID control panel UI template using the builder API.
fn build_template() -> PluginValue {
    Panel::new()
        .padding(16)
        .border(2)
        .border_color("theme.border")
        .shadow("md")
        .width(300)
        .child("title", Label::new("PID Controller").size("xl"))
        .child(
            "controls",
            Stack::vertical()
                .spacing(8)
                // Kp row
                .child(
                    "kp_row",
                    Stack::horizontal()
                        .spacing(8)
                        .child("kp_label", Label::new("Kp").width(120))
                        .child(
                            "kp_input",
                            Input::new().value("1.0").on_change("on_gain_change"),
                        ),
                )
                // Ki row
                .child(
                    "ki_row",
                    Stack::horizontal()
                        .spacing(8)
                        .child("ki_label", Label::new("Ki").width(120))
                        .child(
                            "ki_input",
                            Input::new().value("0.0").on_change("on_gain_change"),
                        ),
                )
                // Kd row
                .child(
                    "kd_row",
                    Stack::horizontal()
                        .spacing(8)
                        .child("kd_label", Label::new("Kd").width(120))
                        .child(
                            "kd_input",
                            Input::new().value("0.0").on_change("on_gain_change"),
                        ),
                )
                // Setpoint row
                .child(
                    "setpoint_row",
                    Stack::horizontal()
                        .spacing(8)
                        .child("setpoint_label", Label::new("Setpoint").width(120))
                        .child(
                            "setpoint_input",
                            Input::new().value("0.0").on_change("on_setpoint_change"),
                        ),
                )
                // Output display
                .child(
                    "output_label",
                    Label::new("Output: 0.0").bind_text("data.${ns}.output_display"),
                )
                // Enable switch
                .child(
                    "enable_switch",
                    Switch::new()
                        .label("Enable")
                        .checked(false)
                        .on_click("on_enable_toggle"),
                ),
        )
        .build()
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
