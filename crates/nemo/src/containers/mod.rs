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
//!
//! [`Router`] is a lower-level, chrome-free switching primitive: it renders one
//! of its `<route>` children by URL-style path, with history, params, and
//! lifecycle hooks. See [`router`].

mod app_shell;
pub mod router;

pub use app_shell::AppShell;
pub use router::{NavLink, Router};
