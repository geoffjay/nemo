//! Nemo WASM Guest SDK.
//!
//! Plugin authors depend on this crate for access to `wit-bindgen` and the
//! documentation on how to write Nemo WASM plugins.
//!
//! Each plugin crate must invoke `wit_bindgen::generate!` pointing at the
//! `nemo-plugin.wit` file (shipped in this crate's `wit/` directory), implement
//! the generated `Guest` trait, and call `export!(MyPlugin)`.
//!
//! # Example
//!
//! ```ignore
//! wit_bindgen::generate!({
//!     path: "wit/nemo-plugin.wit",
//!     world: "nemo-plugin",
//! });
//!
//! use nemo::plugin::host_api;
//! use nemo::plugin::types::{LogLevel, PluginValue};
//!
//! struct MyPlugin;
//!
//! impl Guest for MyPlugin {
//!     fn get_manifest() -> PluginManifest {
//!         PluginManifest {
//!             id: "my-plugin".into(),
//!             name: "My Plugin".into(),
//!             version: "0.1.0".into(),
//!             description: "A sample plugin".into(),
//!             author: Some("Author".into()),
//!         }
//!     }
//!
//!     fn init() {
//!         host_api::log(LogLevel::Info, "Plugin initialized");
//!     }
//!
//!     fn tick() -> u64 {
//!         0
//!     }
//! }
//!
//! export!(MyPlugin);
//! ```

// Re-export wit-bindgen for plugin authors.
pub use wit_bindgen;
