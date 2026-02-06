//! Extension error types.

use thiserror::Error;

/// Errors that can occur in the extension system.
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// Extension not found.
    #[error("Extension not found: {id}")]
    NotFound { id: String },

    /// Failed to load extension.
    #[error("Failed to load extension '{id}': {reason}")]
    LoadError { id: String, reason: String },

    /// Failed to unload extension.
    #[error("Failed to unload extension '{id}': {reason}")]
    UnloadError { id: String, reason: String },

    /// Script execution error.
    #[error("Script execution error in '{script_id}': {reason}")]
    ScriptError { script_id: String, reason: String },

    /// Plugin initialization error.
    #[error("Plugin initialization error in '{plugin_id}': {reason}")]
    PluginInitError { plugin_id: String, reason: String },

    /// Invalid manifest.
    #[error("Invalid manifest for '{id}': {reason}")]
    InvalidManifest { id: String, reason: String },

    /// Permission denied.
    #[error("Permission denied for extension '{id}': {action}")]
    PermissionDenied { id: String, action: String },

    /// Extension already loaded.
    #[error("Extension '{id}' is already loaded")]
    AlreadyLoaded { id: String },

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// RHAI error.
    #[error("RHAI error: {0}")]
    Rhai(String),
}

impl From<Box<rhai::EvalAltResult>> for ExtensionError {
    fn from(err: Box<rhai::EvalAltResult>) -> Self {
        ExtensionError::Rhai(err.to_string())
    }
}
