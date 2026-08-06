//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from XML files
//! - Initializes all subsystems
//! - Launches the GPUI window with router-based navigation

// In the binary's `--test` build the test harness replaces `main`, so all the
// GUI code only reachable from `fn main` (layout, action handlers, project
// globals, ...) looks dead to rustc 1.97+. The normal bin target (also checked
// by `--all-targets`) still enforces dead_code, so real dead code is caught.
#![cfg_attr(test, allow(dead_code))]

use anyhow::{Context as _, Result};
use gpui::*;
use gpui_component::Root;
use gpui_router::{init as router_init, use_navigate};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod app;
mod args;
mod commands;
mod components;
pub mod config;
mod containers;
mod project;
mod runtime;
mod theme;
mod window;
mod workspace;

use args::{Args, Command};
use config::NemoConfig;
use project::ActiveProject;
use window::get_window_options;
use workspace::actions::{
    CloseProject, CloseSettings, OpenProject, OpenSettings, QuitApp, ReloadConfig,
    ShowKeyboardShortcuts, ToggleDevPanel, ToggleTheme,
};
use workspace::utils::{apply_theme_from_runtime, create_runtime};
use workspace::{FooterBar, HeaderBar, Workspace, WorkspaceArgs};

/// Default debounce for `--watch` on the default run path (`nemo dev` sets its own).
const DEFAULT_WATCH_DEBOUNCE_MS: u64 = 200;

fn main() -> Result<()> {
    let mut args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level())
        .with_target(true)
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    // Take the command out so `args` stays whole for the arms that reuse it.
    match args.command.take() {
        Some(Command::New(new_args)) => commands::new::run(new_args),
        Some(Command::Dev(dev_args)) => commands::dev::run(args, dev_args),
        Some(Command::Validate(validate_args)) => commands::validate::run(validate_args),
        Some(Command::Schema(schema_args)) => commands::schema::run(schema_args),
        Some(Command::Build(build_args)) => commands::build::run(build_args),
        Some(Command::Get(get_args)) => commands::get::run(get_args),
        Some(Command::Screenshot(shot_args)) => dispatch_screenshot(shot_args),
        None => {
            let watch = args
                .watch
                .then(|| Duration::from_millis(DEFAULT_WATCH_DEBOUNCE_MS));
            run_app(args, watch, false)
        }
    }
}

/// Dispatches `nemo screenshot`. The command is always present in the CLI so it
/// shows in `--help` and gives an actionable error, but the capture path is only
/// compiled in under the `screenshot` feature (it needs gpui's offscreen render).
#[cfg(feature = "screenshot")]
fn dispatch_screenshot(args: args::ScreenshotArgs) -> Result<()> {
    commands::screenshot::run(args)
}

#[cfg(not(feature = "screenshot"))]
fn dispatch_screenshot(_args: args::ScreenshotArgs) -> Result<()> {
    anyhow::bail!(
        "the `screenshot` subcommand requires a build with the `screenshot` feature.\n\
         Rebuild with, e.g.:\n    \
         cargo run -p nemo --features screenshot -- screenshot --app-config <app.xml> --out <out.png>"
    )
}

