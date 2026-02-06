//! Nemo Application Shell - Main binary.
//!
//! This is the main entry point for Nemo applications. It:
//! - Parses CLI arguments
//! - Loads configuration from HCL files
//! - Initializes all subsystems
//! - Launches the GPUI window

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod app;
mod runtime;

use app::NemoApp;

/// Nemo - A configuration-driven application framework
#[derive(Parser, Debug)]
#[command(name = "nemo")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the main configuration file
    #[arg(short, long, default_value = "app.hcl")]
    config: PathBuf,

    /// Additional configuration directories to scan
    #[arg(short = 'd', long)]
    config_dirs: Vec<PathBuf>,

    /// Extension/plugin directories
    #[arg(short, long)]
    extension_dirs: Vec<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run in headless mode (no UI)
    #[arg(long)]
    headless: bool,

    /// Validate configuration and exit
    #[arg(long)]
    validate_only: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    info!("Nemo v{} starting...", env!("CARGO_PKG_VERSION"));

    // Create the runtime
    let runtime = runtime::NemoRuntime::new(&args.config)?;

    // Add additional config directories
    for dir in &args.config_dirs {
        runtime.add_config_dir(dir)?;
    }

    // Add extension directories
    for dir in &args.extension_dirs {
        runtime.add_extension_dir(dir)?;
    }

    // Load and validate configuration
    info!("Loading configuration from: {:?}", args.config);
    runtime.load_config()?;

    if args.validate_only {
        info!("Configuration validation successful");
        return Ok(());
    }

    // Initialize all subsystems
    info!("Initializing subsystems...");
    runtime.initialize()?;

    if args.headless {
        info!("Running in headless mode");
        runtime.run_headless()?;
    } else {
        info!("Starting GPUI application...");
        NemoApp::run(runtime)?;
    }

    info!("Nemo shutdown complete");
    Ok(())
}
