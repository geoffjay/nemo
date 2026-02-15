//! Nemo Plugin Development Kit
//!
//! This crate provides convenient builders and utilities for developing Nemo plugins.
//! It builds on top of `nemo-plugin-api` to offer a fluent, type-safe API for creating
//! UI layouts, components, and templates.
//!
//! # Example
//!
//! ```rust
//! use nemo_plugin::prelude::*;
//!
//! // Build a simple UI layout
//! let layout = Panel::new()
//!     .padding(16)
//!     .border(2)
//!     .width(300)
//!     .child("title", Label::new("My Plugin").size("xl"))
//!     .child("input", Input::new()
//!         .value("default")
//!         .on_change("on_input_change"))
//!     .build();
//! ```

pub mod builder;
pub mod components;
pub mod containers;
pub mod value;

/// Re-export nemo-plugin-api for convenience
pub use nemo_plugin_api::*;

/// Prelude module containing commonly used types and traits
pub mod prelude {
    pub use crate::builder::*;
    pub use crate::components::*;
    pub use crate::containers::*;
    pub use crate::value::*;
    pub use nemo_plugin_api::*;
}
