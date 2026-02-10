//! Error types for the component registry.

use crate::registry::EntityType;
use thiserror::Error;

/// Error during registration.
#[derive(Debug, Error)]
pub enum RegistrationError {
    /// Entity is already registered.
    #[error("{entity_type:?} '{name}' is already registered")]
    AlreadyRegistered {
        entity_type: EntityType,
        name: String,
    },

    /// Invalid descriptor.
    #[error("Invalid descriptor: {0}")]
    InvalidDescriptor(String),

    /// Lock error.
    #[error("Failed to acquire registry lock")]
    LockError,
}
