use tauri::{AppHandle, Manager};

use crate::workspace::manager::WorkspaceManager;

/// Initialize the WorkspaceManager on app startup.
///
/// Creates workspace infrastructure, detects migration scenarios, and ensures
/// an active workspace is available before returning.
pub async fn initialize_workspace_manager(app: &AppHandle) -> Result<WorkspaceManager, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Step 1: Initialize WorkspaceManager infrastructure (creates workspaces root, global.sqlite, loads registry).
    // Returns with active_db: None -- no workspace is active yet.
    let workspace_mgr = WorkspaceManager::init(app_data_dir.clone()).await?;

    // Step 2: Detect migration scenario
    let existing_db = app_data_dir.join("meeting_minutes.sqlite");
    let workspaces = workspace_mgr.list_workspaces().await;

    // Step 3: Decision tree
    if existing_db.exists() && workspaces.is_empty() {
        // Case A: MIGRATION -- existing database found but no workspaces yet.
        // MIGRATION HOOK: Plan 04 will replace this with actual migration call.
        // For now, create an empty Default workspace so the app is usable during development.
        let default_id = workspace_mgr.create_workspace("Default".to_string()).await?;
        workspace_mgr.switch_workspace(&default_id).await?;
        log::warn!(
            "Migration placeholder: created empty Default workspace. Existing data not yet migrated."
        );
    } else if !workspaces.is_empty() {
        // Case B/C: Workspaces exist -- switch to last_active or first workspace.
        if let Some(last_active) = workspace_mgr.last_active_id().await {
            workspace_mgr.switch_workspace(&last_active).await?;
        } else {
            let first_id = workspaces[0].id.clone();
            workspace_mgr.switch_workspace(&first_id).await?;
        }
    } else {
        // Case D: Fresh install -- no existing DB, no workspaces.
        let default_id = workspace_mgr.create_workspace("Default".to_string()).await?;
        workspace_mgr.switch_workspace(&default_id).await?;
        log::info!("Fresh install: created Default workspace");
    }

    Ok(workspace_mgr)
}
