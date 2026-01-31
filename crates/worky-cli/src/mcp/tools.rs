//! MCP tool definitions and handlers.

use super::protocol::{ToolCallResult, ToolDefinition};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use worky_core::SetOperation;
use worky_fs::{workspace::ItemFilter, Workspace};

/// Get all available tool definitions.
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "worky_list".to_string(),
            description: "List work items in the workspace. Returns a summary of all items with optional filtering by state, assignee, or label.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "state": {
                        "type": "string",
                        "description": "Filter by state (e.g., TODO, IN_PROGRESS, DONE)"
                    },
                    "assignee": {
                        "type": "string",
                        "description": "Filter by assignee name"
                    },
                    "label": {
                        "type": "string",
                        "description": "Filter by label"
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_get".to_string(),
            description: "Get detailed information about a specific work item including its comments/notes.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID (e.g., fs:implement-auth)"
                    },
                    "comments": {
                        "type": "integer",
                        "description": "Number of recent comments to include (default: 10)",
                        "default": 10
                    }
                },
                "required": ["uid"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_create".to_string(),
            description: "Create a new work item. Returns the created item details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Title of the work item"
                    },
                    "state": {
                        "type": "string",
                        "description": "Initial state (default: TODO)"
                    },
                    "assignee": {
                        "type": "string",
                        "description": "Assignee for the item"
                    },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Labels to attach to the item"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of the work item"
                    }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_set".to_string(),
            description: "Update fields on a work item. Use dot notation for nested fields (e.g., fields.priority).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID"
                    },
                    "state": {
                        "type": "string",
                        "description": "New state value"
                    },
                    "assignee": {
                        "type": "string",
                        "description": "New assignee (use empty string to unassign)"
                    },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Replace all labels with these"
                    },
                    "fields": {
                        "type": "object",
                        "description": "Custom fields to set (e.g., {\"priority\": \"high\", \"estimate\": 5})",
                        "additionalProperties": true
                    }
                },
                "required": ["uid"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_log".to_string(),
            description: "Add a comment or note to a work item. Use this to document progress, decisions, or blockers.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID"
                    },
                    "message": {
                        "type": "string",
                        "description": "The comment/note to add"
                    }
                },
                "required": ["uid", "message"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_events".to_string(),
            description: "Get the event history for a work item showing all changes made over time.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID"
                    },
                    "since_days": {
                        "type": "integer",
                        "description": "Only show events from the last N days"
                    }
                },
                "required": ["uid"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_advance".to_string(),
            description: "Advance a work item to the next state in the workflow: TODO → IN_PROGRESS → IN_REVIEW → DONE".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID"
                    }
                },
                "required": ["uid"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "worky_revert".to_string(),
            description: "Move a work item back to the previous state in the workflow: DONE → IN_REVIEW → IN_PROGRESS → TODO".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The work item UID"
                    }
                },
                "required": ["uid"],
                "additionalProperties": false
            }),
        },
    ]
}

/// State workflow order.
const STATE_WORKFLOW: &[&str] = &["TODO", "IN_PROGRESS", "IN_REVIEW", "DONE"];

/// Handle a tool call and return the result.
pub fn handle_tool_call(workspace_path: &Path, name: &str, arguments: Option<Value>) -> ToolCallResult {
    let args = arguments.unwrap_or(json!({}));

    match name {
        "worky_list" => handle_list(workspace_path, args),
        "worky_get" => handle_get(workspace_path, args),
        "worky_create" => handle_create(workspace_path, args),
        "worky_set" => handle_set(workspace_path, args),
        "worky_log" => handle_log(workspace_path, args),
        "worky_events" => handle_events(workspace_path, args),
        "worky_advance" => handle_advance(workspace_path, args),
        "worky_revert" => handle_revert(workspace_path, args),
        _ => ToolCallResult::error(format!("Unknown tool: {name}")),
    }
}

#[derive(Deserialize, Default)]
struct ListArgs {
    state: Option<String>,
    assignee: Option<String>,
    label: Option<String>,
}

fn handle_list(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: ListArgs = serde_json::from_value(args).unwrap_or_default();

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let filter = if args.state.is_some() || args.assignee.is_some() || args.label.is_some() {
        Some(ItemFilter {
            state: args.state,
            assignee: args.assignee,
            label: args.label,
        })
    } else {
        None
    };

    let items = match ws.list_items(filter.as_ref()) {
        Ok(items) => items,
        Err(e) => return ToolCallResult::error(format!("Failed to list items: {e}")),
    };

    if items.is_empty() {
        return ToolCallResult::text("No work items found.");
    }

    let mut output = String::new();
    output.push_str(&format!("Found {} work item(s):\n\n", items.len()));

    for item in &items {
        let assignee = item.assignee.as_deref().unwrap_or("-");
        output.push_str(&format!(
            "• {} [{}] @{}\n  {}\n\n",
            item.uid, item.state, assignee, item.title
        ));
    }

    ToolCallResult::text(output)
}

