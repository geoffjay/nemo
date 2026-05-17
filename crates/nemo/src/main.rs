//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from XML files
//! - Initializes all subsystems
//! - Launches the GPUI window with router-based navigation

use anyhow::{Context as _, Result};
use gpui::*;
use gpui_component::Root;
use gpui_router::{init as router_init, use_navigate};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod app;
mod args;
mod components;
pub mod config;
mod project;
mod runtime;
mod storybook;
mod theme;
mod window;
mod workspace;

use args::{Args, Commands};
use config::NemoConfig;
use project::ActiveProject;
use window::get_window_options;
use workspace::actions::{
    CloseProject, CloseSettings, OpenProject, OpenSettings, QuitApp, ReloadConfig,
    ShowKeyboardShortcuts, ToggleTheme,
};
use workspace::utils::{apply_theme_from_runtime, create_runtime};
use workspace::{FooterBar, HeaderBar, Workspace, WorkspaceArgs};

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

    // Handle storybook subcommand: generate config and launch
    if let Some(Commands::Storybook(ref sb_args)) = args.command {
        return launch_storybook(sb_args, &args);
    }

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
    let gpui_app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    let app_config_path = args.app_config.clone();
    let ws_args = WorkspaceArgs {
        extension_dirs: args.extension_dirs.clone(),
    };

    gpui_app.run(move |cx| {
        gpui_component::init(cx);
        router_init(cx);

        // Apply theme from TOML config (base app settings)
        if nemo_config.app.theme_name != "default" {
            theme::apply_configured_theme(&nemo_config.app.theme_name, "system", None, cx);
        }

        // Apply global font family from TOML config
        if let Some(ref font_family) = nemo_config.app.font_family {
            gpui_component::Theme::global_mut(cx).font_family = font_family.clone().into();
        }

        // Wrap config in Arc<Mutex<>> for sharing with settings view
        let nemo_config = Arc::new(Mutex::new(nemo_config));

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
        let workspace_entity: Rc<RefCell<Option<Entity<Workspace>>>> = Rc::new(RefCell::new(None));

        cx.on_window_closed({
            let workspace_entity = workspace_entity.clone();
            move |cx, _window_id| {
                if let Some(ws) = workspace_entity.borrow().clone() {
                    ws.update(cx, |ws, cx| {
                        ws.shutdown(cx);
                    });
                }
                cx.quit();
            }
        })
        .detach();

        // If app_config provided, create runtime early so we can read window dimensions
        let early_runtime = app_config_path.as_ref().and_then(|config_path| {
            match create_runtime(config_path, &ws_args.extension_dirs) {
                Ok(rt) => Some(rt),
                Err(e) => {
                    tracing::error!("Failed to load project: {}", e);
                    None
                }
            }
        });

        // Read window dimensions from runtime config (if available)
        let (win_w, win_h, win_min_w, win_min_h) = if let Some(ref rt) = early_runtime {
            let w = rt
                .get_config("app.window.width")
                .and_then(|v| v.as_i64().map(|n| n as u32));
            let h = rt
                .get_config("app.window.height")
                .and_then(|v| v.as_i64().map(|n| n as u32));
            let mw = rt
                .get_config("app.window.min_width")
                .and_then(|v| v.as_i64().map(|n| n as u32));
            let mh = rt
                .get_config("app.window.min_height")
                .and_then(|v| v.as_i64().map(|n| n as u32));
            (w, h, mw, mh)
        } else {
            (None, None, None, None)
        };

        let window_options = get_window_options(cx, win_w, win_h, win_min_w, win_min_h);

        cx.open_window(window_options, |window, cx| {
            let nemo_config = nemo_config.clone();
            let ws_args = ws_args.clone();
            let app_config_path = app_config_path.clone();

            let ws = cx.new(|cx| {
                let mut current_route = "/".to_string();

                // If app_config provided via CLI, use the early-created runtime
                if let Some(config_path) = app_config_path {
                    info!("Loading project from: {:?}", config_path);

                    let mut recent_projects = config::recent::RecentProjects::load();
                    recent_projects.add(config_path.clone());
                    recent_projects.save();

                    if let Some(rt) = early_runtime {
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
                        let header_bar = cx
                            .new(|cx| HeaderBar::new(title, github_url, theme_toggle, window, cx));
                        let footer_bar_enabled = rt
                            .get_config("app.window.footer_bar.enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let footer_bar = if footer_bar_enabled {
                            Some(cx.new(|cx| FooterBar::new(window, cx)))
                        } else {
                            None
                        };
                        let app_entity = cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                        cx.set_global(ActiveProject {
                            runtime: rt,
                            app_entity,
                            header_bar,
                            footer_bar,
                            settings_view: None,
                        });
                        current_route = "/app".to_string();
                    }
                }

                let focus_handle = cx.focus_handle();
                focus_handle.focus(window, cx);

                let loader = Workspace::create_loader(&nemo_config, window, cx);

                Workspace {
                    nemo_config,
                    ws_args,
                    current_config_path: if current_route == "/app" {
                        args.app_config.clone()
                    } else {
                        None
                    },
                    pending_project_path: None,
                    pending_close_project: false,
                    focus_handle,
                    current_route,
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

fn launch_storybook(sb_args: &args::StorybookArgs, args: &Args) -> Result<()> {
    use std::env;

    // Determine storybook config output path
    let storybook_config = if let Some(data_dir) = dirs::data_dir() {
        data_dir.join("nemo").join("storybook.xml")
    } else {
        std::env::temp_dir().join("nemo-storybook.xml")
    };

    // Generate the storybook XML config
    info!("Generating storybook config at: {:?}", storybook_config);
    if let Some(parent) = storybook_config.parent() {
        std::fs::create_dir_all(parent).context("Failed to create storybook config directory")?;
    }
    let xml = nemo_storybook_generator::generate_storybook_xml();
    std::fs::write(&storybook_config, xml)
        .context("Failed to write storybook config")?;

    // Build the initial route based on --component flag
    let initial_component = sb_args.component.clone();
    let initial_search = sb_args.search.clone();

    // Store component and search terms as env vars for the runtime to pick up
    if let Some(ref search) = initial_search {
        env::set_var("NEMO_STORYBOOK_SEARCH", search);
    }
    if let Some(ref comp) = initial_component {
        env::set_var("NEMO_STORYBOOK_COMPONENT", comp);
    }

    // Launch the app with the storybook config
    let nemo_config = config::NemoConfig::load_from(args.config.as_ref());
    info!("Starting storybook application...");
    let gpui_app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    let ws_args = WorkspaceArgs {
        extension_dirs: args.extension_dirs.clone(),
    };

    gpui_app.run(move |cx| {
        gpui_component::init(cx);
        router_init(cx);

        if nemo_config.app.theme_name != "default" {
            theme::apply_configured_theme(&nemo_config.app.theme_name, "system", None, cx);
        }
        if let Some(ref font_family) = nemo_config.app.font_family {
            gpui_component::Theme::global_mut(cx).font_family = font_family.clone().into();
        }

        let nemo_config = Arc::new(Mutex::new(nemo_config));

        cx.bind_keys([
            KeyBinding::new("ctrl-shift-r", workspace::actions::ReloadConfig, None),
            KeyBinding::new("ctrl-q", workspace::actions::QuitApp, None),
            KeyBinding::new("ctrl-w", workspace::actions::CloseProject, None),
            KeyBinding::new("ctrl-shift-t", workspace::actions::ToggleTheme, None),
            KeyBinding::new("ctrl-p", workspace::actions::OpenSettings, None),
            KeyBinding::new("escape", workspace::actions::CloseSettings, None),
        ]);

        let workspace_entity: Rc<RefCell<Option<Entity<Workspace>>>> = Rc::new(RefCell::new(None));

        cx.on_window_closed({
            let workspace_entity = workspace_entity.clone();
            move |cx, _window_id| {
                if let Some(ws) = workspace_entity.borrow().clone() {
                    ws.update(cx, |ws, cx| { ws.shutdown(cx); });
                }
                cx.quit();
            }
        }).detach();

        let early_runtime = workspace::utils::create_runtime(&storybook_config, &ws_args.extension_dirs).ok();

        let (win_w, win_h, win_min_w, win_min_h) = if let Some(ref rt) = early_runtime {
            let w = rt.get_config("app.window.width").and_then(|v| v.as_i64().map(|n| n as u32));
            let h = rt.get_config("app.window.height").and_then(|v| v.as_i64().map(|n| n as u32));
            let mw = rt.get_config("app.window.min_width").and_then(|v| v.as_i64().map(|n| n as u32));
            let mh = rt.get_config("app.window.min_height").and_then(|v| v.as_i64().map(|n| n as u32));
            (w, h, mw, mh)
        } else {
            (None, None, None, None)
        };

        let window_options = window::get_window_options(cx, win_w, win_h, win_min_w, win_min_h);

        cx.open_window(window_options, |window, cx| {
            let nemo_config = nemo_config.clone();
            let ws_args = ws_args.clone();
            let storybook_config = storybook_config.clone();
            let initial_component = initial_component.clone();

            let ws = cx.new(|cx| {
                let mut current_route = if let Some(ref comp) = initial_component {
                    format!("/component/{}", comp)
                } else {
                    "/app".to_string()
                };

                if let Some(rt) = early_runtime {
                    apply_theme_from_runtime(&rt, cx);
                    let title = rt.get_config("app.window.title")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "Nemo Storybook".to_string());
                    let github_url = rt.get_config("app.window.header_bar.github_url")
                        .and_then(|v| v.as_str().map(|s| s.to_string()));
                    let theme_toggle = rt.get_config("app.window.header_bar.theme_toggle")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let header_bar = cx.new(|cx| HeaderBar::new(title, github_url, theme_toggle, window, cx));
                    let app_entity = cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                    cx.set_global(project::ActiveProject {
                        runtime: rt,
                        app_entity,
                        header_bar,
                        footer_bar: None,
                        settings_view: None,
                    });
                } else {
                    current_route = "/".to_string();
                }

                let focus_handle = cx.focus_handle();
                focus_handle.focus(window, cx);
                let loader = Workspace::create_loader(&nemo_config, window, cx);

                Workspace {
                    nemo_config,
                    ws_args,
                    current_config_path: Some(storybook_config),
                    pending_project_path: None,
                    pending_close_project: false,
                    focus_handle,
                    current_route,
                    loader,
                }
            });

            let route = ws.read(cx).current_route.clone();
            let needs_refresh = route != "/";
            use_navigate(cx)(route.into());
            if needs_refresh { window.refresh(); }

            *workspace_entity.borrow_mut() = Some(ws.clone());
            cx.new(|_cx| Root::new(ws, window, _cx))
        }).expect("Failed to open storybook window");
    });

    info!("Nemo storybook shutdown complete");
    Ok(())
}
