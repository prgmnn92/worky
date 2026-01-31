//! Workspace management and work item operations.

use crate::config::WorkspaceConfig;
use crate::error::{FsError, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use slug::slugify;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use worky_core::{
    apply_merge_patch, apply_set_operation, diff_values, SetOperation, WorkEvent, WorkItem,
};

/// Directory name for worky configuration.
const WORKY_DIR: &str = ".worky";
/// Configuration file name.
const CONFIG_FILE: &str = "config.yml";
/// Items directory name.
const ITEMS_DIR: &str = "work/items";
/// Meta file name within item directory.
const META_FILE: &str = "meta.yml";
/// Events file name within item directory.
const EVENTS_FILE: &str = "events.ndjson";
/// Notes file name within item directory.
const NOTES_FILE: &str = "notes.md";

/// A workspace manages work items on the filesystem.
#[derive(Debug)]
pub struct Workspace {
    /// Root path of the workspace.
    root: PathBuf,
    /// Workspace configuration.
    config: WorkspaceConfig,
}

impl Workspace {
    /// Initialize a new workspace at the given path.
    ///
    /// # Errors
    /// Returns error if workspace already exists or IO fails.
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let worky_dir = root.join(WORKY_DIR);

        if worky_dir.exists() {
            return Err(FsError::WorkspaceExists(root));
        }

        // Create directory structure
        fs::create_dir_all(&worky_dir)?;
        fs::create_dir_all(root.join(ITEMS_DIR))?;

        // Write default config
        let config = WorkspaceConfig::default();
        let config_path = worky_dir.join(CONFIG_FILE);
        let config_content = serde_yaml::to_string(&config)?;
        fs::write(&config_path, config_content)?;

        info!(path = %root.display(), "Initialized workspace");

        Ok(Self { root, config })
    }

    /// Open an existing workspace at the given path.
    ///
    /// # Errors
    /// Returns error if workspace doesn't exist or config is invalid.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let worky_dir = root.join(WORKY_DIR);
        let config_path = worky_dir.join(CONFIG_FILE);

        if !config_path.exists() {
            return Err(FsError::WorkspaceNotFound(root));
        }

        let config_content = fs::read_to_string(&config_path)?;
        let config: WorkspaceConfig = serde_yaml::from_str(&config_content)?;

        debug!(path = %root.display(), "Opened workspace");

        Ok(Self { root, config })
    }

    /// Get the workspace root path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the workspace configuration.
    #[must_use]
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    /// Get the items directory path.
    fn items_dir(&self) -> PathBuf {
        self.root.join(ITEMS_DIR)
    }

    /// Get the path to an item's directory.
    fn item_dir(&self, slug: &str) -> PathBuf {
        self.items_dir().join(slug)
    }

    /// Extract slug from a UID.
    fn slug_from_uid(uid: &str) -> Result<&str> {
        uid.strip_prefix("fs:")
            .ok_or_else(|| FsError::InvalidUid(uid.to_string()))
    }

    /// Generate a slug from a title.
    fn generate_slug(title: &str) -> String {
        slugify(title)
    }

    /// Create a new work item.
    ///
    /// # Errors
    /// Returns error if item already exists or IO fails.
    pub fn create_item(&self, title: impl Into<String>) -> Result<WorkItem> {
        let title = title.into();
        let slug = Self::generate_slug(&title);
        let uid = format!("fs:{slug}");

        let item_dir = self.item_dir(&slug);
        if item_dir.exists() {
            return Err(FsError::ItemExists(uid));
        }

        // Create item directory
        fs::create_dir_all(&item_dir)?;
        fs::create_dir_all(item_dir.join("artifacts"))?;

        // Create work item
        let item = WorkItem::new(&uid, &title).with_state(&self.config.defaults.state);

        // Write meta.yml
        self.write_meta(&slug, &item)?;

        // Write empty notes.md
        fs::write(item_dir.join(NOTES_FILE), format!("# {title}\n\n"))?;

        // Append CREATED event
        let event = WorkEvent::created(&title);
        self.append_event(&slug, &event)?;

        info!(uid = %uid, title = %title, "Created work item");

        Ok(item)
    }

    /// Get a work item by UID.
    ///
    /// # Errors
    /// Returns error if item doesn't exist or meta is invalid.
    pub fn get_item(&self, uid: &str) -> Result<WorkItem> {
        let slug = Self::slug_from_uid(uid)?;
        let item_dir = self.item_dir(slug);

        if !item_dir.exists() {
            return Err(FsError::ItemNotFound(uid.to_string()));
        }

        self.read_meta(slug)
    }

    /// List all work items, optionally filtered.
    pub fn list_items(&self, filter: Option<&ItemFilter>) -> Result<Vec<WorkItem>> {
        let items_dir = self.items_dir();
        if !items_dir.exists() {
            return Ok(Vec::new());
        }

        let mut items = Vec::new();

        for entry in fs::read_dir(&items_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let slug = entry.file_name().to_string_lossy().to_string();
            let meta_path = entry.path().join(META_FILE);

            if !meta_path.exists() {
                continue;
            }

            match self.read_meta(&slug) {
                Ok(item) => {
                    if filter.is_none_or(|f| f.matches(&item)) {
                        items.push(item);
                    }
                }
                Err(e) => {
                    debug!(slug = %slug, error = %e, "Failed to read item, skipping");
                }
            }
        }

        // Sort by updated_at descending
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(items)
    }

    /// Update a work item with set operations.
    ///
    /// # Errors
    /// Returns error if item doesn't exist or update fails.
    pub fn update_item(&self, uid: &str, operations: &[SetOperation]) -> Result<WorkItem> {
        let slug = Self::slug_from_uid(uid)?;
        let mut item = self.read_meta(slug)?;
        let old_item = item.clone();

        // Convert to JSON for patching
        let mut json_value = serde_json::to_value(&item)?;

        for op in operations {
            apply_set_operation(&mut json_value, op)?;
        }

        // Deserialize back
        item = serde_json::from_value(json_value)?;
        item.touch();

        // Generate events for changes
        let old_json = serde_json::to_value(&old_item)?;
        let new_json = serde_json::to_value(&item)?;
        let changes = diff_values(&old_json, &new_json);

        for (path, old_val, new_val) in changes {
            // Skip updated_at changes
            if path == "updated_at" {
                continue;
            }

            let event = if path == "state" {
                WorkEvent::state_changed(
                    old_val
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                    new_val.as_str().unwrap_or(""),
                )
            } else if path == "assignee" {
                WorkEvent::assigned(
                    old_val.as_ref().and_then(|v| v.as_str()).map(String::from),
                    new_val.as_str().map(String::from),
                )
            } else {
                WorkEvent::field_changed(path, old_val, new_val)
            };

            self.append_event(slug, &event)?;
        }

        // Write updated meta
        self.write_meta(slug, &item)?;

        info!(uid = %uid, "Updated work item");

        Ok(item)
    }

    /// Apply a JSON merge patch to a work item.
    ///
    /// # Errors
    /// Returns error if item doesn't exist or patch fails.
    pub fn patch_item(&self, uid: &str, patch: &Value) -> Result<WorkItem> {
        let slug = Self::slug_from_uid(uid)?;
        let item = self.read_meta(slug)?;

        let old_json = serde_json::to_value(&item)?;
        let mut new_json = old_json.clone();
        apply_merge_patch(&mut new_json, patch);

        // Ensure updated_at is refreshed
        new_json["updated_at"] = serde_json::to_value(Utc::now())?;

        let new_item: WorkItem = serde_json::from_value(new_json.clone())?;

        // Generate events for changes
        let changes = diff_values(&old_json, &new_json);

        for (path, old_val, new_val) in changes {
            if path == "updated_at" {
                continue;
            }

            let event = if path == "state" {
                WorkEvent::state_changed(
                    old_val
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                    new_val.as_str().unwrap_or(""),
                )
            } else {
                WorkEvent::field_changed(path, old_val, new_val)
            };

            self.append_event(slug, &event)?;
        }

        self.write_meta(slug, &new_item)?;

        info!(uid = %uid, "Patched work item");

        Ok(new_item)
    }

    /// Append an event to an item's event log.
    pub fn append_event(&self, slug: &str, event: &WorkEvent) -> Result<()> {
        let events_path = self.item_dir(slug).join(EVENTS_FILE);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;

        let json_line = serde_json::to_string(event)?;
        writeln!(file, "{json_line}")?;

        debug!(slug = %slug, event_type = %event.event_type, "Appended event");

        Ok(())
    }

    /// Read events for an item, optionally filtered by time.
    pub fn read_events(&self, uid: &str, since: Option<DateTime<Utc>>) -> Result<Vec<WorkEvent>> {
        let slug = Self::slug_from_uid(uid)?;
        let events_path = self.item_dir(slug).join(EVENTS_FILE);

        if !events_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&events_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let event: WorkEvent = serde_json::from_str(&line)?;

            if since.is_none_or(|s| event.timestamp >= s) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Add a comment/log entry to an item.
    pub fn add_comment(&self, uid: &str, message: impl Into<String>) -> Result<()> {
        let slug = Self::slug_from_uid(uid)?;

        if !self.item_dir(slug).exists() {
            return Err(FsError::ItemNotFound(uid.to_string()));
        }

        let event = WorkEvent::comment(message);
        self.append_event(slug, &event)?;

        Ok(())
    }

    // Private helpers

    fn read_meta(&self, slug: &str) -> Result<WorkItem> {
        let meta_path = self.item_dir(slug).join(META_FILE);
        let content = fs::read_to_string(&meta_path)?;
        let item: WorkItem = serde_yaml::from_str(&content)?;
        Ok(item)
    }

    fn write_meta(&self, slug: &str, item: &WorkItem) -> Result<()> {
        let meta_path = self.item_dir(slug).join(META_FILE);
        let content = serde_yaml::to_string(item)?;
        fs::write(&meta_path, content)?;
        Ok(())
    }
}

