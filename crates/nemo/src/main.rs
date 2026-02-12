//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from HCL files
//! - Initializes all subsystems
//! - Launches the GPUI window

use anyhow::{Context as _, Result};
use gpui::*;
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use gpui_component::Root;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod app;
mod args;
mod components;
pub mod config;
mod runtime;
mod theme;
mod window;
mod workspace;

use args::Args;
use config::NemoConfig;
use window::get_window_options;
use workspace::{ProjectLoaderView, ProjectSelected};

/// The root workspace entity that manages the application state.
/// It starts in either ProjectLoader mode (no app config) or Application mode.
enum WorkspaceState {
    ProjectLoader(Entity<ProjectLoaderView>),
    Application(Entity<app::App>),
}

#[allow(dead_code)]
struct Workspace {
    state: WorkspaceState,
    nemo_config: NemoConfig,
    ws_args: WorkspaceArgs,
}

/// Subset of args needed after initial parse.
#[derive(Clone)]
struct WorkspaceArgs {
    extension_dirs: Vec<PathBuf>,
}

impl Workspace {
    fn load_project(
        &mut self,
        app_config_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        info!("Loading project from: {:?}", app_config_path);

        // Add to recent projects
        let mut recent = config::recent::RecentProjects::load();
        recent.add(app_config_path.clone());
        recent.save();

        match create_runtime(&app_config_path, &self.ws_args.extension_dirs) {
            Ok(rt) => {
                apply_theme_from_runtime(&rt, cx);
                let app_entity = cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                self.state = WorkspaceState::Application(app_entity);
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to load project: {}", e);
            }
        }
    }

    /// Shut down the application entity if active.
    fn shutdown(&self, cx: &mut Context<'_, Self>) {
        if let WorkspaceState::Application(app) = &self.state {
            app.update(cx, |a, cx| {
                a.shutdown(cx);
            });
        }
    }

    /// Create a ProjectLoaderView and subscribe to its events.
    fn create_loader(
        nemo_config: &NemoConfig,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Entity<ProjectLoaderView> {
        let loader = cx.new(|cx| ProjectLoaderView::new(nemo_config.clone(), window, cx));
        cx.subscribe_in(
            &loader,
            window,
            |ws: &mut Workspace, _loader, event: &ProjectSelected, window, cx| {
                ws.load_project(event.0.clone(), window, cx);
            },
        )
        .detach();
        loader
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let bg_color = cx.theme().colors.background;
        let text_color = cx.theme().colors.foreground;

        let content: AnyElement = match &self.state {
            WorkspaceState::ProjectLoader(loader) => loader.clone().into_any_element(),
            WorkspaceState::Application(app) => app.clone().into_any_element(),
        };

        let mut container = v_flex()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .child(content);

        if let Some(dialog_layer) = Root::render_dialog_layer(window, cx) {
            container = container.child(dialog_layer);
        }
        if let Some(notification_layer) = Root::render_notification_layer(window, cx) {
            container = container.child(notification_layer);
        }

        container
    }
}

/// Creates a NemoRuntime, applies extension dirs, loads config, and initializes.
/// Returns the runtime wrapped in Arc on success.
fn create_runtime(
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
fn apply_theme_from_runtime(runtime: &Arc<runtime::NemoRuntime>, cx: &mut gpui::App) {
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

    // Load NemoConfig (config.toml)
    let nemo_config = NemoConfig::load_from(args.config.as_ref());

    // If app_config is provided via CLI/env, handle headless/validate modes
    if let Some(ref app_config) = args.app_config {
        if args.headless || args.validate_only {
            let rt = runtime::NemoRuntime::new(app_config)?;

            for dir in &args.extension_dirs {
                rt.add_extension_dir(dir)?;
            }

            info!("Loading configuration from: {:?}", app_config);
            rt.load_config()?;

            if args.validate_only {
                info!("Configuration validation successful");
                return Ok(());
            }

            info!("Initializing subsystems...");
            rt.initialize()?;

            info!("Running in headless mode");
            rt.run_headless()?;

            info!("Nemo shutdown complete");
            return Ok(());
        }
    }

    // Launch GPUI application
    info!("Starting GPUI application...");
    let gpui_app = Application::new().with_assets(gpui_component_assets::Assets);

    let app_config_path = args.app_config.clone();
    let ws_args = WorkspaceArgs {
        extension_dirs: args.extension_dirs.clone(),
    };

    gpui_app.run(move |cx| {
        gpui_component::init(cx);

        // Store workspace entity for window close handler
        let workspace_entity: Rc<RefCell<Option<Entity<Workspace>>>> = Rc::new(RefCell::new(None));

        cx.on_window_closed({
            let workspace_entity = workspace_entity.clone();
            move |cx| {
                if let Some(ws) = workspace_entity.borrow().clone() {
                    ws.update(cx, |ws, cx| {
                        ws.shutdown(cx);
                    });
                }
                cx.quit();
            }
        })
        .detach();

        // Default window options
        let window_options = get_window_options(cx, None, None, None, None);

        cx.open_window(window_options, |window, cx| {
            let nemo_config = nemo_config.clone();
            let ws_args = ws_args.clone();
            let app_config_path = app_config_path.clone();

            let ws = cx.new(|cx| {
                let state = if let Some(config_path) = app_config_path {
                    info!("Loading project from: {:?}", config_path);

                    let mut recent = config::recent::RecentProjects::load();
                    recent.add(config_path.clone());
                    recent.save();

                    match create_runtime(&config_path, &ws_args.extension_dirs) {
                        Ok(rt) => {
                            apply_theme_from_runtime(&rt, cx);
                            let app_entity =
                                cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                            WorkspaceState::Application(app_entity)
                        }
                        Err(e) => {
                            tracing::error!("Failed to load project: {}", e);
                            let loader = Workspace::create_loader(&nemo_config, window, cx);
                            WorkspaceState::ProjectLoader(loader)
                        }
                    }
                } else {
                    let loader = Workspace::create_loader(&nemo_config, window, cx);
                    WorkspaceState::ProjectLoader(loader)
                };

                Workspace {
                    state,
                    nemo_config,
                    ws_args,
                }
            });

            *workspace_entity.borrow_mut() = Some(ws.clone());
            cx.new(|_cx| Root::new(ws, window, _cx))
        })
        .expect("Failed to open window");
    });

    info!("Nemo shutdown complete");
    Ok(())
}
