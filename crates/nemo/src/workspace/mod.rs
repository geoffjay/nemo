use gpui::*;
use gpui_component::label::Label;
use gpui_component::notification::{Notification as Toast, NotificationType};
use gpui_component::v_flex;
use gpui_component::ActiveTheme;
use gpui_component::Root;
use gpui_component::WindowExt as _;
use gpui_router::{use_navigate, Route, Routes};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;

pub mod actions;
mod dev_panel;
mod footer_bar;
mod header_bar;
pub mod layout;
pub mod main_view;
pub mod project_loader;
pub mod settings;
pub mod utils;
pub mod xml_edit;
use actions::{
    CloseProject, CloseSettings, OpenProject, OpenSettings, QuitApp, ReloadConfig,
    ShowKeyboardShortcuts, ToggleDevPanel, ToggleTheme,
};
pub use footer_bar::FooterBar;
pub use header_bar::{menu_items_from_config, HeaderBar};
use layout::AppLayout;
use project_loader::{ProjectLoaderView, ProjectSelected};
use settings::SettingsView;
use utils::{apply_theme_from_runtime, create_runtime, shortcut_row};

use crate::app;
use crate::config;
use crate::config::NemoConfig;
use crate::project::ActiveProject;
use crate::runtime;
use crate::theme;
use crate::theme::tokens::{Space, TokenStyled};

/// Subset of args needed after initial parse.
#[derive(Clone)]
pub struct WorkspaceArgs {
    pub extension_dirs: Vec<PathBuf>,
    /// Launch-time `<router>` starting-path override (`--route`). Applies only
    /// to the CLI-launched app, not projects later opened via the loader.
    pub initial_route: Option<String>,
}

/// The root workspace entity that manages the application state.
#[allow(dead_code)]
pub struct Workspace {
    pub nemo_config: Arc<Mutex<NemoConfig>>,
    pub ws_args: WorkspaceArgs,
    pub current_config_path: Option<PathBuf>,
    pub pending_project_path: Option<PathBuf>,
    pub pending_close_project: bool,
    pub focus_handle: FocusHandle,
    /// Current route path for the router.
    pub current_route: String,
    /// The project loader view entity (persists across renders).
    pub loader: Entity<ProjectLoaderView>,
    /// Deferred hot-reload request, set by the `nemo dev` file watcher and
    /// processed in `render` (where `Window` access is available).
    pub pending_reload: bool,
    /// File watcher kept alive for the app's lifetime; not read directly.
    pub(crate) _watcher: Option<notify::RecommendedWatcher>,
    /// Whether the app was launched via `nemo dev` (enables the dev panel).
    pub dev_mode: bool,
    pub dev_panel_window: Option<WindowHandle<gpui_component::Root>>,
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

        // A project opened via the loader is a different app than the one the
        // CLI `--route` targeted, so don't apply the launch override here.
        match create_runtime(&app_config_path, &self.ws_args.extension_dirs, None) {
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
        let menu_items = menu_items_from_config(runtime);
        let runtime = Arc::clone(runtime);
        cx.new(|cx| {
            HeaderBar::new(
                title,
                github_url,
                theme_toggle,
                menu_items,
                runtime,
                self.dev_mode,
                window,
                cx,
            )
        })
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
    pub fn shutdown(&self, cx: &mut Context<'_, Self>) {
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
        self.perform_reload(window, cx);
    }

    /// Rebuild the entire app from the current config path. Shared by the
    /// `ReloadConfig` action (ctrl-shift-r) and the `nemo dev` file watcher.
    fn perform_reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config_path) = self.current_config_path.clone() else {
            return;
        };

        tracing::info!("Reloading configuration from: {:?}", config_path);