#[derive(Deserialize)]
struct GetArgs {
    uid: String,
    #[serde(default = "default_comments")]
    comments: usize,
}

fn default_comments() -> usize {
    10
}

fn handle_get(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: GetArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let item = match ws.get_item(&args.uid) {
        Ok(item) => item,
        Err(e) => return ToolCallResult::error(format!("Failed to get item: {e}")),
    };

    let mut output = String::new();
    output.push_str(&format!("UID: {}\n", item.uid));
    output.push_str(&format!("Title: {}\n", item.title));
    output.push_str(&format!("State: {}\n", item.state));

    if let Some(assignee) = &item.assignee {
        output.push_str(&format!("Assignee: {assignee}\n"));
    }

    if !item.labels.is_empty() {
        output.push_str(&format!("Labels: {}\n", item.labels.join(", ")));
    }

    output.push_str(&format!("Created: {}\n", item.created_at.format("%Y-%m-%d %H:%M UTC")));
    output.push_str(&format!("Updated: {}\n", item.updated_at.format("%Y-%m-%d %H:%M UTC")));

    if !item.fields.is_empty() {
        output.push_str("\nCustom Fields:\n");
        for (key, value) in &item.fields {
            output.push_str(&format!("  {key}: {value}\n"));
        }
    }

    // Get comments
    if args.comments > 0 {
        if let Ok(events) = ws.read_events(&args.uid, None) {
            let comments: Vec<_> = events
                .into_iter()
                .filter(|e| matches!(e.event_type, worky_core::EventType::CommentAdded))
                .collect();

            let recent: Vec<_> = comments.into_iter().rev().take(args.comments).collect();

            if !recent.is_empty() {
                output.push_str("\nRecent Comments:\n");
                for event in recent.into_iter().rev() {
                    let actor = event.actor.as_deref().unwrap_or("user");
                    let time = event.timestamp.format("%Y-%m-%d %H:%M");
                    if let worky_core::EventPayload::Comment(p) = &event.payload {
                        output.push_str(&format!("  [{time}] {actor}: {}\n", p.message));
                    }
                }
            }
        }
    }

    ToolCallResult::text(output)
}

#[derive(Deserialize)]
struct CreateArgs {
    title: String,
    state: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
    description: Option<String>,
}

fn handle_create(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: CreateArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let item = match ws.create_item(&args.title) {
        Ok(item) => item,
        Err(e) => return ToolCallResult::error(format!("Failed to create item: {e}")),
    };

    // Apply additional fields
    let mut operations = Vec::new();

    if let Some(state) = args.state {
        operations.push(SetOperation::new("state", state));
    }

    if let Some(assignee) = args.assignee {
        operations.push(SetOperation::new("assignee", assignee));
    }

    if let Some(labels) = args.labels {
        if !labels.is_empty() {
            operations.push(SetOperation::new(
                "labels",
                serde_json::Value::Array(labels.into_iter().map(serde_json::Value::String).collect()),
            ));
        }
    }

    if let Some(description) = args.description {
        operations.push(SetOperation::new("fields.description", description));
    }

    let final_item = if operations.is_empty() {
        item
    } else {
        match ws.update_item(&item.uid, &operations) {
            Ok(item) => item,
            Err(e) => return ToolCallResult::error(format!("Created item but failed to set fields: {e}")),
        }
    };

    ToolCallResult::text(format!(
        "Created work item: {}\nTitle: {}\nState: {}",
        final_item.uid, final_item.title, final_item.state
    ))
}

#[derive(Deserialize)]
struct SetArgs {
    uid: String,
    state: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
    fields: Option<serde_json::Map<String, Value>>,
}

fn handle_set(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: SetArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let mut operations = Vec::new();

    if let Some(state) = args.state {
        operations.push(SetOperation::new("state", state));
    }

    if let Some(assignee) = args.assignee {
        if assignee.is_empty() {
            operations.push(SetOperation::new("assignee", Value::Null));
        } else {
            operations.push(SetOperation::new("assignee", assignee));
        }
    }

    if let Some(labels) = args.labels {
        operations.push(SetOperation::new(
            "labels",
            serde_json::Value::Array(labels.into_iter().map(serde_json::Value::String).collect()),
        ));
    }

    if let Some(fields) = args.fields {
        for (key, value) in fields {
            operations.push(SetOperation::new(format!("fields.{key}"), value));
        }
    }

    if operations.is_empty() {
        return ToolCallResult::error("No fields to update. Specify at least one of: state, assignee, labels, or fields.");
    }

    let item = match ws.update_item(&args.uid, &operations) {
        Ok(item) => item,
        Err(e) => return ToolCallResult::error(format!("Failed to update item: {e}")),
    };

    ToolCallResult::text(format!(
        "Updated work item: {}\nState: {}\nAssignee: {}",
        item.uid,
        item.state,
        item.assignee.as_deref().unwrap_or("-")
    ))
}

