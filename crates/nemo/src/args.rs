use clap::Parser;
use std::path::PathBuf;
use tracing::Level;

/// Nemo - A configuration-driven application framework
#[derive(Parser, Debug)]
#[command(name = "nemo")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the project configuration file (app.xml).
    /// When not provided, shows the project loader screen.
    #[arg(long, env = "NEMO_APP_CONFIG")]
    pub app_config: Option<PathBuf>,

    /// Path to the TOML application config file (config.toml).
    /// Defaults to $XDG_CONFIG_HOME/nemo/config.toml if not provided.
    #[arg(short, long, env = "NEMO_CONFIG")]
    pub config: Option<PathBuf>,

    /// Additional configuration directories to scan
    #[arg(short = 'd', long)]
    pub app_config_dirs: Vec<PathBuf>,

    /// Extension/plugin directories
    #[arg(short, long, env = "NEMO_EXTENSION_DIRS", value_delimiter = ':')]
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

    /// Run in terminal UI mode (ratatui)
    #[arg(long, conflicts_with = "ui")]
    pub tui: bool,

    /// Run in desktop UI mode (GPUI) — this is the default
    #[arg(long, conflicts_with = "tui")]
    pub ui: bool,
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
