---
phase: 01-workspace-filesystem-db-isolation
plan: 03
subsystem: database
tags: [workspace-manager, tauri-state, sqlite-pool, command-handlers, dependency-injection]

# Dependency graph
requires:
  - phase: 01-workspace-filesystem-db-isolation (plan 02)
    provides: WorkspaceManager struct with active_pool()/global_pool() API
provides:
  - All command handlers rewired from AppState to WorkspaceManager
  - Workspace-aware app startup initialization with migration detection hook
  - Proper pool routing (meeting data -> active_pool, settings -> global_pool)
  - Clean app shutdown via close_active_workspace()
affects: [01-04 migration plan, Phase 2 workspace CRUD commands, any future command handlers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "active_pool() for workspace-scoped meeting/transcript data"
    - "global_pool() for app-wide settings/API keys"
    - "WorkspaceManager as Tauri managed state replacing AppState"
    - "initialize_workspace_manager() orchestration: init -> migration detect -> workspace create/switch"

key-files:
  created: []
  modified:
    - frontend/src-tauri/src/database/setup.rs
    - frontend/src-tauri/src/lib.rs
    - frontend/src-tauri/src/state.rs
    - frontend/src-tauri/src/api/api.rs
    - frontend/src-tauri/src/summary/commands.rs
    - frontend/src-tauri/src/onboarding.rs
    - frontend/src-tauri/src/database/commands.rs

key-decisions:
  - "AppState fully retired, state.rs kept as empty module for compatibility"
  - "Legacy import_and_initialize_database marked with TODO for Plan 04 rather than rewritten"
  - "check_first_launch now checks workspace count instead of sqlite file existence"
  - "initialize_fresh_database uses global_pool for settings, skips DB creation (workspace already exists)"

patterns-established:
  - "Pool routing: all meeting/transcript commands use workspace_mgr.active_pool().await?, settings use workspace_mgr.global_pool()"
  - "active_pool returns owned SqlitePool, pass as &pool to repository methods"
  - "global_pool returns &SqlitePool reference, pass directly"
  - "Setup orchestration: WorkspaceManager::init() -> migration detection -> workspace creation/switch"

# Metrics
duration: 7min
completed: 2026-02-01
---

# Phase 1 Plan 3: Rewire Command Handlers Summary

**All 24+ database access points migrated from AppState to WorkspaceManager with correct pool routing for meeting data vs global settings**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-01T17:54:55Z
- **Completed:** 2026-02-01T18:02:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Replaced AppState with WorkspaceManager across all command handler files (api.rs, summary/commands.rs, onboarding.rs, database/commands.rs)
- Rewrote database/setup.rs with workspace initialization orchestration (init -> migration detect -> workspace create/switch)
- Updated lib.rs setup hook to register WorkspaceManager as Tauri state and exit handler to close active workspace
- Retired AppState struct entirely -- zero active references remain

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite database setup and lib.rs for WorkspaceManager** - `81c2af7` (feat)
2. **Task 2: Rewire all command handlers from AppState to WorkspaceManager** - `6522de7` (feat)

## Files Created/Modified
- `frontend/src-tauri/src/database/setup.rs` - Replaced initialize_database_on_startup() with initialize_workspace_manager() including migration detection
- `frontend/src-tauri/src/lib.rs` - WorkspaceManager registration in setup, close_active_workspace in exit handler
- `frontend/src-tauri/src/state.rs` - Retired AppState struct, kept as empty module
- `frontend/src-tauri/src/api/api.rs` - All 18 command handlers: meeting commands -> active_pool(), settings commands -> global_pool()
- `frontend/src-tauri/src/summary/commands.rs` - 4 summary commands -> active_pool()
- `frontend/src-tauri/src/onboarding.rs` - complete_onboarding -> global_pool() for settings
- `frontend/src-tauri/src/database/commands.rs` - check_first_launch via workspace count, initialize_fresh_database via global_pool, import marked TODO

## Decisions Made
- **AppState fully retired:** state.rs kept as empty module to avoid removing `pub mod state;` from lib.rs (cleanup deferred to future pass)
- **Legacy import not rewritten:** import_and_initialize_database logs a warning and marks TODO for Plan 04 migration rather than attempting workspace-aware import now
- **check_first_launch changed semantics:** Now checks `workspace_mgr.list_workspaces().await.is_empty()` instead of checking for sqlite file existence
- **initialize_fresh_database simplified:** No longer creates DatabaseManager -- uses global_pool from already-initialized WorkspaceManager for settings defaults

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All database access now goes through WorkspaceManager
- Migration hook placeholder in setup.rs ready for Plan 04 to implement actual data migration
- Plan 04 (data migration) can proceed -- it needs to replace the placeholder in setup.rs Case A with actual migration logic
- The `import_and_initialize_database` command needs workspace-aware rewrite in Plan 04

---
*Phase: 01-workspace-filesystem-db-isolation*
*Completed: 2026-02-01*
