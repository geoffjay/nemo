//! Nemo TUI - ratatui terminal rendering backend.
//!
//! This crate renders Nemo configurations as terminal user interfaces
//! using ratatui and crossterm.

mod app;
pub mod components;
pub mod renderer;

use anyhow::Result;
use nemo_ui::runtime::NemoRuntime;
use std::sync::Arc;
use tracing::info;

/// Run the TUI application.
///
/// This is the main entry point for the terminal UI backend. It initializes
/// the terminal, enters the ratatui event loop, and restores the terminal on exit.
pub fn run_tui(runtime: Arc<NemoRuntime>) -> Result<()> {
    info!("Starting TUI application...");
    let mut app = app::TuiApp::new(runtime);
    let result = app.run();
    info!("TUI shutdown complete");
    result
}
