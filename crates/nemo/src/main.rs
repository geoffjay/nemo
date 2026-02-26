//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from XML files
//! - Initializes all subsystems
//! - Dispatches to the appropriate UI backend (GPUI or TUI)

use anyhow::{Context as _, Result};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod args;

use args::Args;
use nemo_ui::config::NemoConfig;
use nemo_ui::runtime;

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

    // Determine UI mode: CLI flag > XML config default > fallback to GPUI
    let use_tui = if args.tui {
        true
    } else if args.ui {
        false
    } else {
        // Check XML config for <app default="tui">
        args.app_config
            .as_ref()
            .and_then(|config_path| {
                let rt = runtime::NemoRuntime::new(config_path).ok()?;
                rt.load_config().ok()?;
                rt.get_config("app.default")
                    .and_then(|v| v.as_str().map(|s| s == "tui"))
            })
            .unwrap_or(false)
    };

    if use_tui {
        let app_config = args.app_config.as_ref().ok_or_else(|| {
            anyhow::anyhow!("TUI mode requires --app-config to be specified")
        })?;
        let rt = nemo_ui::workspace::utils::create_runtime(app_config, &args.extension_dirs)?;
        nemo_tui::run_tui(rt)
    } else {
        nemo_ui::run_gpui(nemo_config, args.app_config.clone(), args.extension_dirs.clone())
    }
}
