//! `nemo validate` — validate a configuration file and exit.
//!
//! Strict-mode lints and JSON output are implemented in Workstream D. The
//! basic parse/validate path already exists on the runtime
//! (`NemoRuntime::load_config`); Workstream D will wrap it here with proper
//! exit codes and diagnostics rendering.

use anyhow::{bail, Result};

use crate::args::ValidateArgs;

pub fn run(_args: ValidateArgs) -> Result<()> {
    bail!("`nemo validate` is not yet implemented (planned in Workstream D)");
}
