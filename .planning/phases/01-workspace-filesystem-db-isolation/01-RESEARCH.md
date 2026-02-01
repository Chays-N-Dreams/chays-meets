# Phase 1: Workspace Filesystem + DB Isolation - Research

**Researched:** 2026-02-01
**Domain:** Rust/Tauri state management, SQLite pool lifecycle, filesystem isolation
**Confidence:** HIGH

## Summary

This phase requires building a WorkspaceManager abstraction that replaces the current single-database `AppState` with a multi-workspace system. The existing codebase uses `sqlx 0.8` with `SqlitePool`, managed as Tauri state via `app.manage(AppState { db_manager })`. Since Tauri's `manage()` does not support replacing state after registration (it returns `false` on subsequent calls for the same type), the current architecture must be restructured to use interior mutability (`Arc<RwLock<...>>`) to enable workspace switching at runtime.

The core implementation involves: (1) a filesystem layer that creates/manages UUID-named workspace directories with manifest.json metadata, (2) a database layer that opens/closes SqlitePool connections as the user switches workspaces, (3) a global database for settings/API keys that lives outside workspace directories, and (4) a migration path that moves existing data from the single `meeting_minutes.sqlite` into a "Default" workspace.

**Primary recommendation:** Wrap the active workspace's DatabaseManager in `Arc<RwLock<Option<DatabaseManager>>>` inside a new `WorkspaceManager` struct registered as Tauri managed state. All existing commands that access `state.db_manager.pool()` (30+ call sites) must go through the WorkspaceManager's accessor method instead.

## Standard Stack

The established libraries/tools for this domain:

### Core (Already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sqlx | 0.8 | SQLite pool, migrations, queries | Already used; compile-time checked queries |
| tokio | 1.32.0 | Async runtime, RwLock, file I/O | Already used; Tauri's async runtime |
| serde / serde_json | 1.0 | JSON serialization for manifest.json, workspaces.json | Already used throughout codebase |
| uuid | 1.0 (v4, serde) | UUID generation for workspace folder names | Already used for meeting/transcript IDs |
| tauri | 2.6.2 | App framework, managed state, path APIs | Already used; provides `app_data_dir()` |
| chrono | 0.4.31 | Timestamps for manifest metadata | Already used for meeting timestamps |

### Supporting (No New Dependencies Required)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::fs | stdlib | Directory creation, file copy, manifest I/O | Workspace folder operations |
| tokio::fs | via tokio | Async file I/O for non-blocking operations | Large file operations (DB copy) |
| log | 0.4 | Structured logging | Already used throughout |

### No New Dependencies Needed
This phase requires **zero new crate additions**. Everything needed is already in `Cargo.toml`. The `uuid` crate already has `v4` and `serde` features enabled. The `serde_json` crate handles manifest serialization. The `sqlx` crate handles all database operations.

## Architecture Patterns

### Current Architecture (Before This Phase)

```
AppState (Tauri managed state, registered once via app.manage())
  +-- db_manager: DatabaseManager
        +-- pool: SqlitePool  (single meeting_minutes.sqlite)

All 30+ command handlers access: state.db_manager.pool()
```

Key files:
- `state.rs`: `pub struct AppState { pub db_manager: DatabaseManager }`
- `database/manager.rs`: `DatabaseManager { pool: SqlitePool }`
- `database/setup.rs`: `app.manage(AppState { db_manager })` called once on startup
- `api/api.rs`: 20+ commands taking `state: tauri::State<'_, AppState>`
- `summary/commands.rs`: 4 commands accessing `state.db_manager.pool()`
- `onboarding.rs`: 1 command accessing `state.db_manager.pool()`

### Recommended Architecture (After This Phase)

```
workspaces/                              (user-configurable root)
  +-- workspaces.json                    (global registry)
  +-- global.sqlite                      (API keys, app settings, workspace order)
  +-- <uuid-1>/                          (workspace folder)
  |     +-- manifest.json                (name, emoji, color, dates)
  |     +-- db.sqlite                    (meetings, transcripts, summaries)
  |     +-- audio/                       (recording files)
  |     +-- notes/                       (meeting notes)
  |     +-- config.json                  (workspace-specific config)
  |     +-- mcp-config.json             (MCP config, future phase)
  +-- <uuid-2>/
        +-- manifest.json
        +-- db.sqlite
        +-- audio/
        +-- notes/
        +-- config.json
        +-- mcp-config.json
```

