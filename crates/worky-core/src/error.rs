//! Error types for worky-core.

use thiserror::Error;

/// Result type alias for worky-core operations.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Errors that can occur in worky-core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Invalid path format for field access.
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Field not found at the specified path.
    #[error("field not found: {0}")]
    FieldNotFound(String),

    /// JSON serialization/deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid state transition.
    #[error("invalid state transition from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },

    /// Validation error.
    #[error("validation error: {0}")]
    Validation(String),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}
