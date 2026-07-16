//! High-level layout **containers**.
//!
//! Containers are complex components that package a common application layout so
//! that application authors describe *intent* ("here is my nav, my content, my
//! status footer") rather than *layout mechanics* (nested stacks, fixed widths,
//! page-toggle handlers). This keeps developer effort focused on Rhai scripts
//! and plugins instead of hand-assembling primitives.
//!
//! The first container is [`AppShell`] — a standard app frame with a left
//! sidenav, a switchable content area, and a full-width footer.

mod app_shell;

pub use app_shell::AppShell;