### Rust State Architecture

```rust
// NEW: Replaces the simple AppState
pub struct WorkspaceManager {
    /// Root directory for all workspaces
    workspaces_root: PathBuf,
    /// Currently active workspace's DB pool (swappable via RwLock)
    active_db: Arc<RwLock<Option<DatabaseManager>>>,
    /// Global settings DB pool (always open, never switches)
    global_db: DatabaseManager,
    /// Currently active workspace UUID
    active_workspace_id: Arc<RwLock<Option<Uuid>>>,
    /// Cached workspace registry (in-memory copy of workspaces.json)
    registry: Arc<RwLock<WorkspaceRegistry>>,
}
```

Register as Tauri state:
```rust
// In setup hook (lib.rs)
app.manage(workspace_manager);

// In command handlers (replaces state: tauri::State<'_, AppState>)
async fn api_get_meetings<R: Runtime>(
    workspace_mgr: tauri::State<'_, WorkspaceManager>,
) -> Result<Vec<Meeting>, String> {
    let pool = workspace_mgr.active_pool().await?;
    MeetingsRepository::get_meetings(&pool).await...
}
```

### Pattern 1: Active Pool Accessor with Error Handling

**What:** A single method on WorkspaceManager that returns the active workspace's pool or an error if no workspace is active.
**When to use:** Every command handler that needs database access.
**Example:**
```rust
impl WorkspaceManager {
    /// Get the active workspace's database pool.
    /// Returns an error if no workspace is currently active.
    pub async fn active_pool(&self) -> Result<SqlitePool, String> {
        let guard = self.active_db.read().await;
        match guard.as_ref() {
            Some(db_manager) => Ok(db_manager.pool().clone()),
            None => Err("No active workspace. Please select or create a workspace.".to_string()),
        }
    }

    /// Get the global settings database pool.
    /// This pool is always available.
    pub fn global_pool(&self) -> &SqlitePool {
        self.global_db.pool()
    }
}
```

### Pattern 2: Workspace Switch with Pool Lifecycle

**What:** Close current workspace pool, open new workspace pool, update active state.
**When to use:** When user switches workspaces (must NOT be called during recording).
**Example:**
```rust
impl WorkspaceManager {
    pub async fn switch_workspace(&self, workspace_id: Uuid) -> Result<(), String> {
        // 1. Close current pool if any
        {
            let mut guard = self.active_db.write().await;
            if let Some(current) = guard.take() {
                current.cleanup().await.map_err(|e| e.to_string())?;
            }
        }

        // 2. Resolve workspace path
        let ws_path = self.workspaces_root.join(workspace_id.to_string());
        let db_path = ws_path.join("db.sqlite");

        // 3. Open new pool and run migrations
        let db_url = format!("sqlite:{}", db_path.display());
        let new_pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                SqliteConnectOptions::from_str(&db_url)?
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .foreign_keys(true)
            )
            .await?;

        // Run workspace schema migrations
        sqlx::migrate!("./migrations").run(&new_pool).await?;

        // 4. Update active state
        {
            let mut guard = self.active_db.write().await;
            *guard = Some(DatabaseManager::from_pool(new_pool));
        }
        {
            let mut guard = self.active_workspace_id.write().await;
            *guard = Some(workspace_id);
        }

        Ok(())
    }
}
```

### Pattern 3: Manifest JSON Structure

