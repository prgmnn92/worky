//! Interactive prompts for work item creation with back navigation.

use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Editor, Input, MultiSelect, Select};

/// Default states available for selection.
const DEFAULT_STATES: &[&str] = &["TODO", "IN_PROGRESS", "IN_REVIEW", "BLOCKED", "DONE"];

/// Common labels for quick selection.
const COMMON_LABELS: &[&str] = &[
    "backend",
    "frontend",
    "bug",
    "feature",
    "documentation",
    "security",
    "performance",
    "devops",
    "testing",
    "urgent",
];

/// Data collected from interactive prompts.
#[derive(Debug, Default, Clone)]
pub struct NewItemInput {
    pub title: String,
    pub state: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub description: Option<String>,
}

/// Steps in the wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Title,
    State,
    Assignee,
    Labels,
    CustomLabels,
    Description,
    Confirm,
}

impl Step {
    fn next(self) -> Option<Step> {
        match self {
            Step::Title => Some(Step::State),
            Step::State => Some(Step::Assignee),
            Step::Assignee => Some(Step::Labels),
            Step::Labels => Some(Step::CustomLabels),
            Step::CustomLabels => Some(Step::Description),
            Step::Description => Some(Step::Confirm),
            Step::Confirm => None,
        }
    }

    fn prev(self) -> Option<Step> {
        match self {
            Step::Title => None,
            Step::State => Some(Step::Title),
            Step::Assignee => Some(Step::State),
            Step::Labels => Some(Step::Assignee),
            Step::CustomLabels => Some(Step::Labels),
            Step::Description => Some(Step::CustomLabels),
            Step::Confirm => Some(Step::Description),
        }
    }

}

/// Result of a prompt - either a value, go back, or cancel.
enum PromptResult<T> {
    Value(T),
    Back,
    Cancel,
}

/// Check if input is a back command.
fn is_back_command(input: &str) -> bool {
    let trimmed = input.trim().to_lowercase();
    trimmed == "<" || trimmed == "back" || trimmed == "b" || trimmed == ".."
}

