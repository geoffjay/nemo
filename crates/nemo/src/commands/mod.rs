//! Nemo CLI subcommand handlers.
//!
//! Each submodule implements one `nemo <command>` handler. The default
//! (no-subcommand) run path lives in `main::run_app`.
//!
//! Handlers here are scaffolded as part of Workstream A (CLI subcommand
//! architecture). The concrete implementations land in later workstreams:
//! - `new`      — Workstream B
//! - `dev`      — Workstream C
//! - `validate` — Workstream D

pub mod dev;
pub mod new;
pub mod validate;