**What:** Typed struct for workspace metadata file.
**When to use:** Creating, reading, updating workspace identity.
**Example:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// Schema version for forward compatibility
    pub version: u32,
    /// Human-readable display name
    pub name: String,
    /// Optional emoji icon (e.g., "🏢", "🎓")
    pub icon: Option<String>,
    /// Accent color as hex string (e.g., "#3B82F6")
    pub accent_color: Option<String>,
    /// Description field
    pub description: Option<String>,
    /// ISO 8601 timestamp
    pub created_at: String,
    /// ISO 8601 timestamp, updated on any change
    pub last_modified: String,
}
```

### Pattern 4: Registry JSON Structure

**What:** Global lookup file for fast workspace enumeration.
**When to use:** App startup, sidebar population, workspace ordering.
**Example:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistry {
    /// Schema version for forward compatibility
    pub version: u32,
    /// Ordered list of workspace entries (order = sidebar order)
    pub workspaces: Vec<WorkspaceEntry>,
    /// UUID of the last active workspace (for restore on app launch)
    pub last_active: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// UUID (matches folder name)
    pub id: String,
    /// Cached display name (from manifest, for fast sidebar rendering)
    pub name: String,
    /// Cached icon (from manifest)
    pub icon: Option<String>,
}
```

### Anti-Patterns to Avoid
- **Direct pool access from AppState:** All existing `state.db_manager.pool()` calls must be replaced. Do NOT keep the old AppState alongside WorkspaceManager.
- **Holding pool references across await points during switch:** Never hold a read lock on `active_db` across a workspace switch boundary. Clone the pool reference before using it.
- **Multiple concurrent pool opens:** Only one workspace pool should be open at a time. The switch method must close-then-open atomically.
- **Mixing global and workspace data:** Settings (API keys, device preferences) go in `global.sqlite`. Meetings, transcripts, summaries go in workspace `db.sqlite`. Do NOT store workspace-specific data in the global DB.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Database migration tracking | Custom version table | `sqlx::migrate!()` macro | Already used; tracks applied migrations in `_sqlx_migrations` table, handles idempotent re-runs |
| UUID generation | Custom ID scheme | `uuid::Uuid::new_v4()` | Already used for meeting IDs; gives guaranteed uniqueness for folder names |
| JSON file serialization | Manual string building | `serde_json::to_string_pretty()` / `serde_json::from_str()` | Type-safe, handles escaping, already in codebase |
| WAL checkpoint on close | Manual PRAGMA calls | `DatabaseManager::cleanup()` already does this | Existing pattern checkpoints WAL and closes pool gracefully |
| Path resolution for app data | Hardcoded paths | `app.path().app_data_dir()` | Cross-platform, respects Tauri bundle identifier |
| SQLite database file creation | Manual file creation | `SqliteConnectOptions::create_if_missing(true)` | Handles file creation atomically with first connection |
| File copy for migration | Custom byte-copy logic | `std::fs::copy()` or `tokio::fs::copy()` | For the one-time migration of existing DB, a simple file copy is sufficient since we checkpoint WAL first |

**Key insight:** The `sqlx::migrate!()` macro embeds migration SQL at compile time and can be run against ANY pool. This means the same set of migrations works for every workspace DB -- each DB independently tracks which migrations it has applied via its own `_sqlx_migrations` table. No need for a shared schema version tracker.

## Common Pitfalls

### Pitfall 1: Tauri manage() Does Not Replace State
**What goes wrong:** Calling `app.manage(AppState { ... })` a second time is silently ignored (returns `false`). The original state persists.
**Why it happens:** Tauri's state manager is designed for set-once semantics. The current codebase gets away with this because `manage()` is only called once per app lifecycle (either in setup.rs for non-first-launch, or in commands.rs for first-launch).
**How to avoid:** Use interior mutability (`Arc<RwLock<...>>`) inside the managed state struct. Register the struct once with `manage()`, then mutate its contents via write locks.
**Warning signs:** If workspace switching seems to have no effect on queries, the old pool is still being used.

### Pitfall 2: SQLite WAL Files Left Behind on Pool Close
**What goes wrong:** After closing a pool without checkpointing, `.wal` and `.shm` files remain alongside the `.sqlite` file. If the workspace folder is moved or the app crashes, these orphaned files can cause "database is malformed" errors on next open.
**Why it happens:** SQLite WAL mode defers writing changes to the main DB file. The existing `DatabaseManager::cleanup()` method already handles this with `PRAGMA wal_checkpoint(TRUNCATE)`.
**How to avoid:** Always call cleanup() before closing a workspace pool. On workspace switch: checkpoint -> close -> open new.
**Warning signs:** `.wal` and `.shm` files visible in workspace folders after switching.

