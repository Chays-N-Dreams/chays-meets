use serde::{Deserialize, Serialize};

/// Per-workspace metadata stored in `manifest.json` inside each workspace folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// Schema version for manifest format (start at 1)
    pub version: u32,
    /// Human-readable display name
    pub name: String,
    /// Optional emoji icon (e.g., "🏢")
    pub icon: Option<String>,
    /// Optional hex accent color (e.g., "#3B82F6")
    pub accent_color: Option<String>,
    /// User description of workspace purpose
    pub description: Option<String>,
    /// Meetily version that created this workspace (debugging aid)
    pub app_version: Option<String>,
    /// ISO 8601 timestamp of creation
    pub created_at: String,
    /// ISO 8601 timestamp of last metadata change
    pub last_modified: String,
}

/// Cached entry in the global registry for fast sidebar rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// UUID string matching the workspace folder name
    pub id: String,
    /// Cached display name from manifest
    pub name: String,
    /// Cached emoji icon from manifest
    pub icon: Option<String>,
}

/// Global registry file (`workspaces.json`) at the workspaces root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistry {
    /// Schema version for registry format (start at 1)
    pub version: u32,
    /// Ordered list of workspace entries (order = sidebar display order)
    pub workspaces: Vec<WorkspaceEntry>,
    /// UUID of last active workspace for restore on launch
    pub last_active: Option<String>,
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self {
            version: 1,
            workspaces: Vec::new(),
            last_active: None,
        }
    }
}
