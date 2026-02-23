//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from HCL files
//! - Initializes all subsystems
//! - Launches the GPUI window with router-based navigation

use anyhow::{Context as _, Result};
use gpui::*;
use gpui_component::label::Label;
use gpui_component::notification::{Notification as Toast, NotificationType};
use gpui_component::ActiveTheme;
use gpui_component::Root;
use gpui_component::WindowExt as _;
use gpui_component::{h_flex, v_flex};
use gpui_router::{init as router_init, use_navigate, Route, Routes};
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
use workspace::layout::AppLayout;
use workspace::{FooterBar, HeaderBar, ProjectLoaderView, ProjectSelected};

actions!(
    nemo,
    [
        ReloadConfig,
        QuitApp,
        CloseProject,
        OpenProject,
        ToggleTheme,
        ShowKeyboardShortcuts,
        OpenSettings,
        CloseSettings,
    ]
);

/// Subset of args needed after initial parse.
#[derive(Clone)]
struct WorkspaceArgs {
    extension_dirs: Vec<PathBuf>,
}

/// GPUI global that holds the active project state.
/// Set when a project is loaded, cleared when closed.
impl Global for ActiveProject {}

struct ActiveProject {
    runtime: Arc<runtime::NemoRuntime>,
    app_entity: Entity<app::App>,
    header_bar: Entity<HeaderBar>,
    footer_bar: Option<Entity<FooterBar>>,
    settings_view: Option<Entity<workspace::settings::SettingsView>>,
}

/// The root workspace entity that manages the application state.
#[allow(dead_code)]
struct Workspace {
    nemo_config: NemoConfig,
    ws_args: WorkspaceArgs,
    current_config_path: Option<PathBuf>,
    pending_project_path: Option<PathBuf>,
    pending_close_project: bool,
    focus_handle: FocusHandle,
    /// Current route path for the router.
    current_route: String,
    /// The project loader view entity (persists across renders).
    loader: Entity<ProjectLoaderView>,
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
                let header_bar = self.create_header_bar(&rt, window, cx);
                let footer_bar = self.create_footer_bar(&rt, window, cx);
                let app_entity = cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                cx.set_global(ActiveProject {
                    runtime: rt,
                    app_entity,
                    header_bar,
                    footer_bar,
                    settings_view: None,
                });
                self.current_config_path = Some(app_config_path);
                self.current_route = "/app".to_string();