### Pitfall 3: Settings Split -- API Keys in Wrong Database
**What goes wrong:** After splitting to per-workspace DBs, the `settings` and `transcript_settings` tables end up in workspace DBs, but they contain API keys that should be global.
**Why it happens:** The current schema has `settings` (with API keys) and `transcript_settings` in the same DB as meetings. The decision is that API keys are global while meetings are per-workspace.
**How to avoid:** Create a separate migration set for `global.sqlite` that includes `settings`, `transcript_settings`, and `licensing` tables. Workspace `db.sqlite` only gets `meetings`, `transcripts`, `summary_processes`, `transcript_chunks`, and `meeting_notes`. All SettingsRepository calls must use `global_pool()` instead of `active_pool()`.
**Warning signs:** API keys disappearing when switching workspaces.

### Pitfall 4: Migration From Single DB -- Data Loss Risk
**What goes wrong:** The existing `meeting_minutes.sqlite` is modified or deleted during migration to the workspace structure, and if migration fails mid-way, data is lost.
**Why it happens:** No backup strategy before destructive migration operations.
**How to avoid:** (1) Copy the original DB file to a backup location before any migration. (2) Create the Default workspace and its `db.sqlite` as a copy of the original. (3) Extract global settings into `global.sqlite`. (4) Only delete/archive the original after verifying the migration succeeded. (5) Keep the backup for at least one app version cycle.
**Warning signs:** Users reporting missing meetings after upgrade.

### Pitfall 5: Concurrent Access During Workspace Switch
**What goes wrong:** A command handler acquires the active pool just before a workspace switch, then uses the old pool to write data that belongs to the new workspace.
**Why it happens:** Race condition between pool accessor reads and switch writes.
**How to avoid:** (1) Recording locks workspace switching (already decided). (2) The switch_workspace method holds a write lock on `active_db` which blocks all readers. (3) Commands should acquire the pool, perform their operation, and release quickly -- no holding pool references across long async operations. (4) Emit a Tauri event before switch starts so frontend can stop in-flight operations.
**Warning signs:** Data appearing in wrong workspace.

### Pitfall 6: Compile-Time Query Checking With Multiple DB Schemas
**What goes wrong:** `sqlx` compile-time query checking (`query!` macros) requires a single `DATABASE_URL` environment variable. If workspace and global schemas differ, which DB does the compile-time checker use?
**Why it happens:** The codebase uses raw `sqlx::query()` and `sqlx::query_as()` (not the `query!` macro), so this is actually not a problem here. But it would be if someone added `query!` macro calls.
**How to avoid:** Continue using `sqlx::query()` and `sqlx::query_as()` (runtime string queries) rather than `query!` macros. The existing codebase already does this consistently.
**Warning signs:** Not applicable unless compile-time checking is added.

### Pitfall 7: Workspace Directory Permissions on Different OS
**What goes wrong:** Creating directories in `app_data_dir` fails on some OS configurations.
**Why it happens:** Permission differences across macOS, Windows, Linux. App sandboxing on macOS.
**How to avoid:** Always use `std::fs::create_dir_all()` which creates intermediate directories. Always use Tauri's `app.path().app_data_dir()` which respects OS conventions and sandboxing. Test on all target platforms.
**Warning signs:** "Permission denied" errors on first workspace creation.

## Code Examples

### Example 1: Creating a New Workspace

```rust
// Source: Pattern derived from existing DatabaseManager::new() and manifest structure
impl WorkspaceManager {
    pub async fn create_workspace(&self, name: String) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let ws_dir = self.workspaces_root.join(id.to_string());

        // Create directory structure
        std::fs::create_dir_all(&ws_dir).map_err(|e| format!("Failed to create workspace dir: {}", e))?;
        std::fs::create_dir_all(ws_dir.join("audio")).map_err(|e| format!("Failed to create audio dir: {}", e))?;
        std::fs::create_dir_all(ws_dir.join("notes")).map_err(|e| format!("Failed to create notes dir: {}", e))?;

        // Write manifest
        let now = chrono::Utc::now().to_rfc3339();
        let manifest = WorkspaceManifest {
            version: 1,
            name: name.clone(),
            icon: None,
            accent_color: None,
            description: None,
            created_at: now.clone(),
            last_modified: now,
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(ws_dir.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Write default config.json
        let default_config = serde_json::json!({});
        std::fs::write(
            ws_dir.join("config.json"),
            serde_json::to_string_pretty(&default_config).unwrap()
        ).map_err(|e| format!("Failed to write config: {}", e))?;

        // Update registry
        {
            let mut reg = self.registry.write().await;
            reg.workspaces.push(WorkspaceEntry {
                id: id.to_string(),
                name: name.clone(),
                icon: None,
            });
            self.save_registry(&reg).map_err(|e| format!("Failed to save registry: {}", e))?;
        }

        Ok(id)
    }
}
```

