//! Output formatting for the CLI.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::Write;
use worky_core::{WorkEvent, WorkItem};

/// Output format for CLI responses.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output
    #[default]
    Human,
    /// JSON output
    Json,
    /// YAML output
    Yaml,
}

/// Print output in the specified format.
pub fn print<T: Serialize + HumanDisplay>(value: &T, format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("{}", value.human_display()),
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).expect("Failed to serialize to JSON")
            );
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(value).expect("Failed to serialize to YAML")
            );
        }
    }
}

/// Print a list in the specified format.
pub fn print_list<T: Serialize + HumanDisplay>(values: &[T], format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            for value in values {
                println!("{}", value.human_display());
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(values).expect("Failed to serialize to JSON")
            );
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(values).expect("Failed to serialize to YAML")
            );
        }
    }
}

/// Print a list of work item summaries with dynamic column widths.
pub fn print_item_list(items: &[WorkItemSummary], format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            if items.is_empty() {
                println!("No work items found.");
                return;
            }

            // Calculate max widths (with minimum for headers)
            let uid_width = items.iter().map(|i| i.uid.len()).max().unwrap_or(3).max(3);
            let state_width = items.iter().map(|i| i.state.len()).max().unwrap_or(5).max(5);
            let assignee_width = items
                .iter()
                .map(|i| i.assignee.as_ref().map(|a| a.len()).unwrap_or(1))
                .max()
                .unwrap_or(1)
                .max(8);

            // Print header
            println!(
                "{:<uid_w$}  {:<state_w$}  {:<assignee_w$}  {}",
                "UID",
                "STATE",
                "ASSIGNEE",
                "TITLE",
                uid_w = uid_width,
                state_w = state_width,
                assignee_w = assignee_width
            );
            let total_width = uid_width + state_width + assignee_width + 20;
            println!("{}", "-".repeat(total_width));

            for item in items {
                let assignee = item.assignee.as_deref().unwrap_or("-");
                println!(
                    "{:<uid_w$}  {:<state_w$}  {:<assignee_w$}  {}",
                    item.uid,
                    item.state,
                    assignee,
                    item.title,
                    uid_w = uid_width,
                    state_w = state_width,
                    assignee_w = assignee_width
                );
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(items).expect("Failed to serialize to JSON")
            );
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(items).expect("Failed to serialize to YAML")
            );
        }
    }
}

/// Print a success message.
pub fn print_success(message: &str, format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("{message}"),
        OutputFormat::Json => {
            println!(r#"{{"status": "ok", "message": "{message}"}}"#);
        }
        OutputFormat::Yaml => {
            println!("status: ok\nmessage: {message}");
        }
    }
}

/// Trait for human-readable display.
pub trait HumanDisplay {
    fn human_display(&self) -> String;
}

impl HumanDisplay for WorkItem {
    fn human_display(&self) -> String {
        let mut out = String::new();

        writeln!(out, "UID:       {}", self.uid).unwrap();
        writeln!(out, "Title:     {}", self.title).unwrap();
        writeln!(out, "State:     {}", self.state).unwrap();

        if let Some(assignee) = &self.assignee {
            writeln!(out, "Assignee:  {assignee}").unwrap();
        }

        if !self.labels.is_empty() {
            writeln!(out, "Labels:    {}", self.labels.join(", ")).unwrap();
        }

        writeln!(out, "Created:   {}", format_time(&self.created_at)).unwrap();
        writeln!(out, "Updated:   {}", format_time(&self.updated_at)).unwrap();

        if !self.fields.is_empty() {
            writeln!(out, "Fields:").unwrap();
            for (key, value) in &self.fields {
                writeln!(out, "  {key}: {value}").unwrap();
            }
        }

        out
    }
}

impl HumanDisplay for WorkEvent {
    fn human_display(&self) -> String {
        let actor = self.actor.as_deref().unwrap_or("system");
        let time = format_time(&self.timestamp);

        format!(
            "[{time}] {actor}: {type} - {payload}",
            type = self.event_type,
            payload = format_payload(&self.payload)
        )
    }
}