/// Filter criteria for listing work items.
#[derive(Debug, Default)]
pub struct ItemFilter {
    /// Filter by state.
    pub state: Option<String>,
    /// Filter by assignee.
    pub assignee: Option<String>,
    /// Filter by label (item must have this label).
    pub label: Option<String>,
}

impl ItemFilter {
    /// Check if an item matches this filter.
    #[must_use]
    pub fn matches(&self, item: &WorkItem) -> bool {
        if let Some(state) = &self.state {
            if !item.state.eq_ignore_ascii_case(state) {
                return false;
            }
        }

        if let Some(assignee) = &self.assignee {
            match &item.assignee {
                Some(a) if a.eq_ignore_ascii_case(assignee) => {}
                _ => return false,
            }
        }

        if let Some(label) = &self.label {
            if !item.has_label(label) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Workspace) {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::init(tmp.path()).unwrap();
        (tmp, ws)
    }

    #[test]
    fn test_init_workspace() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::init(tmp.path()).unwrap();

        assert!(tmp.path().join(".worky/config.yml").exists());
        assert!(tmp.path().join("work/items").exists());
        assert_eq!(ws.config().version, 1);
    }

    #[test]
    fn test_init_existing_fails() {
        let tmp = TempDir::new().unwrap();
        Workspace::init(tmp.path()).unwrap();

        let result = Workspace::init(tmp.path());
        assert!(matches!(result, Err(FsError::WorkspaceExists(_))));
    }

