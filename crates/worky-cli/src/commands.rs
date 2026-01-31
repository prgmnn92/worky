//! CLI command implementations.

use crate::interactive;
use crate::output::{self, OutputFormat, WorkItemSummary};
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use console::style;
use std::path::Path;
use worky_core::SetOperation;
use worky_fs::{workspace::ItemFilter, Workspace};

/// Initialize a new workspace.
pub fn init(path: &Path, format: OutputFormat) -> Result<()> {
    Workspace::init(path).context("Failed to initialize workspace")?;
    output::print_success(
        &format!("Initialized workspace at {}", path.display()),
        format,
    );
    Ok(())
}

/// Create a new work item (non-interactive).
pub fn new_item(
    path: &Path,
    title: &str,
    state: Option<String>,
    labels: Vec<String>,
    assignee: Option<String>,
    description: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;

    let item = ws.create_item(title).context("Failed to create item")?;

    // Apply additional fields if specified
    let mut operations = Vec::new();

    if let Some(state) = state {
        operations.push(SetOperation::new("state", state));
    }

    if let Some(assignee) = assignee {
        operations.push(SetOperation::new("assignee", assignee));
    }

    if !labels.is_empty() {
        operations.push(SetOperation::new(
            "labels",
            serde_json::Value::Array(labels.into_iter().map(serde_json::Value::String).collect()),
        ));
    }

    if let Some(description) = description {
        operations.push(SetOperation::new("fields.description", description));
    }

    let final_item = if operations.is_empty() {
        item
    } else {
        ws.update_item(&item.uid, &operations)
            .context("Failed to apply initial fields")?
    };

    output::print(&final_item, format);
    Ok(())
}

/// Create a new work item interactively.
pub fn new_interactive(path: &Path, format: OutputFormat) -> Result<()> {
    // Verify workspace exists first
    let ws = Workspace::open(path).context("Failed to open workspace")?;
    drop(ws); // Close it for now

    // Run interactive prompts (returns None if cancelled)
    let input = match interactive::prompt_new_item()? {
        Some(input) => input,
        None => {
            println!("{}", style("  Cancelled.").dim());
            return Ok(());
        }
    };

    // Create the item
    let ws = Workspace::open(path)?;
    let item = ws.create_item(&input.title).context("Failed to create item")?;

    // Apply fields
    let mut operations = Vec::new();

    if input.state != "TODO" {
        operations.push(SetOperation::new("state", input.state.clone()));
    }

    if let Some(assignee) = &input.assignee {
        operations.push(SetOperation::new("assignee", assignee.clone()));
    }

    if !input.labels.is_empty() {
        operations.push(SetOperation::new(
            "labels",
            serde_json::Value::Array(
                input
                    .labels
                    .iter()
                    .map(|l| serde_json::Value::String(l.clone()))
                    .collect(),
            ),
        ));
    }

    if let Some(ref description) = input.description {
        operations.push(SetOperation::new("fields.description", description.clone()));
    }

    let final_item = if operations.is_empty() {
        item
    } else {
        ws.update_item(&item.uid, &operations)
            .context("Failed to apply fields")?
    };

    println!();
    println!("{}", style("  ✓ Created!").green().bold());
    println!();
    output::print(&final_item, format);
    Ok(())
}

/// List work items.
pub fn list(
    path: &Path,
    state: Option<String>,
    assignee: Option<String>,
    label: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;

    let filter = if state.is_some() || assignee.is_some() || label.is_some() {
        Some(ItemFilter {
            state,
            assignee,
            label,
        })
    } else {
        None
    };

    let items = ws
        .list_items(filter.as_ref())
        .context("Failed to list items")?;

    if items.is_empty() {
        output::print_success("No items found", format);
        return Ok(());
    }

    // Print header for human format
    if matches!(format, OutputFormat::Human) {
        println!(
            "{:<30} {:12} {:10} {}",
            "UID", "STATE", "ASSIGNEE", "TITLE"
        );
        println!("{}", "-".repeat(80));
    }

    let summaries: Vec<WorkItemSummary> = items.iter().map(WorkItemSummary::from).collect();
    output::print_list(&summaries, format);

    Ok(())
}

