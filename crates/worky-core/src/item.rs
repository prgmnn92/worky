//! Work item model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A work item representing a task, bug, feature, or other trackable unit of work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItem {
    /// Unique identifier (e.g., "fs:implement-auth-redirect").
    pub uid: String,

    /// Human-readable title.
    pub title: String,

    /// Current state (e.g., "TODO", "IN_PROGRESS", "DONE").
    pub state: String,

    /// Assigned person (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Categorization labels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Creation timestamp (ISO 8601 UTC).
    pub created_at: DateTime<Utc>,

    /// Last update timestamp (ISO 8601 UTC).
    pub updated_at: DateTime<Utc>,

    /// Custom fields as nested key-value pairs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, Value>,
}

impl WorkItem {
    /// Create a new work item with minimal required fields.
    #[must_use]
    pub fn new(uid: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            uid: uid.into(),
            title: title.into(),
            state: "TODO".to_string(),
            assignee: None,
            labels: Vec::new(),
            created_at: now,
            updated_at: now,
            fields: HashMap::new(),
        }
    }

    /// Create a new work item with a specific state.
    #[must_use]
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = state.into();
        self
    }

    /// Add an assignee to the work item.
    #[must_use]
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Add labels to the work item.
    #[must_use]
    pub fn with_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Set a custom field value.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Check if the item has a specific label.
    #[must_use]
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l.eq_ignore_ascii_case(label))
    }

    /// Add a label if not already present.
    pub fn add_label(&mut self, label: impl Into<String>) {
        let label = label.into();
        if !self.has_label(&label) {
            self.labels.push(label);
            self.touch();
        }
    }

    /// Remove a label if present.
    pub fn remove_label(&mut self, label: &str) -> bool {
        let initial_len = self.labels.len();
        self.labels.retain(|l| !l.eq_ignore_ascii_case(label));
        if self.labels.len() != initial_len {
            self.touch();
            true
        } else {
            false
        }
    }
}

impl Default for WorkItem {
    fn default() -> Self {
        Self::new("", "Untitled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_work_item() {
        let item = WorkItem::new("fs:test-item", "Test Item");

        assert_eq!(item.uid, "fs:test-item");
        assert_eq!(item.title, "Test Item");
        assert_eq!(item.state, "TODO");
        assert!(item.assignee.is_none());
        assert!(item.labels.is_empty());
        assert!(item.fields.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let item = WorkItem::new("fs:test", "Test")
            .with_state("IN_PROGRESS")
            .with_assignee("alice")
            .with_labels(["backend", "security"])
            .with_field("priority", "high");

        assert_eq!(item.state, "IN_PROGRESS");
        assert_eq!(item.assignee, Some("alice".to_string()));
        assert_eq!(item.labels, vec!["backend", "security"]);
        assert_eq!(item.fields.get("priority"), Some(&Value::from("high")));
    }

    #[test]
    fn test_label_operations() {
        let mut item = WorkItem::new("fs:test", "Test");

        item.add_label("backend");
        assert!(item.has_label("backend"));
        assert!(item.has_label("BACKEND")); // case-insensitive check

        item.add_label("backend"); // duplicate, should not add
        assert_eq!(item.labels.len(), 1);

        assert!(item.remove_label("BACKEND")); // case-insensitive remove
        assert!(!item.has_label("backend"));
        assert!(!item.remove_label("backend")); // already removed
    }
}
