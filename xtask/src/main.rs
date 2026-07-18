//! Development-only tasks for the nemo workspace, run via `cargo xtask <task>`.
//!
//! Tasks here are for developers, not end users, so they live outside the
//! shipped `nemo` binary. Today the only task is `design-export`.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod design_export;

#[derive(Parser)]
#[command(name = "xtask", about = "Nemo development tasks (not shipped)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export the nemo design system (tokens + themes + component structure) as
    /// a `.pen`-friendly JSON intermediate.
    DesignExport(DesignExportArgs),
}

#[derive(clap::Args)]
pub struct DesignExportArgs {
    /// Write the export to this file instead of stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Emit compact (single-line) JSON instead of pretty-printed.
    #[arg(long)]
    pub compact: bool,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::DesignExport(args) => design_export::run(args),
    }
}
