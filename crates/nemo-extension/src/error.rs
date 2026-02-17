//! Extension error types.
//!
//! # Recovery Classification
//!
//! Extension errors generally fall into two categories:
//! - **Configuration/logic errors** (fatal): require fixing the extension
//!   manifest, permissions, or plugin binary.
//! - **Runtime errors** (transient): script execution failures or IO errors
//!   that may succeed on retry.

use thiserror::Error;

/// Errors that can occur in the extension system.
///
/// # Recovery
///
/// | Variant          | Recoverable? | Notes                                       |
/// |-----------------|-------------|----------------------------------------------|
/// | `NotFound`      | Fatal       | Extension ID typo or never loaded            |
/// | `LoadError`     | Fatal       | Bad path, missing symbols, ABI mismatch      |
/// | `UnloadError`   | Fatal       | Internal error during unload                 |
/// | `ScriptError`   | Transient   | Script bug; may succeed after edit/reload    |
/// | `PluginInitError`| Fatal      | Plugin entry function failed                 |
/// | `InvalidManifest`| Fatal      | Manifest parsing failed; fix plugin          |
/// | `PermissionDenied`| Fatal     | Plugin lacks required permission             |
/// | `AlreadyLoaded` | Fatal       | Logic error — check before loading           |
/// | `Io`            | Transient   | File system issue; may resolve               |
/// | `Rhai`          | Transient   | Script evaluation failure                    |
/// | `Wasm`          | Transient   | WASM execution failure                       |
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// Extension not found (fatal — ID not registered).
    #[error("Extension not found: {id}")]
    NotFound { id: String },

    /// Failed to load extension (fatal — binary/path issue).
    #[error("Failed to load extension '{id}': {reason}")]
    LoadError { id: String, reason: String },

    /// Failed to unload extension (fatal — internal error).
    #[error("Failed to unload extension '{id}': {reason}")]
    UnloadError { id: String, reason: String },

    /// Script execution error (transient — may succeed after fix/reload).
    #[error("Script execution error in '{script_id}': {reason}")]
    ScriptError { script_id: String, reason: String },

    /// Plugin initialization error (fatal — entry function failed).
    #[error("Plugin initialization error in '{plugin_id}': {reason}")]
    PluginInitError { plugin_id: String, reason: String },

    /// Invalid manifest (fatal — fix the plugin manifest).
    #[error("Invalid manifest for '{id}': {reason}")]
    InvalidManifest { id: String, reason: String },

    /// Permission denied (fatal — plugin lacks required capability).
    #[error("Permission denied for extension '{id}': {action}")]
    PermissionDenied { id: String, action: String },

    /// Extension already loaded (fatal — logic error).
    #[error("Extension '{id}' is already loaded")]
    AlreadyLoaded { id: String },

    /// IO error (transient — file system issue).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// RHAI script error (transient — script evaluation failure).
    #[error("RHAI error: {0}")]
    Rhai(String),

    /// WASM plugin error (transient — execution failure).
    #[cfg(feature = "wasm")]
    #[error("WASM error: {0}")]
    Wasm(#[from] nemo_wasm::WasmError),
}

impl From<Box<rhai::EvalAltResult>> for ExtensionError {
    fn from(err: Box<rhai::EvalAltResult>) -> Self {
        ExtensionError::Rhai(err.to_string())
    }
}