### Example 2: Migrating Existing Data to Default Workspace

```rust
// Source: Pattern derived from existing DatabaseManager::new_from_app_handle()
async fn migrate_existing_to_default_workspace(
    workspace_mgr: &WorkspaceManager,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let existing_db = app_data_dir.join("meeting_minutes.sqlite");
    if !existing_db.exists() {
        return Ok(()); // No existing data to migrate
    }

    // Step 1: Backup existing DB
    let backup_path = app_data_dir.join("meeting_minutes.sqlite.backup");
    std::fs::copy(&existing_db, &backup_path)
        .map_err(|e| format!("Failed to backup existing DB: {}", e))?;

    // Step 2: Create "Default" workspace
    let default_id = workspace_mgr.create_workspace("Default".to_string()).await?;
    let ws_dir = workspace_mgr.workspaces_root.join(default_id.to_string());

    // Step 3: Copy existing DB as workspace DB (meetings, transcripts, summaries)
    let ws_db = ws_dir.join("db.sqlite");
    std::fs::copy(&existing_db, &ws_db)
        .map_err(|e| format!("Failed to copy DB to workspace: {}", e))?;

    // Step 4: Clean up copied DB -- remove settings tables (they belong in global)
    // Open the workspace DB and drop global-only tables
    let pool = SqlitePool::connect(&format!("sqlite:{}", ws_db.display())).await
        .map_err(|e| format!("Failed to connect to workspace DB: {}", e))?;
    sqlx::query("DROP TABLE IF EXISTS settings").execute(&pool).await.ok();
    sqlx::query("DROP TABLE IF EXISTS transcript_settings").execute(&pool).await.ok();
    sqlx::query("DROP TABLE IF EXISTS licensing").execute(&pool).await.ok();
    pool.close().await;

    // Step 5: Extract settings into global.sqlite
    // (Global DB is initialized separately with its own schema/migrations)
    // Copy API keys from existing DB to global DB
    let existing_pool = SqlitePool::connect(&format!("sqlite:{}", existing_db.display())).await
        .map_err(|e| format!("Failed to connect to existing DB: {}", e))?;
    // ... transfer settings rows to global_db ...
    existing_pool.close().await;

    // Step 6: Switch to the new default workspace
    workspace_mgr.switch_workspace(default_id).await?;

    Ok(())
}
```

### Example 3: Splitting Migration Sets (Workspace vs Global)

```rust
// Workspace DB migrations (meetings, transcripts, summaries, notes)
// Located in: migrations/workspace/
// Applied to each workspace's db.sqlite independently

// Global DB migrations (settings, API keys, licensing)
// Located in: migrations/global/
// Applied only to global.sqlite

// Using Migrator::new() with dynamic paths for separate migration sets:
static WORKSPACE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/workspace");
static GLOBAL_MIGRATOR: Migrator = sqlx::migrate!("./migrations/global");

// On workspace open:
WORKSPACE_MIGRATOR.run(&workspace_pool).await?;

// On app startup:
GLOBAL_MIGRATOR.run(&global_pool).await?;
```

**Important note about migration splitting:** The existing migrations in `./migrations/` create both meeting tables AND settings tables. For the split, the approach should be:

1. Create `migrations/workspace/` with a single initial migration that creates ONLY workspace tables (meetings, transcripts, summary_processes, transcript_chunks, meeting_notes)
2. Create `migrations/global/` with a single initial migration that creates ONLY global tables (settings, transcript_settings, licensing)
3. The existing `migrations/` directory stays unchanged for compile-time compatibility but stops being used at runtime
4. When migrating an existing DB, the `_sqlx_migrations` table is already populated -- the workspace DB copy will have the old migration history. New workspace migrations should use fresh timestamps to avoid conflicts.

