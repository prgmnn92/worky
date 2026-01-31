//! Work event model for append-only change tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Type of event that occurred on a work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Item was created.
    Created,
    /// State changed.
    StateChanged,
    /// Field value changed.
    FieldChanged,
    /// Comment or note added.
    CommentAdded,
    /// Label attached.
    LabelAdded,
    /// Label removed.
    LabelRemoved,
    /// Assignee changed.
    Assigned,
    /// Action performed by AI tool.
    AiAction,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "CREATED"),
            Self::StateChanged => write!(f, "STATE_CHANGED"),
            Self::FieldChanged => write!(f, "FIELD_CHANGED"),
            Self::CommentAdded => write!(f, "COMMENT_ADDED"),
            Self::LabelAdded => write!(f, "LABEL_ADDED"),
            Self::LabelRemoved => write!(f, "LABEL_REMOVED"),
            Self::Assigned => write!(f, "ASSIGNED"),
            Self::AiAction => write!(f, "AI_ACTION"),
        }
    }
}

/// State change payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StateChangePayload {
    pub from: String,
    pub to: String,
}

/// Field change payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FieldChangePayload {
    pub path: String,
    pub old_value: Option<Value>,
    pub new_value: Value,
}

/// Assignee change payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssigneeChangePayload {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Label payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabelPayload {
    pub label: String,
}

/// Comment payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CommentPayload {
    pub message: String,
}

/// AI action payload data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AiActionPayload {
    pub tool: String,
    pub action: String,
    pub details: Option<Value>,
}

/// Payload for different event types.
///
/// Note: With `#[serde(untagged)]`, variants are tried in order during deserialization.
/// Each payload struct uses `deny_unknown_fields` to ensure precise matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EventPayload {
    /// State change payload.
    StateChange(StateChangePayload),
    /// Field change payload.
    FieldChange(FieldChangePayload),
    /// Label operation payload (must come before AssigneeChange which has optional fields).
    Label(LabelPayload),
    /// Comment payload (must come before AssigneeChange which has optional fields).
    Comment(CommentPayload),
    /// AI action payload.
    AiAction(AiActionPayload),
    /// Assignee change payload (has optional fields, so must come last among structs).
    AssigneeChange(AssigneeChangePayload),
    /// Generic payload for extensibility.
    Generic(Value),
}

/// A single event in the work item's history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkEvent {
    /// Unique event identifier.
    pub id: String,

    /// Type of event.
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// When the event occurred (ISO 8601 UTC).
    pub timestamp: DateTime<Utc>,

    /// Who or what caused this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    /// Event-specific payload.
    pub payload: EventPayload,
}

impl WorkEvent {
    /// Create a new event with auto-generated ID and current timestamp.
    #[must_use]
    pub fn new(event_type: EventType, payload: EventPayload) -> Self {
        Self {
            id: format!("evt_{}", Uuid::new_v4().as_simple()),
            event_type,
            timestamp: Utc::now(),
            actor: None,
            payload,
        }
    }

    /// Set the actor for this event.
    #[must_use]
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Create a CREATED event.
    #[must_use]
    pub fn created(title: &str) -> Self {
        Self::new(
            EventType::Created,
            EventPayload::Comment(CommentPayload {
                message: format!("Created: {title}"),
            }),
        )
    }

    /// Create a STATE_CHANGED event.
    #[must_use]
    pub fn state_changed(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::new(
            EventType::StateChanged,
            EventPayload::StateChange(StateChangePayload {
                from: from.into(),
                to: to.into(),
            }),
        )
    }

    /// Create a FIELD_CHANGED event.
    #[must_use]
    pub fn field_changed(
        path: impl Into<String>,
        old_value: Option<Value>,
        new_value: Value,
    ) -> Self {
        Self::new(
            EventType::FieldChanged,
            EventPayload::FieldChange(FieldChangePayload {
                path: path.into(),
                old_value,
                new_value,
            }),
        )
    }

    /// Create an ASSIGNED event.
    #[must_use]
    pub fn assigned(from: Option<String>, to: Option<String>) -> Self {
        Self::new(
            EventType::Assigned,
            EventPayload::AssigneeChange(AssigneeChangePayload { from, to }),
        )
    }

    /// Create a LABEL_ADDED event.
    #[must_use]
    pub fn label_added(label: impl Into<String>) -> Self {
        Self::new(
            EventType::LabelAdded,
            EventPayload::Label(LabelPayload {
                label: label.into(),
            }),
        )
    }

    /// Create a LABEL_REMOVED event.
    #[must_use]
    pub fn label_removed(label: impl Into<String>) -> Self {
        Self::new(
            EventType::LabelRemoved,
            EventPayload::Label(LabelPayload {
                label: label.into(),
            }),
        )
    }

    /// Create a COMMENT_ADDED event.
    #[must_use]
    pub fn comment(message: impl Into<String>) -> Self {
        Self::new(
            EventType::CommentAdded,
            EventPayload::Comment(CommentPayload {
                message: message.into(),
            }),
        )
    }

    /// Create an AI_ACTION event.
    #[must_use]
    pub fn ai_action(tool: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(
            EventType::AiAction,
            EventPayload::AiAction(AiActionPayload {
                tool: tool.into(),
                action: action.into(),
                details: None,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_changed_event() {
        let event = WorkEvent::state_changed("TODO", "IN_PROGRESS").with_actor("alice");

        assert_eq!(event.event_type, EventType::StateChanged);
        assert_eq!(event.actor, Some("alice".to_string()));

        if let EventPayload::StateChange(StateChangePayload { from, to }) = &event.payload {
            assert_eq!(from, "TODO");
            assert_eq!(to, "IN_PROGRESS");
        } else {
            panic!("Expected StateChange payload");
        }
    }

    #[test]
    fn test_event_serialization() {
        let event = WorkEvent::state_changed("TODO", "DONE");
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains(r#""type":"STATE_CHANGED""#));
        assert!(json.contains(r#""from":"TODO""#));
        assert!(json.contains(r#""to":"DONE""#));
    }

    #[test]
    fn test_ai_action_event() {
        let event = WorkEvent::ai_action("worky-tool", "set_state").with_actor("claude");

        assert_eq!(event.event_type, EventType::AiAction);
        assert_eq!(event.actor, Some("claude".to_string()));
    }
}
