//! Error types for the data flow engine.

use thiserror::Error;

/// Error from a data source.
#[derive(Debug, Error)]
pub enum DataSourceError {
    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request error.
    #[error("Request error: {0}")]
    Request(String),

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Timeout error.
    #[error("Timeout")]
    Timeout,

    /// Source not started.
    #[error("Source not started")]
    NotStarted,

    /// Source already running.
    #[error("Source already running")]
    AlreadyRunning,

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error during transformation.
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