### Example 4: Registry Self-Healing (Directory Scan Fallback)

```rust
impl WorkspaceManager {
    /// Rebuild registry by scanning workspace directories
    pub async fn rebuild_registry(&self) -> Result<WorkspaceRegistry, String> {
        let mut entries = Vec::new();

        let read_dir = std::fs::read_dir(&self.workspaces_root)
            .map_err(|e| format!("Failed to read workspaces dir: {}", e))?;

        for entry in read_dir {
            let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
            let path = entry.path();

            if !path.is_dir() { continue; }

            let manifest_path = path.join("manifest.json");
            if !manifest_path.exists() { continue; }

            // Try to parse folder name as UUID
            let folder_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| "Invalid folder name".to_string())?;

            if Uuid::parse_str(folder_name).is_err() { continue; }

            // Read manifest
            let manifest_content = std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("Failed to read manifest: {}", e))?;
            let manifest: WorkspaceManifest = serde_json::from_str(&manifest_content)
                .map_err(|e| format!("Failed to parse manifest: {}", e))?;

            entries.push(WorkspaceEntry {
                id: folder_name.to_string(),
                name: manifest.name.clone(),
                icon: manifest.icon.clone(),
            });
        }

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: entries,
            last_active: None,
        };

        // Save rebuilt registry
        self.save_registry(&registry)?;

        Ok(registry)
    }
}
```

## State of the Art

| Old Approach (Current) | New Approach (This Phase) | Why Change |
|------------------------|--------------------------|------------|
| Single `AppState { db_manager }` via `app.manage()` | `WorkspaceManager` with `Arc<RwLock<Option<DatabaseManager>>>` | Need to swap DB pool at runtime for workspace switching |
| Single `meeting_minutes.sqlite` | Per-workspace `db.sqlite` + `global.sqlite` | Data isolation between workspaces |
| Settings mixed with meetings in one DB | Settings in `global.sqlite`, meetings in workspace DB | API keys should persist across workspace switches |
| `sqlx::migrate!("./migrations")` single migration set | Two migration sets: `workspace/` and `global/` | Different schemas for workspace vs global databases |
| `state.db_manager.pool()` in 30+ commands | `workspace_mgr.active_pool().await?` | Async accessor needed due to RwLock; error handling for no-workspace state |

**Deprecated/outdated patterns to remove:**
- `state::AppState` struct (replaced by WorkspaceManager)
- Direct `DatabaseManager::new_from_app_handle()` (replaced by WorkspaceManager initialization)
- `database::setup::initialize_database_on_startup()` (replaced by workspace initialization logic)

## DB Migration Strategy Decision (Claude's Discretion)

**Decision: Independent per-workspace migrations.**

Rationale:
- The `sqlx::migrate!()` macro creates a static `Migrator` that can be `.run()` against any pool. Each workspace DB independently tracks applied migrations via its own `_sqlx_migrations` table.
- This means if a workspace hasn't been opened in a while (e.g., user created it months ago, app updated since), its migrations will be applied on next open -- safe and automatic.
- A shared schema version would require a global tracker and coordinated updates across all workspace DBs even when they're not open -- unnecessary complexity.
- The sqlx Migrator validates previously applied migrations against the current migration source to detect accidental changes, providing built-in safety.

## Additional Manifest Metadata Fields (Claude's Discretion)

Beyond the basics (name, icon, accent_color, description, created_at, last_modified), add:
- `version: u32` -- Schema version of the manifest format itself (forward compatibility)
- `app_version: String` -- Version of Meetily that created the workspace (debugging aid)

Do NOT add:
- Meeting count or summary stats (stale quickly, must query DB)
- Active/archive status (all workspaces are equal per decision)

## Migration Safety Mechanisms (Claude's Discretion)

