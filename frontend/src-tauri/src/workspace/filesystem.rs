use std::fs;
use std::path::{Path, PathBuf};

use super::types::{WorkspaceEntry, WorkspaceManifest, WorkspaceRegistry};

/// Create a workspace directory with standard subdirectories.
///
/// Creates `{workspaces_root}/{workspace_id}/` with `audio/` and `notes/` subdirs.
/// Returns the workspace directory path.
pub fn create_workspace_dir(workspaces_root: &Path, workspace_id: &str) -> Result<PathBuf, String> {
    let workspace_dir = workspaces_root.join(workspace_id);

    fs::create_dir_all(workspace_dir.join("audio"))
        .map_err(|e| format!("Failed to create workspace audio dir: {}", e))?;
    fs::create_dir_all(workspace_dir.join("notes"))
        .map_err(|e| format!("Failed to create workspace notes dir: {}", e))?;

    log::info!("Created workspace directory: {:?}", workspace_dir);
    Ok(workspace_dir)
}

/// Write a manifest to `{workspace_dir}/manifest.json`.
pub fn write_manifest(workspace_dir: &Path, manifest: &WorkspaceManifest) -> Result<(), String> {
    let manifest_path = workspace_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_path, json)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;
    log::info!("Wrote manifest to {:?}", manifest_path);
    Ok(())
}

/// Read a manifest from `{workspace_dir}/manifest.json`.
pub fn read_manifest(workspace_dir: &Path) -> Result<WorkspaceManifest, String> {
    let manifest_path = workspace_dir.join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest at {:?}: {}", manifest_path, e))?;
    let manifest: WorkspaceManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;
    Ok(manifest)
}

/// Write a default empty config to `{workspace_dir}/config.json`.
pub fn write_default_config(workspace_dir: &Path) -> Result<(), String> {
    let config_path = workspace_dir.join("config.json");
    fs::write(&config_path, "{}")
        .map_err(|e| format!("Failed to write default config: {}", e))?;
    Ok(())
}

/// Save the workspace registry to `{workspaces_root}/workspaces.json` using atomic write.
pub fn save_registry(workspaces_root: &Path, registry: &WorkspaceRegistry) -> Result<(), String> {
    let registry_path = workspaces_root.join("workspaces.json");
    let tmp_path = workspaces_root.join("workspaces.json.tmp");

    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| format!("Failed to serialize registry: {}", e))?;
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write temporary registry: {}", e))?;
    fs::rename(&tmp_path, &registry_path)
        .map_err(|e| format!("Failed to rename registry file: {}", e))?;

    log::info!("Saved workspace registry with {} entries", registry.workspaces.len());
    Ok(())
}

/// Load the workspace registry from `{workspaces_root}/workspaces.json`.
///
/// Returns a default empty registry if the file doesn't exist.
pub fn load_registry(workspaces_root: &Path) -> Result<WorkspaceRegistry, String> {
    let registry_path = workspaces_root.join("workspaces.json");

    if !registry_path.exists() {
        log::info!("No registry file found, returning default empty registry");
        return Ok(WorkspaceRegistry::default());
    }

    let content = fs::read_to_string(&registry_path)
        .map_err(|e| format!("Failed to read registry: {}", e))?;
    let registry: WorkspaceRegistry = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse registry: {}", e))?;

    log::info!("Loaded workspace registry with {} entries", registry.workspaces.len());
    Ok(registry)
}

/// Rebuild the workspace registry by scanning the workspaces root directory.
///
/// Scans for directories containing `manifest.json` with valid UUID folder names.
/// Useful for self-healing if `workspaces.json` gets corrupted.
pub fn rebuild_registry_from_disk(workspaces_root: &Path) -> Result<WorkspaceRegistry, String> {
    let mut entries = Vec::new();

    let dir_entries = fs::read_dir(workspaces_root)
        .map_err(|e| format!("Failed to read workspaces root: {}", e))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        // Check if folder name is a valid UUID
        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if uuid::Uuid::parse_str(&folder_name).is_err() {
            continue;
        }

        // Check if manifest.json exists
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            log::warn!("UUID directory {:?} has no manifest.json, skipping", path);
            continue;
        }

        // Try to read the manifest
        match read_manifest(&path) {
            Ok(manifest) => {
                entries.push(WorkspaceEntry {
                    id: folder_name,
                    name: manifest.name,
                    icon: manifest.icon,
                });
            }
            Err(e) => {
                log::warn!("Failed to read manifest in {:?}: {}", path, e);
            }
        }
    }

    log::info!("Rebuilt registry from disk: found {} workspaces", entries.len());

    Ok(WorkspaceRegistry {
        version: 1,
        workspaces: entries,
        last_active: None,
    })
}
