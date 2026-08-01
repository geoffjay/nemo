use gpui::*;
use gpui_component::h_flex;
use gpui_component::label::Label;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime;
use crate::theme;
use crate::theme::tokens::{Space, TokenStyled};

/// Creates a NemoRuntime, applies extension dirs, loads config, and initializes.
/// Returns the runtime wrapped in Arc on success.
pub fn create_runtime(
    config_path: &std::path::Path,
    extension_dirs: &[PathBuf],
    initial_route: Option<&str>,
) -> Result<Arc<runtime::NemoRuntime>> {
    let rt = runtime::NemoRuntime::new(config_path)?;

    for dir in extension_dirs {
        let _ = rt.add_extension_dir(dir);
    }

    rt.load_config()?;
    rt.initialize()?;

    // A `--route` override must be recorded before the first render so a router
    // picks it up on lazy init.
    if let Some(route) = initial_route {
        rt.set_initial_route(route);
    }

    #[allow(clippy::arc_with_non_send_sync)]
    Ok(Arc::new(rt))
}

/// Apply theme settings from a loaded runtime.
pub fn apply_theme_from_runtime(runtime: &Arc<runtime::NemoRuntime>, cx: &mut gpui::App) {
    // Register any project-defined theme sets (`<themes><theme-set src=.../></themes>`)
    // before resolving the selected theme, so custom names resolve. Called
    // unconditionally (empty list clears the overlay) so switching to a project
    // without custom themes doesn't leave stale entries from a previous project.
    let srcs: Vec<String> = runtime
        .get_config("themes")
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.get("src").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let base_dir = runtime
        .config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    theme::register_project_theme_sets(&base_dir, &srcs);

    if let Some(theme_name) = runtime
        .get_config("app.theme.name")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        let mode = runtime
            .get_config("app.theme.mode")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "dark".to_string());

        // `app.theme.extend` is a flat `{ "color.key": "#hex", ... }` object built
        // from `<theme><extend><color key value/></extend></theme>`; merge it over
        // the resolved base theme.
        let overrides = runtime
            .get_config("app.theme.extend")
            .and_then(|extend_val| {
                let obj = extend_val.as_object()?;
                let json_obj: serde_json::Map<String, serde_json::Value> = obj
                    .iter()
                    .filter_map(|(k, v)| {
                        v.as_str()
                            .map(|s| (k.clone(), serde_json::Value::String(s.to_string())))
                    })
                    .collect();
                if json_obj.is_empty() {
                    return None;
                }
                serde_json::from_value(serde_json::Value::Object(json_obj)).ok()
            });

        theme::apply_configured_theme(&theme_name, &mode, overrides.as_ref(), cx);
    }

    // Apply per-project font family override
    if let Some(font_family) = runtime
        .get_config("app.theme.font_family")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        gpui_component::Theme::global_mut(cx).font_family = font_family.into();
    }

    // Apply per-project roundness override (after theme so it isn't reset).
    // Accepts `app.theme.roundness` (consistent with font_family) or `app.roundness`.
    if let Some(roundness) = runtime
        .get_config("app.theme.roundness")
        .or_else(|| runtime.get_config("app.roundness"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        theme::apply_roundness(&roundness, cx);
    }
}

/// Render a single row for the keyboard shortcuts dialog.
pub fn shortcut_row(label: &str, keystroke: &str) -> impl IntoElement {
    let kbd = gpui_component::kbd::Kbd::new(Keystroke::parse(keystroke).unwrap());
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .py_t(Space::Xs)
        .child(Label::new(label.to_string()))
        .child(kbd)
}
