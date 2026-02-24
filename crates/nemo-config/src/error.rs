//! Error types for the configuration engine.

use crate::location::SourceLocation;
use crate::path::ConfigPath;
use crate::schema::ValidationRule;
use thiserror::Error;

/// Main configuration error type.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error during parsing.
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// Error during validation.
    #[error("Validation failed with {} error(s)", errors.len())]
    Validation { errors: Vec<ValidationError> },

    /// Error during expression resolution.
    #[error("Resolution error: {0}")]
    Resolve(#[from] ResolveError),

    /// IO error.
    #[error("IO error reading {path}: {message}")]
    Io { path: String, message: String },

    /// Schema not found.
    #[error("Schema not found: {name}")]
    SchemaNotFound { name: String },
}

/// Error during configuration parsing.
#[derive(Debug)]
pub struct ParseError {
    /// Error message.
    pub message: String,
    /// Source location of the error.
    pub location: SourceLocation,
    /// The source content.
    pub source: String,
    /// Suggested fixes.
    pub suggestions: Vec<String>,
}

impl ParseError {
    /// Creates a new parse error.
    pub fn new(message: impl Into<String>, location: SourceLocation) -> Self {
        ParseError {
            message: message.into(),
            location,
            source: String::new(),
            suggestions: Vec::new(),
        }
    }

    /// Adds the source content for context.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Adds a suggestion.
    #[allow(dead_code)]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Error during validation.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Path to the problematic value.
    pub path: ConfigPath,
    /// Error message.
    pub message: String,
    /// Expected value/type.
    pub expected: Option<String>,
    /// Actual value/type.
    pub actual: Option<String>,
    /// Source location if available.
    pub location: Option<SourceLocation>,
    /// Error code.
    pub code: ErrorCode,
}

impl ValidationError {
    /// Creates a "missing required" error.
    pub fn missing_required(path: ConfigPath, field: &str) -> Self {
        ValidationError {
            path,
            message: format!("Missing required field: '{}'", field),
            expected: Some(format!("field '{}'", field)),
            actual: None,
            location: None,
            code: ErrorCode::MissingRequired,
        }
    }

    /// Creates a "type mismatch" error.
    pub fn type_mismatch(path: ConfigPath, expected: &str, actual: &str) -> Self {
        ValidationError {
            path,
            message: format!("Type mismatch: expected {}, got {}", expected, actual),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
            location: None,
            code: ErrorCode::InvalidType,
        }
    }

    /// Creates an "unknown property" error.
    pub fn unknown_property(path: ConfigPath, property: &str, known: Vec<String>) -> Self {
        let suggestion = if !known.is_empty() {
            format!(". Known properties: {}", known.join(", "))
        } else {
            String::new()
        };

        ValidationError {
            path,
            message: format!("Unknown property: '{}'{}", property, suggestion),
            expected: None,
            actual: Some(property.to_string()),
            location: None,
            code: ErrorCode::UnknownProperty,
        }
    }

    /// Creates a "rule violation" error.
    pub fn rule_violation(path: ConfigPath, rule: &ValidationRule, message: &str) -> Self {
        ValidationError {
            path,
            message: format!("Validation rule '{}' violated: {}", rule, message),
            expected: Some(rule.to_string()),
            actual: None,
            location: None,
            code: ErrorCode::InvalidValue,
        }
    }

    /// Creates a "schema not found" error.
    pub fn schema_not_found(name: &str) -> Self {
        ValidationError {
            path: ConfigPath::root(),
            message: format!("Schema not found: '{}'", name),
            expected: Some(format!("schema '{}'", name)),
            actual: None,
            location: None,
            code: ErrorCode::SchemaNotFound,
        }
    }

    /// Creates a load error wrapper.
    pub fn load_error(message: &str) -> Self {
        ValidationError {
            path: ConfigPath::root(),
            message: format!("Failed to load configuration: {}", message),
            expected: None,
            actual: None,
            location: None,
            code: ErrorCode::LoadError,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Error codes for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Missing required field.
    MissingRequired,
    /// Type mismatch.
    InvalidType,
    /// Invalid value.
    InvalidValue,
    /// Unknown property.
    UnknownProperty,
    /// Schema not found.
    SchemaNotFound,
    /// Load error.
    LoadError,
}

/// Error during expression resolution.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// Undefined variable.
    #[error("Undefined variable: '{name}'")]
    UndefinedVariable { name: String },

    /// Unknown function.
    #[error("Unknown function: '{name}'")]
    UnknownFunction { name: String },

    /// Invalid argument.
    #[error("Invalid argument to '{function}': {message}")]
    InvalidArgument { function: String, message: String },

    /// Invalid path.
    #[error("Invalid path '{path}': {message}")]
    InvalidPath { path: String, message: String },
}

/// Schema-related errors.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Schema already registered.
    #[error("Schema '{name}' is already registered")]
    AlreadyRegistered { name: String },

    /// Lock error.
    #[error("Failed to acquire lock on schema registry")]
    LockError,
}
