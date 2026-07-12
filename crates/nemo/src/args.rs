use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::Level;

/// Nemo - A configuration-driven application framework
#[derive(Parser, Debug)]
#[command(name = "nemo")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Subcommand to run. When omitted, Nemo launches the application
    /// (or the project loader when no `--app-config` is given).
    #[command(subcommand)]
    pub command: Option<Command>,

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
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Run in headless mode (no UI)
    #[arg(long)]
    pub headless: bool,

    /// Watch the config directory and hot-reload on changes.
    /// Equivalent to running `nemo dev` for the current app.
    #[arg(long)]
    pub watch: bool,

    /// Validate configuration and exit.
    ///
    /// Deprecated: prefer `nemo validate <app.xml>`. Kept for backward
    /// compatibility with the default (no-subcommand) run path.
    #[arg(long)]
    pub validate_only: bool,
}

/// Nemo subcommands.
///
/// The subcommand handlers live in `crate::commands`. When no subcommand is
/// given, `main` runs the application directly (the historical behavior).
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new Nemo project from a template.
    New(NewArgs),
    /// Run an application with hot-reload on configuration changes.
    Dev(DevArgs),
    /// Validate a configuration file and exit.
    Validate(ValidateArgs),
}

/// Arguments for `nemo new`.
#[derive(clap::Args, Debug)]
pub struct NewArgs {
    /// Directory to create for the new project (also used as the app name).
    /// Optional so `--list` can be used without a target.
    pub name: Option<PathBuf>,

    /// Template to scaffold from (e.g. basic, data-binding, calculator, complete).
    #[arg(short, long, default_value = "basic")]
    pub template: String,

    /// Overwrite the target directory even if it is not empty.
    #[arg(long)]
    pub force: bool,

    /// List the available templates and exit.
    #[arg(long)]
    pub list: bool,
}

/// Arguments for `nemo dev`.
#[derive(clap::Args, Debug)]
pub struct DevArgs {
    /// Path to the project configuration file (app.xml).
    #[arg(long, env = "NEMO_APP_CONFIG")]
    pub app_config: Option<PathBuf>,

    /// Path to the TOML application config file (config.toml).
    #[arg(short, long, env = "NEMO_CONFIG")]
    pub config: Option<PathBuf>,

    /// Extension/plugin directories
    #[arg(short, long, env = "NEMO_EXTENSION_DIRS", value_delimiter = ':')]
    pub extension_dirs: Vec<PathBuf>,

    /// Debounce window for coalescing rapid file changes, in milliseconds.
    #[arg(long, default_value_t = 200)]
    pub debounce_ms: u64,
}

/// Arguments for `nemo validate`.
#[derive(clap::Args, Debug)]
pub struct ValidateArgs {
    /// Path to the configuration file to validate.
    pub app_config: PathBuf,

    /// Enable stricter lints (deprecated properties, missing ids, unused templates).
    #[arg(long)]
    pub strict: bool,

    /// Output format for diagnostics.
    #[arg(long, value_enum, default_value = "human")]
    pub format: ValidateFormat,
}

/// Output format for `nemo validate`.
#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum ValidateFormat {
    /// Human-readable diagnostics.
    Human,
    /// Machine-readable JSON diagnostics.
    Json,
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

#[cfg(test)]
mod tests {
    use super::*;

    // The no-subcommand default path must keep working exactly as before.
    #[test]
    fn bare_run_has_no_subcommand() {
        let args = Args::try_parse_from(["nemo", "--app-config", "app.xml"]).unwrap();
        assert!(args.command.is_none());
        assert_eq!(args.app_config, Some(PathBuf::from("app.xml")));
    }

    #[test]
    fn bare_run_with_no_args_is_valid() {
        let args = Args::try_parse_from(["nemo"]).unwrap();
        assert!(args.command.is_none());
        assert!(args.app_config.is_none());
    }

    #[test]
    fn legacy_flags_still_parse() {
        let args = Args::try_parse_from([
            "nemo",
            "--app-config",
            "app.xml",
            "--headless",
            "--validate-only",
            "--verbose",
        ])
        .unwrap();
        assert!(args.command.is_none());
        assert!(args.headless);
        assert!(args.validate_only);
        assert!(args.verbose);
    }

    #[test]
    fn new_subcommand_parses() {
        let args =
            Args::try_parse_from(["nemo", "new", "my-app", "--template", "calculator"]).unwrap();
        match args.command {
            Some(Command::New(new)) => {
                assert_eq!(new.name, Some(PathBuf::from("my-app")));
                assert_eq!(new.template, "calculator");
            }
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn new_list_needs_no_name() {
        let args = Args::try_parse_from(["nemo", "new", "--list"]).unwrap();
        match args.command {
            Some(Command::New(new)) => {
                assert!(new.list);
                assert!(new.name.is_none());
            }
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn dev_subcommand_parses() {
        let args = Args::try_parse_from([
            "nemo",
            "dev",
            "--app-config",
            "app.xml",
            "--debounce-ms",
            "50",
        ])
        .unwrap();
        match args.command {
            Some(Command::Dev(dev)) => {
                assert_eq!(dev.app_config, Some(PathBuf::from("app.xml")));
                assert_eq!(dev.debounce_ms, 50);
            }
            other => panic!("expected Dev, got {other:?}"),
        }
    }

    #[test]
    fn validate_subcommand_parses() {
        let args = Args::try_parse_from([
            "nemo", "validate", "app.xml", "--strict", "--format", "json",
        ])
        .unwrap();
        match args.command {
            Some(Command::Validate(v)) => {
                assert_eq!(v.app_config, PathBuf::from("app.xml"));
                assert!(v.strict);
                assert_eq!(v.format, ValidateFormat::Json);
            }
            other => panic!("expected Validate, got {other:?}"),
        }
    }

    #[test]
    fn watch_flag_parses_on_default_path() {
        let args = Args::try_parse_from(["nemo", "--app-config", "app.xml", "--watch"]).unwrap();
        assert!(args.command.is_none());
        assert!(args.watch);
    }

    #[test]
    fn verbose_is_global_after_subcommand() {
        let args = Args::try_parse_from(["nemo", "validate", "app.xml", "--verbose"]).unwrap();
        assert!(args.verbose);
    }
}