/// Runs the Nemo application (the default, no-subcommand path).
///
/// Handles headless/validate modes when `--app-config` is provided, otherwise
/// launches the GPUI window with router-based navigation. When `watch` is
/// `Some(debounce)`, a file watcher hot-reloads the app on config changes.
pub(crate) fn run_app(mut args: Args, watch: Option<Duration>, dev_mode: bool) -> Result<()> {
    info!("Nemo v{} starting...", env!("CARGO_PKG_VERSION"));

    // Manifest-aware entry resolution: when `--app-config` is a directory or is
    // omitted, resolve the app entry via the nearest `nemo.toml`. An explicit
    // file path is used as-is, so existing invocations are untouched; when the
    // path is omitted and no manifest is in scope, `app_config` stays `None` and
    // the project-loader screen shows as before. With `--dist` (or the manifest's
    // `load = "dist"`), the resolved path is the built `dist/layout.json` instead.
    if let Some(resolved) = resolve_app_config_via_manifest(args.app_config.as_deref(), args.dist) {
        args.app_config = Some(resolved);
    }

    // Load NemoConfig (config.toml)
    let nemo_config = NemoConfig::load_from(args.config.as_ref());

    // Validate app_config path early so we fail fast with a clear message
    // before any subsystem (runtime, recent-projects, etc.) tries to use it.
    if let Some(ref app_config) = args.app_config {
        // For relative paths, resolution against the current directory is
        // implicit in Path::exists/is_file. If current_dir() fails we can
        // still check the path as supplied; either way the error message
        // shows the original path the user passed.
        let resolved = if app_config.is_relative() {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(app_config),
                Err(_) => app_config.clone(),
            }
        } else {
            app_config.clone()
        };

        if !resolved.exists() {
            eprintln!("Error: config file not found: {}", app_config.display());
            std::process::exit(1);
        }
        if !resolved.is_file() {
            eprintln!("Error: config path is not a file: {}", app_config.display());
            std::process::exit(1);
        }
    }

    // If app_config is provided via CLI/env, handle validate/headless modes.
    if let Some(ref app_config) = args.app_config {
        // `--validate-only` is a compatibility alias for `nemo validate`.
        if args.validate_only {
            return commands::validate::run(args::ValidateArgs {
                app_config: app_config.clone(),
                strict: false,
                format: args::ValidateFormat::Human,
            });
        }

        if args.headless {
            let rt = runtime::NemoRuntime::new(app_config)?;

            for dir in &args.extension_dirs {
                rt.add_extension_dir(dir)?;
            }

            info!("Loading configuration from: {:?}", app_config);
            rt.load_config()?;

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

    let params = BootstrapParams {
        nemo_config,
        app_config_path: args.app_config.clone(),
        ws_args: WorkspaceArgs {
            extension_dirs: args.extension_dirs.clone(),
            initial_route: args.route.clone(),
        },
        watch,
        size_override: None,
        dev_mode,
    };

    gpui_app.run(move |cx| {
        build_app_window(cx, params);
    });

    info!("Nemo shutdown complete");
    Ok(())
}

/// Resolves the effective config path (source `app.xml` or a built
/// `dist/layout.json`) via the project manifest.
///
/// * A path to an existing **file** is returned unchanged (today's behavior);
///   `force_dist` is ignored for an explicit file.
/// * A path to a **directory** resolves the nearest `nemo.toml` from it.
/// * A **nonexistent** path is returned unchanged, so the caller's own
///   not-found handling reports it.
/// * When **omitted**, walks up from the current directory for a `nemo.toml`;
///   returns `None` (leaving the loader screen) when none is found.
///
/// For the directory/omitted cases the result is `<root>/<manifest.entry>`
/// normally, or `<root>/<manifest.build.out>/layout.json` when `force_dist` or the
/// manifest's `load = "dist"` selects the built tree. Any manifest read/parse
/// error degrades gracefully to `None` on the run path; `nemo build` surfaces the
/// same errors loudly.
fn resolve_app_config_via_manifest(
    app_config: Option<&std::path::Path>,
    force_dist: bool,
) -> Option<PathBuf> {
    let start = match app_config {
        Some(p) if p.is_file() => return Some(p.to_path_buf()),
        Some(p) if p.is_dir() => p.to_path_buf(),
        Some(p) => return Some(p.to_path_buf()),
        None => std::env::current_dir().ok()?,
    };
    let root = nemo_config::find_project_root(&start)?;
    let manifest =
        nemo_config::ProjectManifest::load(&root.join(nemo_config::MANIFEST_FILE)).ok()?;
    let use_dist = force_dist || manifest.build.load == nemo_config::LoadMode::Dist;
    if use_dist {
        Some(
            root.join(&manifest.build.out)
                .join(nemo_config::DIST_LAYOUT_FILE),
        )
    } else {
        Some(root.join(manifest.entry))
    }
}

/// Parameters for building and opening the main application window.
///
/// Bundles everything the window bootstrap needs so both the normal run path
/// (`run_app`) and the `screenshot` subcommand construct an identical window.
pub(crate) struct BootstrapParams {
    pub nemo_config: NemoConfig,
    pub app_config_path: Option<PathBuf>,
    pub ws_args: WorkspaceArgs,
    /// Hot-reload debounce; `None` disables watching (screenshots never watch).
    pub watch: Option<Duration>,
    /// Force a fixed logical window size (WxH), overriding the config/maximized
    /// default. Used by `screenshot` for deterministic output dimensions.
    pub size_override: Option<(u32, u32)>,
    /// Whether the app was launched via `nemo dev` (enables the dev panel).
    pub dev_mode: bool,
}

/// Builds theme/fonts/runtime/workspace, opens the main window, and returns its
/// handle.
///
/// Must be called inside `gpui_platform::application().run(...)`. Behavior is
/// identical to the historical inline bootstrap in `run_app`; it only
/// additionally returns the window handle (previously discarded) and honors
/// `size_override`, so the `screenshot` subcommand can capture the same window
/// the normal run path produces.
pub(crate) fn build_app_window(cx: &mut App, params: BootstrapParams) -> WindowHandle<Root> {
    let BootstrapParams {
        nemo_config,
        app_config_path,
        ws_args,
        watch,
        size_override,
        dev_mode,
    } = params;

    gpui_component::init(cx);
    router_init(cx);

    // Apply theme from TOML config (base app settings)
    if nemo_config.app.theme_name != "default" {
        let mode = nemo_config.app.theme_mode.as_deref().unwrap_or("system");
        theme::apply_configured_theme(&nemo_config.app.theme_name, mode, None, cx);
    }

    // Apply global font family from TOML config
    if let Some(ref font_family) = nemo_config.app.font_family {
        gpui_component::Theme::global_mut(cx).font_family = font_family.clone().into();
    }

    // Apply global roundness from TOML config (after theme so it isn't reset)
    if let Some(ref roundness) = nemo_config.app.roundness {
        theme::apply_roundness(roundness, cx);
    }

    // Wrap config in Arc<Mutex<>> for sharing with settings view
    let nemo_config = Arc::new(Mutex::new(nemo_config));

    // Primary modifier: cmd on macOS (so native menu accelerators render as ⌘),
    // ctrl elsewhere (x11/wayland).
    #[cfg(target_os = "macos")]
    const PRIMARY: &str = "cmd";
    #[cfg(not(target_os = "macos"))]
    const PRIMARY: &str = "ctrl";

    cx.bind_keys([
        KeyBinding::new(&format!("{PRIMARY}-shift-r"), ReloadConfig, None),
        KeyBinding::new(&format!("{PRIMARY}-q"), QuitApp, None),
        KeyBinding::new(&format!("{PRIMARY}-w"), CloseProject, None),
        KeyBinding::new(&format!("{PRIMARY}-o"), OpenProject, None),
        KeyBinding::new(&format!("{PRIMARY}-shift-t"), ToggleTheme, None),
        KeyBinding::new(&format!("{PRIMARY}-p"), OpenSettings, None),
        KeyBinding::new("escape", CloseSettings, None),
        KeyBinding::new("f10", ShowKeyboardShortcuts, None),
        KeyBinding::new(&format!("{PRIMARY}-shift-e"), ToggleDevPanel, None),
    ]);

    // Store workspace entity + main window ID for window close handler.
    let workspace_entity: Rc<RefCell<Option<Entity<Workspace>>>> = Rc::new(RefCell::new(None));
    let main_window_id: Rc<RefCell<Option<WindowId>>> = Rc::new(RefCell::new(None));

    cx.on_window_closed({
        let workspace_entity = workspace_entity.clone();
        let main_window_id = main_window_id.clone();
        move |cx, window_id| {
            // If the main window closed, shut down and quit.
            if main_window_id.borrow().as_ref() == Some(&window_id) {
                if let Some(ws) = workspace_entity.borrow().clone() {
                    ws.update(cx, |ws, cx| {
                        ws.shutdown(cx);
                    });
                }
                cx.quit();
                return;
            }
            // Otherwise a non-main window closed (e.g. the dev panel builder
            // window). Clean up the workspace's dev panel handle and focus the
            // main window.
            if let Some(ws) = workspace_entity.borrow().clone() {
                // Find the main window handle and focus it.
                if let Some(main_id) = main_window_id.borrow().as_ref() {
                    if let Some(handle) =
                        cx.windows().into_iter().find(|h| h.window_id() == *main_id)
                    {
                        let _ = handle.update(cx, |_, window, _| {
                            window.activate_window();
                        });
                    }
                }
                ws.update(cx, |ws, cx| {
                    ws.handle_dev_panel_window_closed(cx);
                });
            }
        }
    })
    .detach();

    // If app_config provided, create runtime early so we can read window dimensions
    let early_runtime = app_config_path.as_ref().and_then(|config_path| {
        match create_runtime(
            config_path,
            &ws_args.extension_dirs,
            ws_args.initial_route.as_deref(),
        ) {
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

    // `--size` (screenshot) forces fixed, windowed dimensions for deterministic
    // output; otherwise use the config-derived size (or maximized default).
    let (win_w, win_h) = match size_override {
        Some((w, h)) => (Some(w), Some(h)),
        None => (win_w, win_h),
    };

    // Bring the app to the foreground and install the native menu bar before the
    // window opens. Without `activate` the process never takes focus (esp. when
    // launched from a terminal); without `set_menus` macOS shows no app menu.
    // `set_menus` reads the keymap bound above, so accelerators resolve here.
    let app_title = early_runtime
        .as_ref()
        .and_then(|rt| {
            rt.get_config("app.window.title")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "Nemo".to_string());
    cx.activate(true);
    cx.set_menus(workspace::menu::app_menus(app_title, dev_mode));

    let window_options = get_window_options(cx, win_w, win_h, win_min_w, win_min_h);

    let main_window_handle = cx
        .open_window(window_options, |window, cx| {
            let nemo_config = nemo_config.clone();
            let ws_args = ws_args.clone();
            let app_config_path = app_config_path.clone();
            // Preserved for the workspace's `current_config_path`, since the binding
            // below is consumed while creating the runtime.
            let current_cfg_for_ws = app_config_path.clone();
            // Captured for hot-reload watching before the values below are moved
            // into the workspace constructor.
            let watch_cfg = app_config_path.clone();
            let watch_exts = ws_args.extension_dirs.clone();

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
                        let menu_items = workspace::menu_items_from_config(&rt);
                        let header_bar = cx.new(|_cx| {
                            HeaderBar::new(
                                title,
                                github_url,
                                theme_toggle,
                                menu_items,
                                Arc::clone(&rt),
                                dev_mode,
                            )
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
                        current_cfg_for_ws
                    } else {
                        None
                    },
                    pending_project_path: None,
                    pending_close_project: false,
                    focus_handle,
                    current_route,
                    loader,
                    pending_reload: false,
                    _watcher: None,
                    dev_mode,
                    dev_panel_window: None,
                }
            });

            // Navigate to the initial route after window creation
            let route = ws.read(cx).current_route.clone();
            let needs_refresh = route != "/";
            use_navigate(cx)(route.into());
            if needs_refresh {
                window.refresh();
            }

            // Start hot-reload file watching when requested (nemo dev / --watch).
            if let Some(debounce) = watch {
                if let Some(cfg) = watch_cfg.as_ref() {
                    let mut watch_paths: Vec<PathBuf> = Vec::new();
                    match cfg.parent() {
                        Some(parent) if !parent.as_os_str().is_empty() => {
                            watch_paths.push(parent.to_path_buf());
                        }
                        _ => watch_paths.push(PathBuf::from(".")),
                    }
                    watch_paths.extend(watch_exts.iter().cloned());
                    ws.update(cx, |ws, cx| ws.start_watching(watch_paths, debounce, cx));
                }
            }

            *workspace_entity.borrow_mut() = Some(ws.clone());
            cx.new(|_cx| Root::new(ws, window, _cx))
        })
        .expect("Failed to open window");

    *main_window_id.borrow_mut() = Some(AnyWindowHandle::from(main_window_handle).window_id());
    main_window_handle
}

#[cfg(test)]
mod manifest_launch_tests {
    // Import specific items (not `use super::*`) — a glob import in a nemo-bin
    // test module blows the macro-recursion limit.
    use super::resolve_app_config_via_manifest;
    use std::path::PathBuf;

    #[test]
    fn directory_resolves_to_manifest_entry() {
        let dir = std::env::temp_dir().join(format!("nemo_launch_dir_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nemo.toml"),
            "name = \"t\"\nentry = \"main.xml\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("main.xml"), "<nemo/>").unwrap();

        let resolved = resolve_app_config_via_manifest(Some(&dir), false);
        assert_eq!(resolved, Some(dir.join("main.xml")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn explicit_file_is_returned_unchanged() {
        let dir = std::env::temp_dir().join(format!("nemo_launch_file_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("app.xml");
        std::fs::write(&file, "<nemo/>").unwrap();

        assert_eq!(
            resolve_app_config_via_manifest(Some(&file), false),
            Some(file.clone())
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonexistent_path_is_returned_unchanged() {
        let p = PathBuf::from("/no/such/path/app.xml");
        assert_eq!(resolve_app_config_via_manifest(Some(&p), false), Some(p));
    }

    #[test]
    fn dist_flag_resolves_to_built_layout() {
        let dir = std::env::temp_dir().join(format!("nemo_launch_dist_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nemo.toml"),
            "name = \"t\"\nentry = \"app.xml\"\n[build]\nout = \"dist\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("app.xml"), "<nemo/>").unwrap();

        // --dist selects the built tree; source is unaffected without it.
        assert_eq!(
            resolve_app_config_via_manifest(Some(&dir), true),
            Some(dir.join("dist").join("layout.json"))
        );
        assert_eq!(
            resolve_app_config_via_manifest(Some(&dir), false),
            Some(dir.join("app.xml"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manifest_load_dist_selects_built_layout() {
        let dir = std::env::temp_dir().join(format!("nemo_launch_loaddist_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nemo.toml"),
            "name = \"t\"\n[build]\nload = \"dist\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("app.xml"), "<nemo/>").unwrap();

        // The manifest opts into dist even without the flag.
        assert_eq!(
            resolve_app_config_via_manifest(Some(&dir), false),
            Some(dir.join("dist").join("layout.json"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
