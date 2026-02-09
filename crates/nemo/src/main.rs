//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from HCL files
//! - Initializes all subsystems
//! - Launches the GPUI window

use anyhow::{Context, Result};
use gpui::*;
use gpui_component::Root;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod app;
mod args;
mod components;
mod runtime;
mod theme;
mod window;
mod workspace;

use app::App;
use args::Args;
use window::get_window_options;

fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level())
        .with_target(true)
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    info!("Nemo v{} starting...", env!("CARGO_PKG_VERSION"));

    let runtime = runtime::NemoRuntime::new(&args.config)?;

    for dir in &args.config_dirs {
        runtime.add_config_dir(dir)?;
    }

    for dir in &args.extension_dirs {
        runtime.add_extension_dir(dir)?;
    }

    info!("Loading configuration from: {:?}", args.config);
    runtime.load_config()?;

    if args.validate_only {
        info!("Configuration validation successful");
        return Ok(());
    }

    info!("Initializing subsystems...");
    runtime.initialize()?;

    if args.headless {
        info!("Running in headless mode");
        runtime.run_headless()?;
    } else {
        info!("Starting GPUI application...");
        let runtime = Arc::new(runtime);
        let app = Application::new().with_assets(gpui_component_assets::Assets);

        app.run(move |cx| {
            gpui_component::init(cx);

            // Apply configured theme if specified
            if let Some(theme_name) = runtime
                .get_config("app.theme.name")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
            {
                let mode = runtime
                    .get_config("app.theme.mode")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "dark".to_string());

                // Parse extend block: app.theme.extend is { "theme_name": { key: val, ... } }
                let overrides = runtime
                    .get_config("app.theme.extend")
                    .and_then(|extend_val| {
                        let obj = extend_val.as_object()?;
                        // Get the first (and typically only) labeled block's inner value
                        let (_, inner) = obj.iter().next()?;
                        let inner_obj = inner.as_object()?;
                        // Convert to serde_json::Value for deserialization
                        let json_obj: serde_json::Map<String, serde_json::Value> = inner_obj
                            .iter()
                            .filter_map(|(k, v)| {
                                v.as_str()
                                    .map(|s| (k.clone(), serde_json::Value::String(s.to_string())))
                            })
                            .collect();
                        serde_json::from_value(serde_json::Value::Object(json_obj)).ok()
                    });

                theme::apply_configured_theme(
                    &theme_name,
                    &mode,
                    overrides.as_ref(),
                    cx,
                );
            }

            // Store the App entity so we can access it on window close
            let app_entity: Rc<RefCell<Option<Entity<App>>>> = Rc::new(RefCell::new(None));

            // Close all sessions and quit the application when the window is closed
            cx.on_window_closed({
                let app_entity = app_entity.clone();
                move |cx| {
                    // Shutdown the app to close any open sessions
                    if let Some(app) = app_entity.borrow().clone() {
                        app.update(cx, |app, app_cx| {
                            app.shutdown(app_cx);
                        });
                    }
                    cx.quit();
                }
            })
            .detach();

            let width = runtime
                .get_config("app.window.width")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let height = runtime
                .get_config("app.window.height")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let min_width = runtime
                .get_config("app.window.min_width")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let min_height = runtime
                .get_config("app.window.min_height")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let runtime = Arc::clone(&runtime);
            let window_options = get_window_options(cx, width, height, min_width, min_height);

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| App::new(runtime, window, cx));
                *app_entity.borrow_mut() = Some(view.clone());
                cx.new(|_cx| Root::new(view, window, _cx))
            })
            .expect("Failed to open window");
        });
    }

    info!("Nemo shutdown complete");
    Ok(())
}