1. **Backup before migration:** Copy `meeting_minutes.sqlite` to `meeting_minutes.sqlite.pre-workspace-backup` before any migration operations
2. **Verify after migration:** After creating Default workspace, open its DB and verify meeting count matches original
3. **Atomic registry updates:** Write registry to `workspaces.json.tmp` then rename to `workspaces.json` (atomic on most filesystems)
4. **Keep backup for one version cycle:** Don't auto-delete the backup; let users manually clean it up or auto-remove after next successful app launch

## Workspace Root Directory Change Handling (Claude's Discretion)

**Decision: Leave existing workspaces in place, update root path in global config.**

- When user changes workspace root directory, do NOT attempt to move existing workspaces (too risky, could fail mid-copy for large audio folders)
- Instead: update the root path in global settings, scan the new directory for any existing workspaces
- If user wants workspaces in the new location, they can manually move the folders (they're self-contained by design)
- Display a clear message: "Workspaces in the previous location will no longer appear. Move workspace folders manually if needed."

## Open Questions

1. **Audio file migration path**
   - What we know: Existing meetings have `folder_path` pointing to recording directories (typically in Downloads or user-selected locations). These audio files live outside the app data directory.
   - What's unclear: Should the Default workspace migration move/copy audio files into the workspace's `audio/` folder, or just keep the existing `folder_path` references? Moving could be slow for large recordings.
   - Recommendation: Keep existing `folder_path` references as-is for migrated meetings. New recordings in any workspace go into that workspace's `audio/` folder. Add this as a documented behavior difference between migrated and new meetings.

2. **Global DB migration set management**
   - What we know: Need separate migration directories for workspace and global schemas. `sqlx::migrate!()` macro works with a compile-time path.
   - What's unclear: Using two `sqlx::migrate!()` macros with different paths (e.g., `./migrations/workspace` and `./migrations/global`) -- need to verify this compiles correctly with two static Migrator instances.
   - Recommendation: Test this pattern early. If it doesn't work, use `Migrator::new()` with runtime paths for at least one of the two sets.

3. **First launch flow integration**
   - What we know: Current first-launch flow emits `first-launch-detected` event and waits for user to import or create fresh DB. WorkspaceManager changes this flow.
   - What's unclear: Exact integration point -- does first launch create a Default workspace automatically, or show the "Create your first workspace" state?
   - Recommendation: On genuine first launch (no existing DB), auto-create a "Default" workspace so the user can start recording immediately. The "create your first workspace" empty state only appears if the user explicitly deletes all workspaces.

## Sources

### Primary (HIGH confidence)
- **Existing codebase analysis** -- Read all files in `database/`, `state.rs`, `lib.rs`, `api/api.rs`, `summary/commands.rs`, `onboarding.rs`, all 10 migration files
- [sqlx Pool docs](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html) -- Pool lifecycle (close, no reopen)
- [sqlx SqliteConnectOptions docs](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html) -- Connection options, create_if_missing, journal_mode
- [sqlx Migrator docs](https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html) -- run() against multiple pools, independent tracking
- [Tauri State Management docs](https://v2.tauri.app/develop/state-management/) -- manage() returns false on duplicate, interior mutability pattern

### Secondary (MEDIUM confidence)
- [Tauri manage() behavior](https://github.com/tauri-apps/tauri/discussions/3911) -- Confirmed manage() does not replace, must use Mutex/RwLock
- [SQLite Backup API](https://www.sqlite.org/backup.html) -- Official backup approach for live databases
- [Tauri BaseDirectory docs](https://docs.rs/tauri/latest/tauri/path/enum.BaseDirectory.html) -- app_data_dir resolution per OS

### Tertiary (LOW confidence)
- None -- all findings verified against official documentation or codebase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in Cargo.toml, verified versions match
- Architecture: HIGH -- patterns derived directly from codebase analysis + verified Tauri/sqlx API docs
- Pitfalls: HIGH -- pitfalls 1-5 verified against official docs; pitfall 6-7 from codebase observation
- Migration strategy: HIGH -- sqlx Migrator behavior verified against official docs
- Code examples: MEDIUM -- patterns are sound but untested; exact API ergonomics may need adjustment

**Research date:** 2026-02-01
**Valid until:** 2026-03-01 (stable domain; sqlx 0.8 and Tauri 2.6 are mature releases)
