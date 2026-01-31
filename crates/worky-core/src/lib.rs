//! worky-core: Domain models and patch logic for work item management.
//!
//! This crate provides:
//! - `WorkItem`: The core work item model with normalized and custom fields
//! - `WorkEvent`: Append-only event log entries for tracking changes
//! - Patch operations for applying updates via JSON merge patch

pub mod error;
pub mod event;
pub mod item;
pub mod patch;

pub use error::{CoreError, Result};
pub use event::{
    AiActionPayload, AssigneeChangePayload, CommentPayload, EventPayload, EventType,
    FieldChangePayload, LabelPayload, StateChangePayload, WorkEvent,
};
pub use item::WorkItem;
pub use patch::{apply_merge_patch, apply_set_operation, diff_values, resolve_path, SetOperation};
