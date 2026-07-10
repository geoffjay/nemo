//! `nemo dev` — run an application with hot-reload on configuration changes.
//!
//! The watcher/reload wiring is implemented in Workstream C. The full-rebuild
//! path it will drive already exists (`workspace::Workspace::reload_config`,
//! dispatched today by the `ReloadConfig` action / `ctrl-shift-r`).

use anyhow::{bail, Result};

use crate::args::DevArgs;

pub fn run(_args: DevArgs) -> Result<()> {
    bail!("`nemo dev` is not yet implemented (planned in Workstream C)");
}
