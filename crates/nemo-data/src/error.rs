//! Error types for the data flow engine.
//!
//! # Recovery Classification
//!
//! Errors are classified as **transient** (retryable) or **fatal** (requires
//! user/config intervention). Transient errors may succeed on a subsequent
//! attempt; fatal errors indicate a logic or configuration problem.

use thiserror::Error;

/// Error from a data source.
///
/// # Recovery
///
/// | Variant        | Recoverable? | Notes                                   |
/// |---------------|-------------|------------------------------------------|
/// | `Connection`  | Transient   | Retry with backoff; endpoint may be down |
/// | `Request`     | Transient   | Retry; may be a transient server error   |
/// | `Parse`       | Fatal       | Data format mismatch; fix config/source  |
/// | `Timeout`     | Transient   | Retry; consider increasing timeout       |
/// | `NotStarted`  | Fatal       | Logic error — call `start()` first       |
/// | `AlreadyRunning` | Fatal    | Logic error — source is already active   |
/// | `Io`          | Transient   | Depends on underlying cause              |
#[derive(Debug, Error)]
pub enum DataSourceError {
    /// Connection error (transient — retry with backoff).
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request error (transient — retry).
    #[error("Request error: {0}")]
    Request(String),

    /// Parse error (fatal — data format mismatch).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Timeout error (transient — retry or increase timeout).
    #[error("Timeout")]
    Timeout,

    /// Source not started (fatal — logic error).
    #[error("Source not started")]
    NotStarted,

    /// Source already running (fatal — logic error).
    #[error("Source already running")]
    AlreadyRunning,

    /// IO error (transient — depends on underlying cause).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error during transformation.
///
/// # Recovery
///
/// All transform errors are **fatal** for the current data update. They
/// indicate a mismatch between the data shape and the transform
/// configuration. Fix the transform pipeline or the data source schema.
#[derive(Debug, Error)]
pub enum TransformError {
    /// Invalid input type.
    #[error("Invalid input type: expected {expected}, got {actual}")]
    InvalidType { expected: String, actual: String },

    /// Expression evaluation error.
    #[error("Expression error: {0}")]
    Expression(String),

    /// Missing field.
    #[error("Missing field: {0}")]
    MissingField(String),

    /// Index out of bounds.
    #[error("Index out of bounds: {index} (length: {length})")]
    IndexOutOfBounds { index: usize, length: usize },
}

/// Error in the transform pipeline.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Transform failed at a specific stage.
    #[error("Transform failed at stage {stage}: {error}")]
    TransformFailed {
        stage: usize,
        #[source]
        error: TransformError,
    },

    /// Pipeline is empty.
    #[error("Pipeline is empty")]
    Empty,
}

/// Error in the repository.
///
/// # Recovery
///
/// | Variant        | Recoverable? | Notes                                    |
/// |---------------|-------------|-------------------------------------------|
/// | `NotFound`    | Transient   | Path may not exist yet; retry after write |
/// | `InvalidPath` | Fatal       | Malformed path string; fix config         |
/// | `TypeMismatch`| Fatal       | Schema/binding mismatch; fix config       |
/// | `LockError`   | Fatal       | Mutex poisoned — unrecoverable            |
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Path not found.
    #[error("Path not found: {0}")]
    NotFound(String),

    /// Invalid path.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Type mismatch.
    #[error("Type mismatch at path {path}: expected {expected}, got {actual}")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    /// Lock error.
    #[error("Failed to acquire lock")]
    LockError,
}

/// Error in the binding system.
///
/// # Recovery
///
/// All binding errors are **fatal** for the affected binding. They indicate
/// a configuration problem (wrong path, missing component, incompatible
/// transform). Fix the binding definition in the XML config.
#[derive(Debug, Error)]
pub enum BindingError {
    /// Source path not found.
    #[error("Source path not found: {0}")]
    SourceNotFound(String),

    /// Target component not found.
    #[error("Target component not found: {0}")]
    TargetNotFound(String),

    /// Transform error.
    #[error("Transform error: {0}")]
    Transform(#[from] TransformError),

    /// Invalid binding mode.
    #[error("Invalid binding mode for this operation")]
    InvalidMode,
}

/// Error during action execution.
///
/// # Recovery
///
/// | Variant           | Recoverable? | Notes                                 |
/// |------------------|-------------|----------------------------------------|
/// | `NotFound`       | Fatal       | Action name typo or not registered     |
/// | `ExecutionFailed`| Transient   | Action logic failed; may succeed later |
/// | `InvalidParams`  | Fatal       | Wrong parameters; fix config           |
/// | `Timeout`        | Transient   | Retry or increase timeout              |
#[derive(Debug, Error)]
pub enum ActionError {
    /// Action not found.
    #[error("Action not found: {0}")]
    NotFound(String),

    /// Execution failed.
    #[error("Action execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid parameters.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Timeout.
    #[error("Action timed out")]
    Timeout,
}

/// Top-level data flow error.
///
/// # Recovery
///
/// Recoverability depends on the wrapped inner error. Check the inner
/// error's documentation to determine whether a retry is appropriate.
#[derive(Debug, Error)]
pub enum DataFlowError {
    /// Source error.
    #[error("Source '{source_id}' error: {error}")]
    Source {
        source_id: String,
        #[source]
        error: DataSourceError,
    },

    /// Transform error.
    #[error("Transform error for source '{source_id}' at stage {stage}: {error}")]
    Transform {
        source_id: String,
        stage: usize,
        #[source]
        error: TransformError,
    },

    /// Repository error.
    #[error("Repository error at path '{path}': {error}")]
    Repository {
        path: String,
        #[source]
        error: RepositoryError,
    },

    /// Binding error.
    #[error("Binding '{binding_id}' error: {error}")]
    Binding {
        binding_id: String,
        #[source]
        error: BindingError,
    },

    /// Action error.
    #[error("Action '{action}' error: {error}")]
    Action {
        action: String,
        #[source]
        error: ActionError,
    },
}
