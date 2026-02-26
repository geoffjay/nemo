//! Nemo UI - GPUI desktop rendering backend.
//!
//! This crate contains all GPUI-specific code for rendering Nemo applications
//! as desktop GUI windows.

use anyhow::Result;
use gpui::*;
use gpui_component::Root;
use gpui_router::{init as router_init, use_navigate};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;

pub mod app;
pub mod components;
pub mod config;
pub mod project;
pub mod runtime;
pub mod theme;
pub mod window;
pub mod workspace;

use config::NemoConfig;
use project::ActiveProject;
use window::get_window_options;
use workspace::actions::{
    CloseProject, CloseSettings, OpenProject, OpenSettings, QuitApp, ReloadConfig,
    ShowKeyboardShortcuts, ToggleTheme,
};
use workspace::utils::{apply_theme_from_runtime, create_runtime};
use workspace::{FooterBar, HeaderBar, Workspace, WorkspaceArgs};

/// Run the GPUI desktop application.
///
/// This is the main entry point for the desktop UI backend. It creates a GPUI
/// application window, sets up routing, key bindings, and renders the workspace.
pub fn run_gpui(
    nemo_config: NemoConfig,
    app_config_path: Option<PathBuf>,
    extension_dirs: Vec<PathBuf>,
) -> Result<()> {
    info!("Starting GPUI application...");
    let gpui_app = Application::new().with_assets(gpui_component_assets::Assets);

    let ws_args = WorkspaceArgs {
        extension_dirs: extension_dirs.clone(),
    };

    gpui_app.run(move |cx| {
        gpui_component::init(cx);
        router_init(cx);

        // Apply theme from TOML config (base app settings)
        if nemo_config.app.theme_name != "default" {
            theme::apply_configured_theme(&nemo_config.app.theme_name, "system", None, cx);
        }

        cx.bind_keys([
            KeyBinding::new("ctrl-shift-r", ReloadConfig, None),
            KeyBinding::new("ctrl-q", QuitApp, None),
            KeyBinding::new("ctrl-w", CloseProject, None),
            KeyBinding::new("ctrl-o", OpenProject, None),
            KeyBinding::new("ctrl-shift-t", ToggleTheme, None),
            KeyBinding::new("ctrl-p", OpenSettings, None),
            KeyBinding::new("escape", CloseSettings, None),
            KeyBinding::new("f10", ShowKeyboardShortcuts, None),
        ]);

        // Store workspace entity for window close handler
        let workspace_entity: Rc<RefCell<Option<Entity<Workspace>>>> =
            Rc::new(RefCell::new(None));

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
                let mut current_route = "/".to_string();

                // If app_config provided via CLI, load project immediately
                if let Some(ref config_path) = app_config_path {
                    info!("Loading project from: {:?}", config_path);

                    let mut recent_projects = config::recent::RecentProjects::load();
                    recent_projects.add(config_path.clone());
                    recent_projects.save();

                    match create_runtime(config_path, &ws_args.extension_dirs) {
                        Ok(rt) => {
                            apply_theme_from_runtime(&rt, cx);
                            let title = rt
                                .get_config("app.window.title")
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                .unwrap_or_else(|| "Nemo Application".to_string());
                            let github_url = rt
                                .get_config("app.window.header_bar.github_url")
                                .and_then(|v| v.as_str().map(|s| s.to_string()));
                            let theme_toggle = rt
                                .get_config("app.window.header_bar.theme_toggle")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let header_bar = cx.new(|cx| {
                                HeaderBar::new(title, github_url, theme_toggle, window, cx)
                            });
                            let footer_bar_enabled = rt
                                .get_config("app.window.footer_bar.enabled")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let footer_bar = if footer_bar_enabled {
                                Some(cx.new(|cx| FooterBar::new(window, cx)))
                            } else {
                                None
                            };
                            let app_entity =
                                cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                            cx.set_global(ActiveProject {
                                runtime: rt,
                                app_entity,
                                header_bar,
                                footer_bar,
                                settings_view: None,
                            });
                            current_route = "/app".to_string();
                        }
                        Err(e) => {
                            tracing::error!("Failed to load project: {}", e);
                        }
                    }
                }

                let focus_handle = cx.focus_handle();
                focus_handle.focus(window);

                let loader = Workspace::create_loader(&nemo_config, window, cx);

                Workspace {
                    nemo_config,
                    ws_args,
                    current_config_path: app_config_path.clone(),
                    pending_project_path: None,
                    pending_close_project: false,
                    focus_handle,
                    current_route: current_route.clone(),
                    loader,
                }
            });

            // Navigate to the initial route after window creation
            let route = ws.read(cx).current_route.clone();
            let needs_refresh = route != "/";
            use_navigate(cx)(route.into());
            if needs_refresh {
                window.refresh();
            }

            *workspace_entity.borrow_mut() = Some(ws.clone());
            cx.new(|_cx| Root::new(ws, window, _cx))
        })
        .expect("Failed to open window");
    });

    info!("Nemo shutdown complete");
    Ok(())
}
