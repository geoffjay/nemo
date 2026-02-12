wit_bindgen::generate!({
    path: "../../../../crates/nemo-wasm/wit/nemo-plugin.wit",
    world: "nemo-plugin",
});

use nemo::plugin::host_api;
use nemo::plugin::types::{LogLevel, PluginValue};

struct MockDataPlugin;

/// Persistent counter — bumped on each tick.
static mut COUNTER: i64 = 0;

impl Guest for MockDataPlugin {
    fn get_manifest() -> PluginManifest {
        PluginManifest {
            id: "mock-data-wasm".into(),
            name: "Mock Data Provider (WASM)".into(),
            version: "0.1.0".into(),
            description: "Provides simulated sensor data for testing data bindings (WASM)".into(),
            author: None,
        }
    }

    fn init() {
        let _ = host_api::set_data("mock.temperature", &PluginValue::FloatVal(22.5));
        let _ = host_api::set_data("mock.humidity", &PluginValue::FloatVal(45.0));
        let _ = host_api::set_data("mock.counter", &PluginValue::IntegerVal(0));
        let _ = host_api::set_data("mock.status", &PluginValue::StringVal("online".to_string()));

        host_api::log(LogLevel::Info, "Mock data WASM plugin initialized");
    }

    fn tick() -> u64 {
        // Safety: WASM is single-threaded, no concurrent access.
        let counter = unsafe {
            COUNTER += 1;
            COUNTER
        };

        // Sine wave temperature (20-25 degrees)
        let temp = 22.5 + 2.5 * (counter as f64 * 0.1).sin();
        let _ = host_api::set_data("mock.temperature", &PluginValue::FloatVal(temp));

        // Cosine humidity (40-60%)
        let humidity = 50.0 + 10.0 * (counter as f64 * 0.07).cos();
        let _ = host_api::set_data("mock.humidity", &PluginValue::FloatVal(humidity));

        // Incrementing counter
        let _ = host_api::set_data("mock.counter", &PluginValue::IntegerVal(counter));

        // Alternating status
        let status = if counter % 10 < 7 {
            "online"
        } else {
            "maintenance"
        };
        let _ = host_api::set_data("mock.status", &PluginValue::StringVal(status.to_string()));

        // Return 2000ms until next tick
        2000
    }
}

export!(MockDataPlugin);