#[derive(Deserialize)]
struct LogArgs {
    uid: String,
    message: String,
}

fn handle_log(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: LogArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    match ws.add_comment(&args.uid, &args.message) {
        Ok(_) => ToolCallResult::text(format!("Added comment to {}", args.uid)),
        Err(e) => ToolCallResult::error(format!("Failed to add comment: {e}")),
    }
}

#[derive(Deserialize)]
struct EventsArgs {
    uid: String,
    since_days: Option<u32>,
}

fn handle_events(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: EventsArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let since = args.since_days.map(|days| {
        chrono::Utc::now() - chrono::Duration::days(i64::from(days))
    });

    let events = match ws.read_events(&args.uid, since) {
        Ok(events) => events,
        Err(e) => return ToolCallResult::error(format!("Failed to read events: {e}")),
    };

    if events.is_empty() {
        return ToolCallResult::text("No events found.");
    }

    let mut output = String::new();
    output.push_str(&format!("Event history for {} ({} events):\n\n", args.uid, events.len()));

    for event in &events {
        let actor = event.actor.as_deref().unwrap_or("system");
        let time = event.timestamp.format("%Y-%m-%d %H:%M");
        let payload = format_payload(&event.payload);
        output.push_str(&format!("[{time}] {}: {} - {}\n", event.event_type, actor, payload));
    }

    ToolCallResult::text(output)
}

#[derive(Deserialize)]
struct AdvanceArgs {
    uid: String,
}

fn handle_advance(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: AdvanceArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let item = match ws.get_item(&args.uid) {
        Ok(item) => item,
        Err(e) => return ToolCallResult::error(format!("Failed to get item: {e}")),
    };

    let current_state = item.state.as_str();
    let current_idx = STATE_WORKFLOW
        .iter()
        .position(|&s| s.eq_ignore_ascii_case(current_state));

    let next_state = match current_idx {
        Some(idx) if idx < STATE_WORKFLOW.len() - 1 => STATE_WORKFLOW[idx + 1],
        Some(_) => {
            return ToolCallResult::text(format!(
                "{} is already at final state ({})",
                args.uid, current_state
            ));
        }
        None => "IN_PROGRESS", // Unknown state, move to IN_PROGRESS
    };

    let operations = vec![SetOperation::new("state", next_state)];
    match ws.update_item(&args.uid, &operations) {
        Ok(_) => ToolCallResult::text(format!(
            "Advanced {}: {} → {}",
            args.uid, current_state, next_state
        )),
        Err(e) => ToolCallResult::error(format!("Failed to update item: {e}")),
    }
}

fn handle_revert(workspace_path: &Path, args: Value) -> ToolCallResult {
    let args: AdvanceArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {e}")),
    };

    let ws = match Workspace::open(workspace_path) {
        Ok(ws) => ws,
        Err(e) => return ToolCallResult::error(format!("Failed to open workspace: {e}")),
    };

    let item = match ws.get_item(&args.uid) {
        Ok(item) => item,
        Err(e) => return ToolCallResult::error(format!("Failed to get item: {e}")),
    };

    let current_state = item.state.as_str();
    let current_idx = STATE_WORKFLOW
        .iter()
        .position(|&s| s.eq_ignore_ascii_case(current_state));

    let prev_state = match current_idx {
        Some(idx) if idx > 0 => STATE_WORKFLOW[idx - 1],
        Some(_) => {
            return ToolCallResult::text(format!(
                "{} is already at initial state ({})",
                args.uid, current_state
            ));
        }
        None => "TODO", // Unknown state, move to TODO
    };

    let operations = vec![SetOperation::new("state", prev_state)];
    match ws.update_item(&args.uid, &operations) {
        Ok(_) => ToolCallResult::text(format!(
            "Reverted {}: {} → {}",
            args.uid, current_state, prev_state
        )),
        Err(e) => ToolCallResult::error(format!("Failed to update item: {e}")),
    }
}

fn format_payload(payload: &worky_core::EventPayload) -> String {
    match payload {
        worky_core::EventPayload::StateChange(p) => format!("{} → {}", p.from, p.to),
        worky_core::EventPayload::FieldChange(p) => format!("{} = {}", p.path, p.new_value),
        worky_core::EventPayload::AssigneeChange(p) => {
            format!(
                "{} → {}",
                p.from.as_deref().unwrap_or("(none)"),
                p.to.as_deref().unwrap_or("(none)")
            )
        }
        worky_core::EventPayload::Label(p) => p.label.clone(),
        worky_core::EventPayload::Comment(p) => {
            if p.message.len() > 50 {
                format!("{}...", &p.message[..47])
            } else {
                p.message.clone()
            }
        }
        worky_core::EventPayload::AiAction(p) => format!("{}: {}", p.tool, p.action),
        worky_core::EventPayload::Generic(v) => v.to_string(),
    }
}