                use_navigate(cx)("/app".into());
                window.refresh();
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to load project: {}", e);
            }
        }
    }

    fn create_header_bar(
        &self,
        runtime: &Arc<runtime::NemoRuntime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<HeaderBar> {
        let title = runtime
            .get_config("app.window.title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Nemo Application".to_string());
        let github_url = runtime
            .get_config("app.window.header_bar.github_url")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let theme_toggle = runtime
            .get_config("app.window.header_bar.theme_toggle")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        cx.new(|cx| HeaderBar::new(title, github_url, theme_toggle, window, cx))
    }

    fn create_footer_bar(
        &self,
        runtime: &Arc<runtime::NemoRuntime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<FooterBar>> {
        let enabled = runtime
            .get_config("app.window.footer_bar.enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if enabled {
            Some(cx.new(|cx| FooterBar::new(window, cx)))
        } else {
            None
        }
    }

    /// Shut down the active project if one exists.
    fn shutdown(&self, cx: &mut Context<'_, Self>) {
        let app_entity = cx
            .try_global::<ActiveProject>()
            .map(|p| p.app_entity.clone());
        if let Some(entity) = app_entity {
            entity.update(cx, |a, cx| {
                a.shutdown(cx);
            });
        }
    }

    fn reload_config(&mut self, _: &ReloadConfig, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config_path) = self.current_config_path.clone() else {
            return;
        };

        tracing::info!("Reloading configuration from: {:?}", config_path);

        match create_runtime(&config_path, &self.ws_args.extension_dirs) {
            Ok(rt) => {
                self.shutdown(cx);
                apply_theme_from_runtime(&rt, cx);
                let header_bar = self.create_header_bar(&rt, window, cx);
                let footer_bar = self.create_footer_bar(&rt, window, cx);
                let app_entity = cx.new(|cx| app::App::new(Arc::clone(&rt), window, cx));
                cx.set_global(ActiveProject {
                    runtime: rt,
                    app_entity,
                    header_bar,
                    footer_bar,
                    settings_view: None,
                });
                self.current_route = "/app".to_string();

                use_navigate(cx)("/app".into());
                window.refresh();
                window.push_notification("Configuration reloaded", cx);
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Reload failed: {}", e);
                window.push_notification(
                    Toast::new()
                        .message(format!("Reload failed: {}", e))
                        .with_type(NotificationType::Error),
                    cx,
                );
            }
        }
    }

    fn quit_app(&mut self, _: &QuitApp, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title("Quit Application")
                .child(Label::new(
                    "Are you sure you want to quit? Any unsaved work will be lost.",
                ))
                .confirm()
                .on_ok(move |_, _window, cx| {
                    if let Some(ws) = entity.upgrade() {
                        ws.update(cx, |ws, cx| {
                            tracing::info!("Quitting application");
                            ws.shutdown(cx);
                            cx.quit();
                        });
                    }
                    true
                })
        });
    }

    fn close_project(&mut self, _: &CloseProject, window: &mut Window, cx: &mut Context<Self>) {
        if cx.try_global::<ActiveProject>().is_none() {
            return;
        }

        let entity = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title("Close Project")
                .child(Label::new(
                    "Are you sure you want to close the current project?",
                ))
                .confirm()
                .on_ok(move |_, _window, cx| {
                    if let Some(ws) = entity.upgrade() {
                        ws.update(cx, |ws, cx| {
                            ws.pending_close_project = true;
                            cx.notify();
                        });
                    }
                    true
                })
        });
    }

    fn open_project(&mut self, _: &OpenProject, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select an app.hcl configuration file".into()),
        });

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |ws, cx| {
                        ws.pending_project_path = Some(path);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn toggle_theme(&mut self, _: &ToggleTheme, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_component::{Theme, ThemeMode};

        let current_mode = Theme::global(cx).mode;
        let new_mode = if current_mode == ThemeMode::Dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };

        theme::change_color_mode(new_mode, window, cx);

        let mode_name = if new_mode == ThemeMode::Dark {
            "dark"
        } else {
            "light"
        };
        window.push_notification(format!("Switched to {} mode", mode_name), cx);
        cx.notify();
    }

    fn open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        if cx.try_global::<ActiveProject>().is_none() {
            return;
        }

        // Ensure settings view entity exists
        let needs_create = cx.global::<ActiveProject>().settings_view.is_none();

        if needs_create {
            let runtime = cx.global::<ActiveProject>().runtime.clone();
            let sv = cx.new(|cx| workspace::settings::SettingsView::new(runtime, window, cx));
            cx.global_mut::<ActiveProject>().settings_view = Some(sv);
        }

        self.current_route = "/app/settings".to_string();
        use_navigate(cx)("/app/settings".into());
        window.refresh();
        cx.notify();
    }

    fn close_settings(&mut self, _: &CloseSettings, _window: &mut Window, cx: &mut Context<Self>) {
        if self.current_route == "/app/settings" {
            self.current_route = "/app".to_string();
            use_navigate(cx)("/app".into());
            cx.notify();
        }
    }

    fn show_keyboard_shortcuts(
        &mut self,
        _: &ShowKeyboardShortcuts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, |dialog, _window, _cx| {
            dialog
                .title("Keyboard Shortcuts")
                .w(px(420.))
                .close_button(true)
                .child(
                    v_flex()
                        .gap_1()
                        .child(shortcut_row("Open Project", "ctrl-o"))
                        .child(shortcut_row("Close Project", "ctrl-w"))
                        .child(shortcut_row("Reload Configuration", "ctrl-shift-r"))
                        .child(shortcut_row("Toggle Light/Dark Theme", "ctrl-shift-t"))
                        .child(shortcut_row("Settings", "ctrl-p"))
                        .child(shortcut_row("Keyboard Shortcuts", "f10"))
                        .child(shortcut_row("Quit Application", "ctrl-q")),
                )
        });
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
        // Process deferred actions that need Window access
        if let Some(path) = self.pending_project_path.take() {
            self.load_project(path, window, cx);
        }
        if self.pending_close_project {
            self.pending_close_project = false;
            tracing::info!("Closing current project");
            self.shutdown(cx);
            cx.remove_global::<ActiveProject>();
            self.current_config_path = None;
            self.current_route = "/".to_string();
            // Recreate loader so it gets fresh recent projects list
            self.loader = Workspace::create_loader(&self.nemo_config, window, cx);

            use_navigate(cx)("/".into());
            window.refresh();
            window.push_notification("Project closed", cx);
        }

        let bg_color = cx.theme().colors.background;
        let text_color = cx.theme().colors.foreground;

        // Use the persisted loader entity so event subscriptions remain valid
        let loader = self.loader.clone();

        let mut routes = Routes::new().child(Route::new().index().element(loader));

        // Add app routes if project is active — nested under AppLayout which
        // provides the shared header bar, with child routes for main and settings.
        if let Some(project) = cx.try_global::<ActiveProject>() {
            let app_entity = project.app_entity.clone();
            let header_bar = project.header_bar.clone();
            let footer_bar = project.footer_bar.clone();
            let settings_view = project.settings_view.clone();

            let mut app_children = vec![Route::new().index().element(app_entity)];

            if let Some(sv) = settings_view {
                app_children.push(Route::new().path("settings").element(sv));
            }

            routes = routes.child(
                Route::new()
                    .path("app")
                    .layout(AppLayout::new(header_bar, footer_bar))
                    .children(app_children),
            );
        }

        let mut container = v_flex()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::reload_config))
            .on_action(cx.listener(Self::quit_app))
            .on_action(cx.listener(Self::close_project))
            .on_action(cx.listener(Self::open_project))
            .on_action(cx.listener(Self::toggle_theme))
            .on_action(cx.listener(Self::show_keyboard_shortcuts))
            .on_action(cx.listener(Self::open_settings))
            .on_action(cx.listener(Self::close_settings))
            .child(routes);

        if let Some(dialog_layer) = Root::render_dialog_layer(window, cx) {
            container = container.child(dialog_layer);
        }
        if let Some(notification_layer) = Root::render_notification_layer(window, cx) {
            container = container.child(notification_layer);
        }

        container
    }
}

/// Render a single row for the keyboard shortcuts dialog.
fn shortcut_row(label: &str, keystroke: &str) -> impl IntoElement {
    let kbd = gpui_component::kbd::Kbd::new(Keystroke::parse(keystroke).unwrap());
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .py_1()
        .child(Label::new(label.to_string()))
        .child(kbd)
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
        router_init(cx);

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
                if let Some(config_path) = app_config_path {
                    info!("Loading project from: {:?}", config_path);

                    let mut recent_projects = config::recent::RecentProjects::load();
                    recent_projects.add(config_path.clone());
                    recent_projects.save();

                    match create_runtime(&config_path, &ws_args.extension_dirs) {
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
            use_navigate(cx)(route.into());

            *workspace_entity.borrow_mut() = Some(ws.clone());
            cx.new(|_cx| Root::new(ws, window, _cx))
        })
        .expect("Failed to open window");
    });

    info!("Nemo shutdown complete");
    Ok(())
}
