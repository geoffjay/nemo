use gpui::*;
use gpui_component::h_flex;
use gpui_component::label::Label;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime;
use crate::theme;

/// Creates a NemoRuntime, applies extension dirs, loads config, and initializes.
/// Returns the runtime wrapped in Arc on success.
pub fn create_runtime(
    config_path: &std::path::Path,
    extension_dirs: &[PathBuf],
) -> Result<Arc<runtime::NemoRuntime>> {
    let rt = runtime::NemoRuntime::new(config_path)?;

    for dir in extension_dirs {
        let _ = rt.add_extension_dir(dir);
    }

    rt.load_config()?;
    rt.initialize()?;

    #[allow(clippy::arc_with_non_send_sync)]
    Ok(Arc::new(rt))
}

/// Apply theme settings from a loaded runtime.
pub fn apply_theme_from_runtime(runtime: &Arc<runtime::NemoRuntime>, cx: &mut gpui::App) {
    if let Some(theme_name) = runtime
        .get_config("app.theme.name")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        let mode = runtime
            .get_config("app.theme.mode")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "dark".to_string());

        let overrides = runtime
            .get_config("app.theme.extend")
            .and_then(|extend_val| {
                let obj = extend_val.as_object()?;
                let (_, inner) = obj.iter().next()?;
                let inner_obj = inner.as_object()?;
                let json_obj: serde_json::Map<String, serde_json::Value> = inner_obj
                    .iter()
                    .filter_map(|(k, v)| {
                        v.as_str()
                            .map(|s| (k.clone(), serde_json::Value::String(s.to_string())))
                    })
                    .collect();
                serde_json::from_value(serde_json::Value::Object(json_obj)).ok()
            });

        theme::apply_configured_theme(&theme_name, &mode, overrides.as_ref(), cx);
    }
}

/// Render a single row for the keyboard shortcuts dialog.
pub fn shortcut_row(label: &str, keystroke: &str) -> impl IntoElement {
    let kbd = gpui_component::kbd::Kbd::new(Keystroke::parse(keystroke).unwrap());
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .py_1()
        .child(Label::new(label.to_string()))
        .child(kbd)
}