    #[test]
    fn test_open_workspace() {
        let tmp = TempDir::new().unwrap();
        Workspace::init(tmp.path()).unwrap();

        let ws = Workspace::open(tmp.path()).unwrap();
        assert_eq!(ws.config().version, 1);
    }

    #[test]
    fn test_create_and_get_item() {
        let (_tmp, ws) = setup();

        let item = ws.create_item("Implement auth redirect").unwrap();
        assert_eq!(item.uid, "fs:implement-auth-redirect");
        assert_eq!(item.title, "Implement auth redirect");
        assert_eq!(item.state, "TODO");

        let fetched = ws.get_item("fs:implement-auth-redirect").unwrap();
        assert_eq!(fetched.uid, item.uid);
        assert_eq!(fetched.title, item.title);
    }

    #[test]
    fn test_update_item() {
        let (_tmp, ws) = setup();

        ws.create_item("Test task").unwrap();

        let ops = vec![
            SetOperation::new("state", "IN_PROGRESS"),
            SetOperation::new("assignee", "alice"),
        ];

        let updated = ws.update_item("fs:test-task", &ops).unwrap();
        assert_eq!(updated.state, "IN_PROGRESS");
        assert_eq!(updated.assignee, Some("alice".to_string()));
    }

    #[test]
    fn test_list_items() {
        let (_tmp, ws) = setup();

        ws.create_item("Task 1").unwrap();
        ws.create_item("Task 2").unwrap();

        let items = ws.list_items(None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_with_filter() {
        let (_tmp, ws) = setup();

        ws.create_item("Task 1").unwrap();
        ws.update_item(
            "fs:task-1",
            &[SetOperation::new("state", "IN_PROGRESS")],
        )
        .unwrap();
        ws.create_item("Task 2").unwrap();

        let filter = ItemFilter {
            state: Some("IN_PROGRESS".to_string()),
            ..Default::default()
        };

        let items = ws.list_items(Some(&filter)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].state, "IN_PROGRESS");
    }

    #[test]
    fn test_events() {
        let (_tmp, ws) = setup();

        ws.create_item("Event test").unwrap();
        ws.update_item(
            "fs:event-test",
            &[SetOperation::new("state", "IN_PROGRESS")],
        )
        .unwrap();

        let events = ws.read_events("fs:event-test", None).unwrap();
        assert!(events.len() >= 2); // CREATED + STATE_CHANGED
    }
}