/// Get a work item by UID.
pub fn get(path: &Path, uid: &str, comment_count: usize, format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;
    let item = ws.get_item(uid).context("Failed to get item")?;

    // Get comments if requested
    let comments = if comment_count > 0 {
        let events = ws.read_events(uid, None).unwrap_or_default();
        // Filter to only COMMENT_ADDED events and take last N
        events
            .into_iter()
            .filter(|e| matches!(e.event_type, worky_core::EventType::CommentAdded))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(comment_count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        vec![]
    };

    output::print_item_with_comments(&item, &comments, format);
    Ok(())
}

/// Set field values on a work item.
pub fn set(path: &Path, uid: &str, assignments: &[String], format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;

    let operations: Vec<SetOperation> = assignments
        .iter()
        .map(|a| SetOperation::parse(a))
        .collect::<worky_core::Result<Vec<_>>>()
        .context("Failed to parse assignments")?;

    let item = ws
        .update_item(uid, &operations)
        .context("Failed to update item")?;

    output::print(&item, format);
    Ok(())
}

/// Apply a JSON merge patch.
pub fn patch(path: &Path, uid: &str, merge_json: &str, format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;

    let patch: serde_json::Value =
        serde_json::from_str(merge_json).context("Invalid JSON patch")?;

    let item = ws.patch_item(uid, &patch).context("Failed to patch item")?;

    output::print(&item, format);
    Ok(())
}

/// Show event history.
pub fn events(
    path: &Path,
    uid: &str,
    since_days: Option<u32>,
    format: OutputFormat,
) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;

    let since = since_days.map(|days| Utc::now() - Duration::days(i64::from(days)));

    let events = ws
        .read_events(uid, since)
        .context("Failed to read events")?;

    if events.is_empty() {
        output::print_success("No events found", format);
        return Ok(());
    }

    output::print_list(&events, format);
    Ok(())
}

/// Add a comment/log entry.
pub fn log(path: &Path, uid: &str, message: &str, format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;
    ws.add_comment(uid, message)
        .context("Failed to add comment")?;
    output::print_success("Comment added", format);
    Ok(())
}

/// State workflow order.
const STATE_WORKFLOW: &[&str] = &["TODO", "IN_PROGRESS", "IN_REVIEW", "DONE"];

/// Advance a work item to the next state.
pub fn advance(path: &Path, uid: &str, format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;
    let item = ws.get_item(uid).context("Failed to get item")?;

    let current_state = item.state.as_str();

    // Find current position in workflow
    let current_idx = STATE_WORKFLOW
        .iter()
        .position(|&s| s.eq_ignore_ascii_case(current_state));

    let next_state = match current_idx {
        Some(idx) if idx < STATE_WORKFLOW.len() - 1 => STATE_WORKFLOW[idx + 1],
        Some(_) => {
            output::print_success(
                &format!("{uid} is already at final state ({current_state})"),
                format,
            );
            return Ok(());
        }
        None => {
            // Unknown state (e.g., BLOCKED), move to IN_PROGRESS
            "IN_PROGRESS"
        }
    };

    let operations = vec![SetOperation::new("state", next_state)];
    let updated = ws
        .update_item(uid, &operations)
        .context("Failed to update item")?;

    println!(
        "{}",
        style(format!("  {} → {}", current_state, next_state))
            .green()
            .bold()
    );
    output::print(&updated, format);
    Ok(())
}

/// Revert a work item to the previous state.
pub fn revert(path: &Path, uid: &str, format: OutputFormat) -> Result<()> {
    let ws = Workspace::open(path).context("Failed to open workspace")?;
    let item = ws.get_item(uid).context("Failed to get item")?;

    let current_state = item.state.as_str();

    // Find current position in workflow
    let current_idx = STATE_WORKFLOW
        .iter()
        .position(|&s| s.eq_ignore_ascii_case(current_state));

    let prev_state = match current_idx {
        Some(idx) if idx > 0 => STATE_WORKFLOW[idx - 1],
        Some(_) => {
            output::print_success(
                &format!("{uid} is already at initial state ({current_state})"),
                format,
            );
            return Ok(());
        }
        None => {
            // Unknown state (e.g., BLOCKED), move to TODO
            "TODO"
        }
    };

    let operations = vec![SetOperation::new("state", prev_state)];
    let updated = ws
        .update_item(uid, &operations)
        .context("Failed to update item")?;

    println!(
        "{}",
        style(format!("  {} → {}", current_state, prev_state))
            .yellow()
            .bold()
    );
    output::print(&updated, format);
    Ok(())
}

/// Start the tool server.
pub fn tool_serve(path: &Path, host: &str, port: u16) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { worky_toolserver::serve(path, host, port).await })
}
