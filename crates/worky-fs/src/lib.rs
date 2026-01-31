//! Filesystem backend for worky work item storage.
//!
//! Stores work items as directories with:
//! - `meta.yml`: Item metadata
//! - `events.ndjson`: Append-only event log
//! - `notes.md`: Free-form notes
//! - `artifacts/`: Attached files

pub mod config;
pub mod error;
pub mod workspace;

pub use config::WorkspaceConfig;
pub use error::{FsError, Result};
pub use workspace::Workspace;
