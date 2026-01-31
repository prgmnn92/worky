//! Patch operations for work item updates.
//!
//! Supports:
//! - Path-based set operations (e.g., `state=IN_PROGRESS`, `fields.priority=high`)
//! - JSON Merge Patch (RFC 7396) for complex updates

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A single set operation (path = value).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetOperation {
    /// Dot-separated path (e.g., "fields.priority" or "state").
    pub path: String,
    /// Value to set.
    pub value: Value,
}

impl SetOperation {
    /// Create a new set operation.
    #[must_use]
    pub fn new(path: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            path: path.into(),
            value: value.into(),
        }
    }

    /// Parse a "key=value" string into a SetOperation.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidPath` if the format is invalid.
    pub fn parse(input: &str) -> Result<Self> {
        let (path, value) = input
            .split_once('=')
            .ok_or_else(|| CoreError::InvalidPath(format!("expected 'key=value', got '{input}'")))?;

        let path = path.trim();
        let value_str = value.trim();

        // Try to parse as JSON first, fall back to string
        let value = serde_json::from_str(value_str).unwrap_or_else(|_| Value::String(value_str.to_string()));

        Ok(Self::new(path, value))
    }
}

/// Resolve a dot-separated path to a JSON pointer.
///
/// Examples:
/// - `state` → `/state`
/// - `fields.priority` → `/fields/priority`
/// - `fields.System.IterationPath` → `/fields/System/IterationPath`
#[must_use]
pub fn resolve_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    format!("/{}", path.replace('.', "/"))
}

/// Get a mutable reference to a value at a dot-separated path, creating intermediate objects if needed.
fn get_or_create_path<'a>(root: &'a mut Value, path: &str) -> Result<&'a mut Value> {
    if path.is_empty() {
        return Ok(root);
    }

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for (i, part) in parts.iter().enumerate() {
        if !current.is_object() {
            return Err(CoreError::InvalidPath(format!(
                "cannot traverse into non-object at '{}'",
                parts[..i].join(".")
            )));
        }

        let obj = current.as_object_mut().unwrap();

        // For the last part, we want to return a mutable reference
        if i == parts.len() - 1 {
            // Insert null if not present, then return reference
            if !obj.contains_key(*part) {
                obj.insert((*part).to_string(), Value::Null);
            }
            return Ok(obj.get_mut(*part).unwrap());
        }

        // For intermediate parts, create object if not present
        if !obj.contains_key(*part) {
            obj.insert((*part).to_string(), Value::Object(Map::new()));
        }

        current = obj.get_mut(*part).unwrap();
    }

    Ok(current)
}

/// Apply a set operation to a JSON value.
///
/// # Errors
/// Returns `CoreError::InvalidPath` if the path cannot be resolved.
pub fn apply_set_operation(root: &mut Value, op: &SetOperation) -> Result<Option<Value>> {
    let target = get_or_create_path(root, &op.path)?;
    let old_value = if target.is_null() {
        None
    } else {
        Some(target.clone())
    };
    *target = op.value.clone();
    Ok(old_value)
}

