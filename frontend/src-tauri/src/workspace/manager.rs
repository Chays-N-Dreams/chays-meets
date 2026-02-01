use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::database::manager::DatabaseManager;
use super::filesystem;
use super::types::{WorkspaceEntry, WorkspaceManifest, WorkspaceRegistry};

/// Central abstraction that manages workspace lifecycle, database pool switching,
/// and global settings database. Replaces the old AppState as Tauri managed state.
pub struct WorkspaceManager {
    /// Root directory for all workspaces (e.g., ~/Library/Application Support/Meetily/workspaces/)
    workspaces_root: PathBuf,
    /// Currently active workspace's DB pool (swappable via RwLock)
    active_db: Arc<RwLock<Option<DatabaseManager>>>,
    /// Global settings DB pool (always open, never switches)
    global_db: DatabaseManager,
    /// Currently active workspace UUID
    active_workspace_id: Arc<RwLock<Option<String>>>,
    /// Cached workspace registry (in-memory copy of workspaces.json)
    registry: Arc<RwLock<WorkspaceRegistry>>,
}

impl WorkspaceManager {
    /// Initialize the WorkspaceManager infrastructure.
    ///
    /// Creates the workspaces root directory, initializes global.sqlite, and loads the registry.
    /// Does NOT create any workspaces or switch to any workspace — returns with `active_db: None`.
    /// Migration detection and workspace creation are handled by `database/setup.rs` after init.
    pub async fn init(app_data_dir: PathBuf) -> Result<Self, String> {
        let workspaces_root = app_data_dir.join("workspaces");

        // Create workspaces root directory if it doesn't exist
        std::fs::create_dir_all(&workspaces_root)
            .map_err(|e| format!("Failed to create workspaces root: {}", e))?;

        // Initialize global.sqlite
        let global_db_path = workspaces_root.join("global.sqlite");
        let global_options = SqliteConnectOptions::new()
            .filename(&global_db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let global_pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(global_options)
            .await
            .map_err(|e| format!("Failed to connect to global database: {}", e))?;

        // Run global schema creation
        Self::run_global_migrations(&global_pool).await?;

        let global_db = DatabaseManager::from_pool(global_pool);

        // Load registry from disk (or create empty if missing)
        let registry = filesystem::load_registry(&workspaces_root)?;

        log::info!(
            "WorkspaceManager initialized: root={:?}, {} workspaces in registry",
            workspaces_root,
            registry.workspaces.len()
        );

        Ok(Self {
            workspaces_root,
            active_db: Arc::new(RwLock::new(None)),
            global_db,
            active_workspace_id: Arc::new(RwLock::new(None)),
            registry: Arc::new(RwLock::new(registry)),
        })
    }

    /// Get the active workspace's SQLite pool.
    ///
    /// Returns an error if no workspace is currently active.
    pub async fn active_pool(&self) -> Result<SqlitePool, String> {
        let guard = self.active_db.read().await;
        match &*guard {
            Some(db_manager) => Ok(db_manager.pool().clone()),
            None => Err("No active workspace. Please select or create a workspace.".to_string()),
        }
    }

    /// Get a reference to the global settings database pool.
    ///
    /// Always available — does not depend on an active workspace.
    pub fn global_pool(&self) -> &SqlitePool {
        self.global_db.pool()
    }

    /// Switch to a different workspace by closing the current pool and opening a new one.
    pub async fn switch_workspace(&self, workspace_id: &str) -> Result<(), String> {
        // Close current pool if any
        {
            let mut active = self.active_db.write().await;
            if let Some(db_manager) = active.take() {
                if let Err(e) = db_manager.cleanup().await {
                    log::warn!("Failed to cleanup previous workspace pool: {}", e);
                }
            }
        }

        // Resolve workspace path
        let workspace_dir = self.workspaces_root.join(workspace_id);
        if !workspace_dir.exists() {
            return Err(format!("Workspace directory does not exist: {:?}", workspace_dir));
        }
        if !workspace_dir.join("manifest.json").exists() {
            return Err(format!("Workspace missing manifest.json: {:?}", workspace_dir));
        }

        // Open workspace database
        let db_path = workspace_dir.join("db.sqlite");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| format!("Failed to connect to workspace database: {}", e))?;

        // Run workspace migrations
        Self::run_workspace_migrations(&pool).await?;

        // Update active state
        let db_manager = DatabaseManager::from_pool(pool);
        {
            let mut active = self.active_db.write().await;
            *active = Some(db_manager);
        }
        {
            let mut active_id = self.active_workspace_id.write().await;
            *active_id = Some(workspace_id.to_string());
        }

        // Update registry's last_active and save to disk
        {
            let mut reg = self.registry.write().await;
            reg.last_active = Some(workspace_id.to_string());
            filesystem::save_registry(&self.workspaces_root, &reg)?;
        }

        log::info!("Switched to workspace: {}", workspace_id);
        Ok(())
    }

