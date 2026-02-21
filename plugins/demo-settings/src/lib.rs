use indexmap::IndexMap;
use nemo_plugin_api::*;

fn init(registrar: &mut dyn PluginRegistrar) {
    // Build a settings page using PluginValue objects
    let page = PluginValue::Object(IndexMap::from([(
        "children".to_string(),
        PluginValue::Array(vec![
            // Panel: General Settings
            PluginValue::Object(IndexMap::from([
                ("type".to_string(), PluginValue::String("panel".to_string())),
                (
                    "title".to_string(),
                    PluginValue::String("Display Options".to_string()),
                ),
                (
                    "children".to_string(),
                    PluginValue::Array(vec![
                        PluginValue::Object(IndexMap::from([
                            (
                                "type".to_string(),
                                PluginValue::String("switch".to_string()),
                            ),
                            (
                                "label".to_string(),
                                PluginValue::String("Enable Notifications".to_string()),
                            ),
                            ("default".to_string(), PluginValue::Bool(true)),
                        ])),
                        PluginValue::Object(IndexMap::from([
                            (
                                "type".to_string(),
                                PluginValue::String("switch".to_string()),
                            ),
                            (
                                "label".to_string(),
                                PluginValue::String("Auto-refresh Data".to_string()),
                            ),
                            ("default".to_string(), PluginValue::Bool(false)),
                        ])),
                    ]),
                ),
            ])),
            // Panel: Connection Settings
            PluginValue::Object(IndexMap::from([
                ("type".to_string(), PluginValue::String("panel".to_string())),
                (
                    "title".to_string(),
                    PluginValue::String("Connection".to_string()),
                ),
                (
                    "children".to_string(),
                    PluginValue::Array(vec![
                        PluginValue::Object(IndexMap::from([
                            ("type".to_string(), PluginValue::String("input".to_string())),
                            (
                                "label".to_string(),
                                PluginValue::String("Server URL".to_string()),
                            ),
                            (
                                "placeholder".to_string(),
                                PluginValue::String("https://api.example.com".to_string()),
                            ),
                        ])),
                        PluginValue::Object(IndexMap::from([
                            (
                                "type".to_string(),
                                PluginValue::String("slider".to_string()),
                            ),
                            (
                                "label".to_string(),
                                PluginValue::String("Refresh Interval (seconds)".to_string()),
                            ),
                            ("min".to_string(), PluginValue::Integer(1)),
                            ("max".to_string(), PluginValue::Integer(60)),
                            ("step".to_string(), PluginValue::Integer(1)),
                            ("value".to_string(), PluginValue::Integer(10)),
                        ])),
                    ]),
                ),
            ])),
            // A standalone button
            PluginValue::Object(IndexMap::from([
                (
                    "type".to_string(),
                    PluginValue::String("button".to_string()),
                ),
                (
                    "label".to_string(),
                    PluginValue::String("Test Connection".to_string()),
                ),
            ])),
        ]),
    )]));

    registrar.register_settings_page("Demo Plugin", page);

    registrar
        .context()
        .log(LogLevel::Info, "Demo settings plugin initialized");
}

declare_plugin!(
    PluginManifest::new(
        "demo-settings",
        "Demo Settings Plugin",
        semver::Version::new(0, 1, 0)
    )
    .with_description("Demonstrates plugin settings pages in Nemo")
    .with_capability(Capability::Settings("Demo Plugin".to_string())),
    init
);