        // Hot-reload recreates the runtime (router state resets to defaults);
        // `--route` is a launch-time override, not reapplied on reload.
        match create_runtime(&config_path, &self.ws_args.extension_dirs, None) {
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

    /// Watch `paths` and request a hot-reload when a relevant file (`.xml`,
    /// `.rhai`, `.toml`) changes. Directories are watched recursively. Used by
    /// `nemo dev` and the `--watch` flag.
    pub fn start_watching(
        &mut self,
        paths: Vec<PathBuf>,
        debounce: Duration,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        for path in &paths {
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            match watcher.watch(path, mode) {
                Ok(()) => tracing::info!("Watching {:?} for changes", path),
                Err(e) => tracing::warn!("Failed to watch {:?}: {}", path, e),
            }
        }
        // Keep the watcher alive for the lifetime of the workspace.
        self._watcher = Some(watcher);

        let poll = Duration::from_millis(120);
        cx.spawn(
            async move |this: WeakEntity<Self>, cx: &mut AsyncApp| loop {
                cx.background_executor().timer(poll).await;

                // Collect events since the last poll; decide whether to reload.
                let mut changed = false;
                while let Ok(res) = rx.try_recv() {
                    if let Ok(event) = res {
                        if reload_relevant(&event) {
                            changed = true;
                        }
                    }
                }
                if !changed {
                    continue;
                }

                // Let a burst of edits settle, then drain anything that arrived.
                cx.background_executor().timer(debounce).await;
                while rx.try_recv().is_ok() {}

                let alive = this
                    .update(cx, |ws, cx| {
                        ws.pending_reload = true;
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            },
        )
        .detach();
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
            prompt: Some("Select an app.xml configuration file".into()),
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
            let nemo_config = Arc::clone(&self.nemo_config);
            let sv = cx.new(|cx| SettingsView::new(runtime, nemo_config, window, cx));
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

    fn toggle_dev_panel(
        &mut self,
        _: &ToggleDevPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dev_mode {
            return;
        }
        self.process_dev_panel_toggle(window, cx);
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
                        .gap_t(Space::Xs)
                        .child(shortcut_row("Open Project", "ctrl-o"))
                        .child(shortcut_row("Close Project", "ctrl-w"))
                        .child(shortcut_row("Reload Configuration", "ctrl-shift-r"))
                        .child(shortcut_row("Toggle Light/Dark Theme", "ctrl-shift-t"))
                        .child(shortcut_row("Settings", "ctrl-p"))
                        .child(shortcut_row("Keyboard Shortcuts", "f10"))
                        .child(shortcut_row("Toggle Dev Panel (dev mode)", "ctrl-shift-e"))
                        .child(shortcut_row("Quit Application", "ctrl-q")),
                )
        });
    }

    /// Toggles the dev panel builder window. When no builder window is open,
    /// opens one. When the builder window is already open, focuses it.
    /// Only acts in dev mode with a loaded project.
    fn process_dev_panel_toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dev_mode {
            return;
        }

        // If the builder window is already open, focus it.
        if let Some(handle) = self.dev_panel_window.as_ref() {
            let _ = handle.update(cx, |_, window, _| {
                window.activate_window();
            });
            return;
        }

        // Need a project root for the file tree.
        let root = self
            .current_config_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());
        let Some(project_root) = root else {
            window.push_notification("Open a project first", cx);
            return;
        };
        let runtime = match cx.try_global::<ActiveProject>() {
            Some(p) => Arc::clone(&p.runtime),
            None => return,
        };
        let panel = cx.new(|cx| dev_panel::DevPanel::new(project_root, runtime, window, cx));