/// Apply a JSON Merge Patch (RFC 7396) to a value.
///
/// Rules:
/// - If patch is null, replace target with null
/// - If patch is not an object, replace target with patch
/// - If patch is an object, merge recursively
///   - null values in patch remove keys from target
///   - other values replace/add keys
pub fn apply_merge_patch(target: &mut Value, patch: &Value) {
    if !patch.is_object() {
        *target = patch.clone();
        return;
    }

    if !target.is_object() {
        *target = Value::Object(Map::new());
    }

    let target_obj = target.as_object_mut().unwrap();
    let patch_obj = patch.as_object().unwrap();

    for (key, value) in patch_obj {
        if value.is_null() {
            target_obj.remove(key);
        } else if value.is_object() {
            // Recursively merge
            let target_value = target_obj
                .entry(key.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            apply_merge_patch(target_value, value);
        } else {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

/// Detect changes between two values and return the differences.
#[must_use]
pub fn diff_values(old: &Value, new: &Value) -> Vec<(String, Option<Value>, Value)> {
    let mut changes = Vec::new();
    diff_recursive(old, new, String::new(), &mut changes);
    changes
}

fn diff_recursive(
    old: &Value,
    new: &Value,
    path: String,
    changes: &mut Vec<(String, Option<Value>, Value)>,
) {
    if old == new {
        return;
    }

    match (old, new) {
        (Value::Object(old_obj), Value::Object(new_obj)) => {
            // Check for changed and added keys
            for (key, new_val) in new_obj {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                match old_obj.get(key) {
                    Some(old_val) => diff_recursive(old_val, new_val, child_path, changes),
                    None => changes.push((child_path, None, new_val.clone())),
                }
            }

            // Check for removed keys
            for (key, old_val) in old_obj {
                if !new_obj.contains_key(key) {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    changes.push((child_path, Some(old_val.clone()), Value::Null));
                }
            }
        }
        _ => {
            // Leaf value changed
            let old_val = if *old == Value::Null {
                None
            } else {
                Some(old.clone())
            };
            changes.push((path, old_val, new.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_path() {
        assert_eq!(resolve_path("state"), "/state");
        assert_eq!(resolve_path("fields.priority"), "/fields/priority");
        assert_eq!(
            resolve_path("fields.System.IterationPath"),
            "/fields/System/IterationPath"
        );
        assert_eq!(resolve_path(""), "");
    }

    #[test]
    fn test_set_operation_parse() {
        let op = SetOperation::parse("state=IN_PROGRESS").unwrap();
        assert_eq!(op.path, "state");
        assert_eq!(op.value, Value::String("IN_PROGRESS".to_string()));

        let op = SetOperation::parse("fields.priority=high").unwrap();
        assert_eq!(op.path, "fields.priority");
        assert_eq!(op.value, Value::String("high".to_string()));

        // JSON value
        let op = SetOperation::parse("count=42").unwrap();
        assert_eq!(op.value, json!(42));

        let op = SetOperation::parse("active=true").unwrap();
        assert_eq!(op.value, json!(true));
    }

    #[test]
    fn test_apply_set_operation() {
        let mut value = json!({
            "state": "TODO",
            "fields": {}
        });

        let op = SetOperation::new("state", "IN_PROGRESS");
        let old = apply_set_operation(&mut value, &op).unwrap();
        assert_eq!(old, Some(json!("TODO")));
        assert_eq!(value["state"], json!("IN_PROGRESS"));

        // Nested path
        let op = SetOperation::new("fields.priority", "high");
        let old = apply_set_operation(&mut value, &op).unwrap();
        assert!(old.is_none() || old == Some(Value::Null));
        assert_eq!(value["fields"]["priority"], json!("high"));

        // Deep nested path (auto-create intermediate objects)
        let op = SetOperation::new("fields.System.IterationPath", "Sprint 1");
        apply_set_operation(&mut value, &op).unwrap();
        assert_eq!(value["fields"]["System"]["IterationPath"], json!("Sprint 1"));
    }

    #[test]
    fn test_apply_merge_patch() {
        let mut target = json!({
            "title": "Original",
            "state": "TODO",
            "fields": {
                "priority": "low"
            }
        });

        let patch = json!({
            "state": "IN_PROGRESS",
            "fields": {
                "priority": "high",
                "blocked": true
            }
        });

        apply_merge_patch(&mut target, &patch);

        assert_eq!(target["title"], json!("Original")); // unchanged
        assert_eq!(target["state"], json!("IN_PROGRESS")); // updated
        assert_eq!(target["fields"]["priority"], json!("high")); // updated
        assert_eq!(target["fields"]["blocked"], json!(true)); // added
    }

    #[test]
    fn test_merge_patch_removes_null() {
        let mut target = json!({
            "a": "keep",
            "b": "remove"
        });

        let patch = json!({
            "b": null,
            "c": "add"
        });

        apply_merge_patch(&mut target, &patch);

        assert_eq!(target["a"], json!("keep"));
        assert!(target.get("b").is_none());
        assert_eq!(target["c"], json!("add"));
    }

    #[test]
    fn test_diff_values() {
        let old = json!({
            "state": "TODO",
            "assignee": "alice",
            "fields": {
                "priority": "low"
            }
        });

        let new = json!({
            "state": "IN_PROGRESS",
            "fields": {
                "priority": "high",
                "blocked": true
            }
        });

        let changes = diff_values(&old, &new);

        assert!(changes.iter().any(|(p, _, v)| p == "state" && *v == json!("IN_PROGRESS")));
        assert!(changes.iter().any(|(p, _, _)| p == "assignee")); // removed
        assert!(changes.iter().any(|(p, _, v)| p == "fields.priority" && *v == json!("high")));
        assert!(changes.iter().any(|(p, _, v)| p == "fields.blocked" && *v == json!(true)));
    }
}
