use clap::Parser;
use std::path::PathBuf;
use tracing::Level;

/// Nemo - A configuration-driven application framework
#[derive(Parser, Debug)]
#[command(name = "nemo")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the main configuration file
    #[arg(short, long, default_value = "app.hcl")]
    pub config: PathBuf,

    /// Additional configuration directories to scan
    #[arg(short = 'd', long)]
    pub config_dirs: Vec<PathBuf>,

    /// Extension/plugin directories
    #[arg(short, long)]
    pub extension_dirs: Vec<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Run in headless mode (no UI)
    #[arg(long)]
    pub headless: bool,

    /// Validate configuration and exit
    #[arg(long)]
    pub validate_only: bool,
}

impl Args {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args())
    }

    pub fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
}