/// Run interactive prompts to collect work item data.
pub fn prompt_new_item() -> Result<Option<NewItemInput>> {
    let theme = ColorfulTheme::default();
    let mut input = NewItemInput::default();
    let mut step = Step::Title;

    println!();
    println!("{}", style("  Create New Work Item").bold().cyan());
    println!("{}", style("  ─────────────────────").dim());
    println!(
        "  {}",
        style("Type '<' or 'back' to go back, Ctrl+C to cancel").dim()
    );
    println!();

    loop {
        match step {
            Step::Title => {
                match prompt_title(&theme, &input.title)? {
                    PromptResult::Value(v) => {
                        input.title = v;
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        // Can't go back from first step
                        println!("  {}", style("Already at first step").dim());
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::State => {
                match prompt_state(&theme, &input.state)? {
                    PromptResult::Value(v) => {
                        input.state = v;
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::Assignee => {
                match prompt_assignee(&theme, input.assignee.as_deref())? {
                    PromptResult::Value(v) => {
                        input.assignee = v;
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::Labels => {
                match prompt_labels(&theme, &input.labels)? {
                    PromptResult::Value(v) => {
                        input.labels = v;
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::CustomLabels => {
                match prompt_custom_labels(&theme, &input.labels)? {
                    PromptResult::Value(v) => {
                        // Merge with existing labels
                        for label in v {
                            if !input.labels.iter().any(|l| l.eq_ignore_ascii_case(&label)) {
                                input.labels.push(label);
                            }
                        }
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::Description => {
                match prompt_description(&theme, input.description.as_deref())? {
                    PromptResult::Value(v) => {
                        input.description = v;
                        step = step.next().unwrap();
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }

            Step::Confirm => {
                match prompt_confirm(&theme, &input)? {
                    PromptResult::Value(confirmed) => {
                        if confirmed {
                            return Ok(Some(input));
                        } else {
                            // Go back to allow editing
                            step = Step::Title;
                        }
                    }
                    PromptResult::Back => {
                        step = step.prev().unwrap();
                    }
                    PromptResult::Cancel => return Ok(None),
                }
            }
        }
    }
}

fn prompt_title(theme: &ColorfulTheme, current: &str) -> Result<PromptResult<String>> {
    let prompt = format!("{} Title", style("[1/6]").dim());

    let mut input_builder = Input::<String>::with_theme(theme).with_prompt(&prompt);

    if !current.is_empty() {
        input_builder = input_builder.default(current.to_string());
    }

    let result = input_builder.interact_text().context("Failed to read title")?;

    if is_back_command(&result) {
        return Ok(PromptResult::Back);
    }

    if result.trim().is_empty() {
        println!("  {}", style("Title cannot be empty").red());
        return prompt_title(theme, current);
    }

    Ok(PromptResult::Value(result.trim().to_string()))
}

fn prompt_state(theme: &ColorfulTheme, current: &str) -> Result<PromptResult<String>> {
    println!(
        "  {} {}",
        style("[2/6]").dim(),
        style("State (↑↓ to select, enter to confirm, '<' to go back)").dim()
    );

    let default_index = if current.is_empty() {
        0
    } else {
        DEFAULT_STATES
            .iter()
            .position(|&s| s.eq_ignore_ascii_case(current))
            .unwrap_or(0)
    };

    let selection = Select::with_theme(theme)
        .items(DEFAULT_STATES)
        .default(default_index)
        .interact_opt()
        .context("Failed to read state")?;

    match selection {
        Some(idx) => Ok(PromptResult::Value(DEFAULT_STATES[idx].to_string())),
        None => Ok(PromptResult::Back), // Esc pressed
    }
}

fn prompt_assignee(
    theme: &ColorfulTheme,
    current: Option<&str>,
) -> Result<PromptResult<Option<String>>> {
    let prompt = format!("{} Assignee (optional, '<' to go back)", style("[3/6]").dim());

    let mut input_builder = Input::<String>::with_theme(theme)
        .with_prompt(&prompt)
        .allow_empty(true);

    if let Some(c) = current {
        input_builder = input_builder.default(c.to_string());
    }

    let result = input_builder
        .interact_text()
        .context("Failed to read assignee")?;

    if is_back_command(&result) {
        return Ok(PromptResult::Back);
    }

    let trimmed = result.trim();
    if trimmed.is_empty() {
        Ok(PromptResult::Value(None))
    } else {
        Ok(PromptResult::Value(Some(trimmed.to_string())))
    }
}

fn prompt_labels(theme: &ColorfulTheme, current: &[String]) -> Result<PromptResult<Vec<String>>> {
    println!(
        "  {} {}",
        style("[4/6]").dim(),
        style("Labels (space to toggle, enter to confirm, Esc to go back)").dim()
    );

    // Pre-select current labels
    let defaults: Vec<bool> = COMMON_LABELS
        .iter()
        .map(|&label| current.iter().any(|l| l.eq_ignore_ascii_case(label)))
        .collect();

    let selection = MultiSelect::with_theme(theme)
        .items(COMMON_LABELS)
        .defaults(&defaults)
        .interact_opt()
        .context("Failed to read labels")?;

    match selection {
        Some(indices) => {
            let labels = indices
                .iter()
                .map(|&i| COMMON_LABELS[i].to_string())
                .collect();
            Ok(PromptResult::Value(labels))
        }
        None => Ok(PromptResult::Back), // Esc pressed
    }
}

fn prompt_custom_labels(
    theme: &ColorfulTheme,
    _current: &[String],
) -> Result<PromptResult<Vec<String>>> {
    let prompt = format!(
        "{} Additional labels (comma-separated, optional, '<' to go back)",
        style("[5/6]").dim()
    );

    let result: String = Input::with_theme(theme)
        .with_prompt(&prompt)
        .allow_empty(true)
        .interact_text()
        .context("Failed to read custom labels")?;

    if is_back_command(&result) {
        return Ok(PromptResult::Back);
    }

    let labels: Vec<String> = result
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(PromptResult::Value(labels))
}

fn prompt_description(
    theme: &ColorfulTheme,
    current: Option<&str>,
) -> Result<PromptResult<Option<String>>> {
    let prompt = format!(
        "{} Description? (y=editor, n=skip, <=back)",
        style("[6/6]").dim()
    );

    let choices = if current.is_some() {
        &["Edit in editor", "Keep current", "Clear", "Go back"][..]
    } else {
        &["Open editor", "Skip", "Go back"][..]
    };

    let selection = Select::with_theme(theme)
        .with_prompt(&prompt)
        .items(choices)
        .default(if current.is_some() { 1 } else { 1 })
        .interact_opt()
        .context("Failed to read description choice")?;

    match selection {
        None => Ok(PromptResult::Back), // Esc
        Some(idx) => {
            if current.is_some() {
                match idx {
                    0 => {
                        // Edit in editor
                        let edited = Editor::new()
                            .edit(current.unwrap_or(""))
                            .context("Failed to open editor")?;
                        Ok(PromptResult::Value(edited.filter(|s| !s.trim().is_empty())))
                    }
                    1 => Ok(PromptResult::Value(current.map(String::from))), // Keep
                    2 => Ok(PromptResult::Value(None)),                       // Clear
                    3 => Ok(PromptResult::Back),
                    _ => Ok(PromptResult::Value(None)),
                }
            } else {
                match idx {
                    0 => {
                        // Open editor
                        println!(
                            "  {}",
                            style("Opening editor... (save and close to continue)").dim()
                        );
                        let edited = Editor::new()
                            .edit("")
                            .context("Failed to open editor")?;
                        Ok(PromptResult::Value(edited.filter(|s| !s.trim().is_empty())))
                    }
                    1 => Ok(PromptResult::Value(None)), // Skip
                    2 => Ok(PromptResult::Back),
                    _ => Ok(PromptResult::Value(None)),
                }
            }
        }
    }
}

fn prompt_confirm(theme: &ColorfulTheme, input: &NewItemInput) -> Result<PromptResult<bool>> {
    println!();
    println!("{}", style("  ┌─ Summary ─────────────────────────────").dim());
    println!("  │ Title:    {}", style(&input.title).green());
    println!("  │ State:    {}", style(&input.state).yellow());
    if let Some(assignee) = &input.assignee {
        println!("  │ Assignee: {}", style(assignee).blue());
    }
    if !input.labels.is_empty() {
        println!(
            "  │ Labels:   {}",
            style(input.labels.join(", ")).magenta()
        );
    }
    if let Some(desc) = &input.description {
        let preview = if desc.len() > 50 {
            format!("{}...", &desc[..47])
        } else {
            desc.clone()
        };
        println!("  │ Desc:     {}", style(preview).dim());
    }
    println!("{}", style("  └─────────────────────────────────────────").dim());
    println!();

    let choices = &["Create", "Edit (go back to title)", "Cancel"];

    let selection = Select::with_theme(theme)
        .with_prompt("Action")
        .items(choices)
        .default(0)
        .interact_opt()
        .context("Failed to read confirmation")?;

    match selection {
        None => Ok(PromptResult::Cancel),
        Some(0) => Ok(PromptResult::Value(true)),  // Create
        Some(1) => Ok(PromptResult::Value(false)), // Edit
        Some(2) => Ok(PromptResult::Cancel),       // Cancel
        _ => Ok(PromptResult::Cancel),
    }
}

