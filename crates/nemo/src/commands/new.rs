//! `nemo new` — scaffold a new project from a template.
//!
//! Scaffolding logic is implemented in Workstream B. This handler currently
//! validates arguments and reports that the command is not yet available.

use anyhow::{bail, Result};

use crate::args::NewArgs;

pub fn run(args: NewArgs) -> Result<()> {
    if args.list {
        // Workstream B will enumerate embedded templates here.
        bail!("`nemo new --list` is not yet implemented (planned in Workstream B)");
    }

    match args.name {
        Some(_) => bail!("`nemo new` is not yet implemented (planned in Workstream B)"),
        None => bail!("`nemo new` requires a project name, e.g. `nemo new my-app`"),
    }
}
