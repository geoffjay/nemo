use nemo_plugin_api::*;
use std::sync::Arc;

fn init(registrar: &mut dyn PluginRegistrar) {
    // Get the plugin context for background data updates
    let ctx = registrar.context_arc();

    // Set initial mock data
    let _ = ctx.set_data("mock.temperature", PluginValue::Float(22.5));
    let _ = ctx.set_data("mock.humidity", PluginValue::Float(45.0));
    let _ = ctx.set_data("mock.counter", PluginValue::Integer(0));
    let _ = ctx.set_data("mock.status", PluginValue::String("online".to_string()));

    ctx.log(LogLevel::Info, "Mock data plugin initialized");

    // Spawn a background thread that periodically updates mock values
    std::thread::spawn(move || {
        let mut counter: i64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));

            counter += 1;

            // Sine wave temperature (20-25 degrees)
            let temp = 22.5 + 2.5 * (counter as f64 * 0.1).sin();
            let _ = ctx.set_data("mock.temperature", PluginValue::Float(temp));

            // Random-ish humidity (40-60%)
            let humidity = 50.0 + 10.0 * (counter as f64 * 0.07).cos();
            let _ = ctx.set_data("mock.humidity", PluginValue::Float(humidity));

            // Incrementing counter
            let _ = ctx.set_data("mock.counter", PluginValue::Integer(counter));

            // Alternating status
            let status = if counter % 10 < 7 { "online" } else { "maintenance" };
            let _ = ctx.set_data(
                "mock.status",
                PluginValue::String(status.to_string()),
            );
        }
    });
}

declare_plugin!(
    PluginManifest::new("mock-data", "Mock Data Provider", semver::Version::new(0, 1, 0))
        .with_description("Provides simulated sensor data for testing data bindings")
        .with_capability(Capability::DataSource("mock".to_string())),
    init
);
