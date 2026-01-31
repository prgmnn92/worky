//! Error types for the filesystem backend.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for filesystem operations.
pub type Result<T> = std::result::Result<T, FsError>;

/// Errors that can occur in filesystem operations.
#[derive(Debug, Error)]
pub enum FsError {
    /// Workspace not found at the specified path.
    #[error("workspace not found at '{0}'")]
    WorkspaceNotFound(PathBuf),

    /// Workspace already exists.
    #[error("workspace already exists at '{0}'")]
    WorkspaceExists(PathBuf),

    /// Work item not found.
    #[error("work item not found: {0}")]
    ItemNotFound(String),

    /// Work item already exists.
    #[error("work item already exists: {0}")]
    ItemExists(String),

    /// Invalid UID format.
    #[error("invalid UID format: {0}")]
    InvalidUid(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error.
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Core library error.
    #[error("core error: {0}")]
    Core(#[from] worky_core::CoreError),

    /// Invalid slug.
    #[error("invalid slug: {0}")]
    InvalidSlug(String),
}