fn format_time(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn format_payload(payload: &worky_core::EventPayload) -> String {
    match payload {
        worky_core::EventPayload::StateChange(p) => format!("{} → {}", p.from, p.to),
        worky_core::EventPayload::FieldChange(p) => {
            format!("{} = {}", p.path, p.new_value)
        }
        worky_core::EventPayload::AssigneeChange(p) => {
            format!(
                "{} → {}",
                p.from.as_deref().unwrap_or("(none)"),
                p.to.as_deref().unwrap_or("(none)")
            )
        }
        worky_core::EventPayload::Label(p) => p.label.clone(),
        worky_core::EventPayload::Comment(p) => p.message.clone(),
        worky_core::EventPayload::AiAction(p) => format!("{}: {}", p.tool, p.action),
        worky_core::EventPayload::Generic(v) => v.to_string(),
    }
}

/// Summary view of a work item for list output.
#[derive(Debug, Serialize)]
pub struct WorkItemSummary {
    pub uid: String,
    pub title: String,
    pub state: String,
    pub assignee: Option<String>,
}

impl From<&WorkItem> for WorkItemSummary {
    fn from(item: &WorkItem) -> Self {
        Self {
            uid: item.uid.clone(),
            title: item.title.clone(),
            state: item.state.clone(),
            assignee: item.assignee.clone(),
        }
    }
}

impl HumanDisplay for WorkItemSummary {
    fn human_display(&self) -> String {
        let assignee = self.assignee.as_deref().unwrap_or("-");
        format!(
            "{:<30} {:12} {:10} {}",
            self.uid, self.state, assignee, self.title
        )
    }
}

/// Print a work item with its comments.
pub fn print_item_with_comments(
    item: &WorkItem,
    comments: &[WorkEvent],
    format: OutputFormat,
) {
    match format {
        OutputFormat::Human => {
            println!("{}", item.human_display());

            if !comments.is_empty() {
                println!("Comments:");
                println!("{}", "-".repeat(60));
                for comment in comments {
                    let time = format_time(&comment.timestamp);
                    let actor = comment.actor.as_deref().unwrap_or("user");
                    if let worky_core::EventPayload::Comment(p) = &comment.payload {
                        // Handle multi-line comments with proper indentation
                        let lines: Vec<&str> = p.message.lines().collect();
                        if lines.len() == 1 {
                            println!("  [{time}] {actor}: {}", p.message);
                        } else {
                            println!("  [{time}] {actor}:");
                            for line in lines {
                                println!("    {line}");
                            }
                        }
                    }
                }
            }
        }
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct ItemWithComments<'a> {
                #[serde(flatten)]
                item: &'a WorkItem,
                comments: Vec<CommentView<'a>>,
            }

            #[derive(serde::Serialize)]
            struct CommentView<'a> {
                timestamp: &'a DateTime<Utc>,
                actor: Option<&'a str>,
                message: &'a str,
            }

            let comment_views: Vec<CommentView> = comments
                .iter()
                .filter_map(|e| {
                    if let worky_core::EventPayload::Comment(p) = &e.payload {
                        Some(CommentView {
                            timestamp: &e.timestamp,
                            actor: e.actor.as_deref(),
                            message: &p.message,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let output = ItemWithComments {
                item,
                comments: comment_views,
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("Failed to serialize to JSON")
            );
        }
        OutputFormat::Yaml => {
            // For YAML, just print item then comments section
            println!(
                "{}",
                serde_yaml::to_string(item).expect("Failed to serialize to YAML")
            );
            if !comments.is_empty() {
                println!("comments:");
                for comment in comments {
                    if let worky_core::EventPayload::Comment(p) = &comment.payload {
                        let time = format_time(&comment.timestamp);
                        let actor = comment.actor.as_deref().unwrap_or("user");
                        println!("  - timestamp: {time}");
                        println!("    actor: {actor}");
                        // Handle multi-line in YAML
                        if p.message.contains('\n') {
                            println!("    message: |");
                            for line in p.message.lines() {
                                println!("      {line}");
                            }
                        } else {
                            println!("    message: {}", p.message);
                        }
                    }
                }
            }
        }
    }
}
