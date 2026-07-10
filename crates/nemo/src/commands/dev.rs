//! `nemo dev` — run an application with hot-reload on configuration changes.
//!
//! Reuses the standard launch path (`crate::run_app`) with watching enabled.
//! The full-rebuild it drives already exists
//! (`workspace::Workspace::perform_reload`, also bound to `ctrl-shift-r`).

use std::time::Duration;

use anyhow::{bail, Result};

use crate::args::{Args, DevArgs};

pub fn run(mut args: Args, dev: DevArgs) -> Result<()> {
    // Dev-specific flags take precedence over the global equivalents.
    if dev.app_config.is_some() {
        args.app_config = dev.app_config;
    }
    if dev.config.is_some() {
        args.config = dev.config;
    }
    if !dev.extension_dirs.is_empty() {
        args.extension_dirs = dev.extension_dirs;
    }

    if args.app_config.is_none() {
        bail!("`nemo dev` requires --app-config <app.xml> (or NEMO_APP_CONFIG)");
    }

    // Dev mode always runs the UI with watching enabled.
    args.headless = false;
    args.validate_only = false;

    crate::run_app(args, Some(Duration::from_millis(dev.debounce_ms)))
}