    /// Create a new workspace with the given name.
    ///
    /// Creates the directory structure, manifest, and default config.
    /// Does NOT switch to the new workspace — call `switch_workspace` afterwards.
    /// Returns the new workspace's UUID.
    pub async fn create_workspace(&self, name: String) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();

        // Create directory structure
        let workspace_dir = filesystem::create_workspace_dir(&self.workspaces_root, &id)?;

        // Create manifest
        let now = chrono::Utc::now().to_rfc3339();
        let manifest = WorkspaceManifest {
            version: 1,
            name: name.clone(),
            icon: None,
            accent_color: None,
            description: None,
            app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            created_at: now.clone(),
            last_modified: now,
        };
        filesystem::write_manifest(&workspace_dir, &manifest)?;

        // Write default config
        filesystem::write_default_config(&workspace_dir)?;

        // Update in-memory registry
        {
            let mut reg = self.registry.write().await;
            reg.workspaces.push(WorkspaceEntry {
                id: id.clone(),
                name,
                icon: None,
            });
            filesystem::save_registry(&self.workspaces_root, &reg)?;
        }

        log::info!("Created workspace: {}", id);
        Ok(id)
    }

    /// List all workspaces from the in-memory registry.
    pub async fn list_workspaces(&self) -> Vec<WorkspaceEntry> {
        let reg = self.registry.read().await;
        reg.workspaces.clone()
    }

    /// Get the currently active workspace ID, if any.
    pub async fn active_workspace_id(&self) -> Option<String> {
        let guard = self.active_workspace_id.read().await;
        guard.clone()
    }

    /// Get the workspaces root directory path.
    pub fn workspaces_root(&self) -> &Path {
        &self.workspaces_root
    }

    /// Get the last_active workspace ID from the registry.
    pub async fn last_active_id(&self) -> Option<String> {
        let reg = self.registry.read().await;
        reg.last_active.clone()
    }

    /// Close the active workspace pool for shutdown.
    pub async fn close_active_workspace(&self) -> Result<(), String> {
        let mut active = self.active_db.write().await;
        if let Some(db_manager) = active.take() {
            db_manager.cleanup().await
                .map_err(|e| format!("Failed to cleanup active workspace: {}", e))?;
        }
        let mut active_id = self.active_workspace_id.write().await;
        *active_id = None;
        log::info!("Active workspace closed");
        Ok(())
    }

    /// Run workspace schema migrations on a pool.
    async fn run_workspace_migrations(pool: &SqlitePool) -> Result<(), String> {
        let sql = include_str!("../../migrations/workspace/20260201000000_workspace_schema.sql");
        Self::execute_multi_statement_sql(pool, sql, "workspace").await
    }

    /// Run global schema migrations on a pool.
    async fn run_global_migrations(pool: &SqlitePool) -> Result<(), String> {
        let sql = include_str!("../../migrations/global/20260201000000_global_schema.sql");
        Self::execute_multi_statement_sql(pool, sql, "global").await
    }

    /// Execute multi-statement SQL by splitting on semicolons and running each statement.
    async fn execute_multi_statement_sql(pool: &SqlitePool, sql: &str, label: &str) -> Result<(), String> {
        for statement in sql.split(';') {
            let trimmed = statement.trim();
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }
            // Skip lines that are only comments
            let non_comment_content: String = trimmed
                .lines()
                .filter(|line| !line.trim().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n");
            if non_comment_content.trim().is_empty() {
                continue;
            }
            sqlx::query(trimmed)
                .execute(pool)
                .await
                .map_err(|e| format!("Failed to run {} migration statement: {}", label, e))?;
        }
        Ok(())
    }
}
