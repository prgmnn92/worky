//! Workspace configuration.

use serde::{Deserialize, Serialize};

/// Workspace configuration stored in `.worky/config.yml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Configuration version.
    #[serde(default = "default_version")]
    pub version: u32,

    /// Workspace settings.
    #[serde(default)]
    pub workspace: WorkspaceSettings,

    /// Default values for new items.
    #[serde(default)]
    pub defaults: ItemDefaults,
}

fn default_version() -> u32 {
    1
}

/// Workspace-level settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Workspace name.
    #[serde(default)]
    pub name: Option<String>,
}

/// Default values for new work items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDefaults {
    /// Default state for new items.
    #[serde(default = "default_state")]
    pub state: String,

    /// Default labels for new items.
    #[serde(default)]
    pub labels: Vec<String>,
}

fn default_state() -> String {
    "TODO".to_string()
}

impl Default for ItemDefaults {
    fn default() -> Self {
        Self {
            state: default_state(),
            labels: Vec::new(),
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            version: 1,
            workspace: WorkspaceSettings::default(),
            defaults: ItemDefaults::default(),
        }
    }
}

impl WorkspaceConfig {
    /// Create a new config with the given workspace name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            workspace: WorkspaceSettings {
                name: Some(name.into()),
            },
            ..Default::default()
        }
    }
}