        // Open the builder in a separate window.
        let dev_window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.), px(650.)),
                cx,
            ))),
            titlebar: Some(gpui_component::TitleBar::title_bar_options()),
            window_decorations: Some(WindowDecorations::Client),
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            window_min_size: Some(size(px(500.), px(400.))),
            ..Default::default()
        };

        match cx.open_window(dev_window_options, |window, cx| {
            cx.new(|cx| gpui_component::Root::new(panel.clone(), window, cx))
        }) {
            Ok(handle) => {
                self.dev_panel_window = Some(handle);
            }
            Err(e) => {
                tracing::error!("Failed to open dev panel window: {}", e);
                window.push_notification(
                    Toast::new()
                        .message(format!("Failed to open dev panel: {e}"))
                        .with_type(NotificationType::Error),
                    cx,
                );
            }
        }
        cx.notify();
    }

    /// Called when the dev panel window is closed (e.g. by the user clicking the
    /// window's close button). Cleans up the handle and focuses the main window.
    pub fn handle_dev_panel_window_closed(&mut self, cx: &mut Context<Self>) {
        self.dev_panel_window = None;
        cx.notify();
    }

    /// Create a ProjectLoaderView and subscribe to its events.
    pub fn create_loader(
        nemo_config: &Arc<Mutex<NemoConfig>>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Entity<ProjectLoaderView> {
        let config_snapshot = nemo_config.lock().unwrap().clone();
        let loader = cx.new(|cx| ProjectLoaderView::new(config_snapshot, window, cx));
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

/// Whether a filesystem event should trigger a hot-reload.
fn reload_relevant(event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }
    event.paths.iter().any(|path| path_is_watchable(path))
}

/// Whether a changed path is one we care about: a `.xml`/`.rhai`/`.toml` source
/// file that is not an editor temp/hidden file or under a build/VCS directory.
fn path_is_watchable(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    // Skip editor temp files and hidden files.
    if name.is_empty() || name.starts_with('.') || name.ends_with('~') {
        return false;
    }
    // Skip build artifacts and VCS directories.
    if path.components().any(|c| {
        matches!(
            c.as_os_str().to_str(),
            Some("target") | Some(".git") | Some("node_modules")
        )
    }) {
        return false;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("xml") | Some("rhai") | Some("toml")
    )
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
        if self.pending_reload {
            self.pending_reload = false;
            self.perform_reload(window, cx);
        }

        let bg_color = cx.theme().colors.background;
        let text_color = cx.theme().colors.foreground;

        // Use the persisted loader entity so event subscriptions remain valid
        let loader = self.loader.clone();

        let mut routes = Routes::new().child(
            Route::new()
                .index()
                .element(move |_, _| AnyView::from(loader.clone())),
        );

        // Add app routes if project is active — nested under AppLayout which
        // provides the shared header bar, with child routes for main and settings.
        if let Some(project) = cx.try_global::<ActiveProject>() {
            let app_entity = project.app_entity.clone();
            let header_bar = project.header_bar.clone();
            let footer_bar = project.footer_bar.clone();
            let settings_view = project.settings_view.clone();

            let ae = app_entity.clone();
            let mut app_children = vec![Route::new()
                .index()
                .element(move |_, _| AnyView::from(ae.clone()))];

            if let Some(sv) = settings_view {
                app_children.push(
                    Route::new()
                        .path("settings")
                        .element(move |_, _| AnyView::from(sv.clone())),
                );
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
            .on_action(cx.listener(Self::toggle_dev_panel))
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

#[cfg(test)]
mod watch_tests {
    use super::path_is_watchable;
    use std::path::Path;

    #[test]
    fn watches_config_and_script_sources() {
        assert!(path_is_watchable(Path::new("/proj/app.xml")));
        assert!(path_is_watchable(Path::new("/proj/scripts/handlers.rhai")));
        assert!(path_is_watchable(Path::new("/proj/config.toml")));
        assert!(path_is_watchable(Path::new("app.xml")));
    }

    #[test]
    fn ignores_irrelevant_extensions() {
        assert!(!path_is_watchable(Path::new("/proj/README.md")));
        assert!(!path_is_watchable(Path::new("/proj/data.json")));
        assert!(!path_is_watchable(Path::new("/proj/noext")));
    }

    #[test]
    fn ignores_editor_temp_and_hidden_files() {
        assert!(!path_is_watchable(Path::new("/proj/app.xml~")));
        assert!(!path_is_watchable(Path::new("/proj/.app.xml.swp")));
        assert!(!path_is_watchable(Path::new("/proj/.hidden.xml")));
    }

    #[test]
    fn ignores_build_and_vcs_dirs() {
        assert!(!path_is_watchable(Path::new("/proj/target/debug/app.xml")));
        assert!(!path_is_watchable(Path::new("/proj/.git/config.toml")));
        assert!(!path_is_watchable(Path::new(
            "/proj/node_modules/pkg/thing.xml"
        )));
    }
}
