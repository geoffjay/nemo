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
mod runtime;
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

            // Get window configuration from config
            let title = runtime
                .get_config("app.window.title")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "Nemo Application".to_string());

            let width = runtime
                .get_config("app.window.width")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let height = runtime
                .get_config("app.window.height")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);

            let runtime = Arc::clone(&runtime);
            let window_options = get_window_options(cx, title, width, height);

            cx.open_window(window_options, |window, cx| {
                // window.set_window_title(&title);

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
