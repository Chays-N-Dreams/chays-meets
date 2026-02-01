# Pitfalls Research: Meetily v0.3.0 (Workspaces + MCP)

**Domain:** Desktop meeting assistant -- workspace isolation and MCP client integration
**Researched:** 2026-02-01
**Confidence:** MEDIUM-HIGH (verified against codebase analysis, MCP spec, SQLite docs, community reports)

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or major architectural problems.

### Pitfall 1: Single-DB Assumption Baked Throughout Both Codebases

**What goes wrong:**
The current codebase has a **single global DatabaseManager** instantiated at startup in both the Tauri frontend (`AppState { db_manager }` in `state.rs`) and the Python backend (`db = DatabaseManager()` at module level in `main.py`). Every API endpoint, every Tauri command, and every repository accesses this single instance. Moving to per-workspace databases requires threading a workspace context through every data access path -- and missing even one path causes cross-workspace data leakage or crashes.

**Why it happens:**
The Tauri `AppState` struct holds a single `DatabaseManager` with a single `SqlitePool`. The backend Python `DatabaseManager.__init__` defaults to `os.getenv('DATABASE_PATH', 'meeting_minutes.db')` -- a single path. Developers underestimate how many call sites reference these globals. The Tauri frontend has the `db_manager` accessed via `app.state::<AppState>()` in at least 8 files. The backend has `db = DatabaseManager()` used by all 15+ endpoints.

**How to avoid:**
1. **Audit every database access point first.** Before writing workspace code, grep for every `AppState`, `db_manager`, `DatabaseManager()`, and `self.db_path` reference across both codebases. Document them in a checklist.
2. **Introduce a WorkspaceContext abstraction** that wraps DatabaseManager. All existing code should migrate to resolving the current workspace's DB through this context, not accessing a global.
3. **In the Tauri frontend:** Replace the single `AppState { db_manager }` with a `WorkspaceManager` that holds a map of workspace IDs to their DatabaseManagers. The "current workspace" becomes a Tauri-managed state selection.
4. **In the Python backend:** Pass workspace ID in API requests. The backend resolves the correct database path from the workspace ID. Never rely on a module-level global `db`.
5. **Migration order matters:** Build the workspace abstraction layer first, verify all existing functionality still works through it (with a "default" workspace), THEN add multi-workspace support.

**Warning signs:**
- Meetings from Workspace A appearing in Workspace B's list
- Settings changes in one workspace affecting another
- "Database is locked" errors when switching workspaces
- Backend returning 500 errors after workspace switch

**Phase to address:** Phase 1 (Database abstraction layer) -- this is the foundational work that everything else depends on.

---

### Pitfall 2: Workspace Switching During Active Recording Causes Data Corruption

**What goes wrong:**
A user starts recording in Workspace A, then switches to Workspace B. The recording pipeline (audio capture, VAD, Whisper transcription) is writing to Workspace A's database and audio folder. If the workspace switch swaps the active database connection, incoming transcript segments silently write to the wrong database -- or fail entirely because the old connection was closed. Audio files may be saved to the wrong workspace's folder, orphaned, or truncated.

**Why it happens:**
The recording pipeline in Rust (`recording_manager.rs`, `pipeline.rs`, `recording_saver.rs`) captures references to the save path and database at recording start time. If workspace switching replaces the global state mid-recording, these captured references become stale or dangling. The `RECORDING_FLAG: AtomicBool` is a process-global singleton -- it does not know about workspaces.

**How to avoid:**
1. **Lock workspace switching while recording is active.** The simplest and safest approach: if `is_recording()` returns true, the UI should disable workspace switching with a clear message ("Stop recording before switching workspaces").
2. **Pin recording to a workspace.** When recording starts, capture the workspace ID and all paths/connections at that moment. The recording pipeline should hold its own references, not query global state.
3. **Make the recording state workspace-aware.** Replace the global `RECORDING_FLAG` with per-workspace recording state. But this adds complexity -- multiple simultaneous recordings across workspaces is likely out of scope for v0.3.0.
4. **Test the edge case explicitly:** Start recording, attempt workspace switch, verify behavior.

**Warning signs:**
- Transcripts appearing in the wrong workspace after switching
- Audio files saved in unexpected directories
- "Database connection closed" errors in logs during recording
- `RECORDING_FLAG` returning stale values

**Phase to address:** Phase 1 (Workspace model) -- define the workspace switching rules upfront. The UI must enforce them from day one.

---

### Pitfall 3: MCP Server Child Processes Become Zombies or Orphans

**What goes wrong:**
The MCP stdio transport requires the client (Meetily) to spawn MCP server processes as child processes and communicate via stdin/stdout pipes. If Meetily crashes, exits unexpectedly, or fails to clean up properly, these child processes continue running indefinitely as orphans. On Windows specifically, this is a known and well-documented problem (see [Claude Code issue #15211](https://github.com/anthropics/claude-code/issues/15211) and [MetaMCP issue #128](https://github.com/metatool-ai/metamcp/issues/128)). Over time, orphaned processes accumulate, consuming memory and potentially holding file locks.

**Why it happens:**
- Rust's `std::process::Child` has no `Drop` implementation -- if the `Child` handle goes out of scope without `wait()` or `kill()`, the process keeps running as a zombie.
- On Windows, `kill()` on child processes does not always work (known Tauri bug, [issue #4949](https://github.com/tauri-apps/tauri/issues/4949)).
- Crash paths (panics, SIGKILL, force quit) bypass cleanup code.
- Per-workspace MCP servers mean more child processes to track.

**How to avoid:**
1. **Implement a process supervisor pattern.** Create a `McpProcessManager` that:
   - Tracks all spawned MCP server PIDs
   - Periodically health-checks each process (via `try_wait()`)
   - Kills and restarts failed processes
   - Cleans up all processes on app shutdown
2. **Use platform-specific process grouping:**
   - **macOS/Linux:** Spawn MCP servers in their own process group. On shutdown, send `SIGTERM` to the entire group, then `SIGKILL` after a timeout.
   - **Windows:** Use Job Objects to tie child processes to the parent. When the parent exits (for any reason), Windows automatically terminates all processes in the Job.
3. **Register cleanup in Tauri's exit handler.** The current `RunEvent::Exit` handler already cleans up the database and sidecar. Add MCP server cleanup there.
4. **Write a PID file.** On startup, check if previous MCP server processes are still running (from a crash). Kill them before spawning new ones.
5. **Idle timeout.** MCP servers that have been idle for extended periods should be terminated and respawned on-demand (as implemented by [DeployStack's ProcessManager](https://docs.deploystack.io/development/satellite/process-management)).

**Warning signs:**
- System monitor showing orphaned `node`, `python`, or other MCP server processes
- Port conflicts when restarting MCP servers
- Memory usage growing over time
- File lock errors on workspace databases

**Phase to address:** Phase 2 (MCP framework) -- this must be built into the MCP client from the start, not bolted on later.

---

### Pitfall 4: Migration From Single DB to Per-Workspace DBs Loses Existing User Data

**What goes wrong:**
Existing Meetily users have all their meetings, transcripts, summaries, settings, and API keys in a single `meeting_minutes.sqlite` file (Tauri frontend, managed by sqlx with 10 migration files) and potentially a legacy `meeting_minutes.db` (Python backend). The transition to per-workspace databases must preserve this data. A botched migration that drops or fails to copy data will lose users' meeting history permanently.

**Why it happens:**
- The Tauri `DatabaseManager` already handles legacy `.db` to `.sqlite` migration, but the one-to-many splitting (single DB to multiple workspace DBs) is a fundamentally different operation.
- SQLite's limited `ALTER TABLE` support means schema changes during migration may require table rebuilds (create new, copy data, drop old, rename).
- Settings data (API keys, model config) should probably be global, not per-workspace -- but the current schema puts them in the same DB as meetings.
- The Python backend has its own schema that is not managed by sqlx migrations -- it uses ad-hoc `ALTER TABLE` with try/except blocks.

**How to avoid:**
1. **Treat migration as a first-class feature, not an afterthought.** Design the migration path before writing workspace code.
2. **Default workspace strategy:** On first launch after update, create a "Default" workspace and move ALL existing data into it. The user's experience is unchanged until they explicitly create a second workspace.
3. **Separate global vs. workspace data early:**
   - **Global:** Settings, API keys, app preferences, license info
   - **Per-workspace:** Meetings, transcripts, summaries, audio files, MCP server configs, system prompts
   - This split should be designed first, then the migration plan follows naturally.
4. **Never modify the original database in-place during migration.** Copy to the new location, verify integrity, then mark the original as migrated.
5. **Write a migration test** that uses a real pre-migration database (snapshot from the current version) and verifies all data survives the transition.
6. **Version the workspace database schema independently.** Use SQLite's `PRAGMA user_version` or continue with sqlx migrations, but track versions per-workspace-DB.

**Warning signs:**
- Empty meeting lists after app update
- Settings reset to defaults after migration
- "Meeting not found" errors for existing meetings
- WAL/SHM files orphaned from old database location

**Phase to address:** Phase 1 (Database layer) -- migration must be designed alongside the workspace data model, not after.

---

### Pitfall 5: MCP Server Security -- Spawning Arbitrary Processes From User Config

**What goes wrong:**
Per-workspace MCP server configuration means users (or workspace config files) specify commands to spawn as child processes. If this configuration is not validated, an attacker (or a malicious shared workspace config) could specify arbitrary commands -- effectively giving code execution to anyone who can write to the config. Even without malicious intent, a misconfigured MCP server command could execute destructive operations.

**Why it happens:**
- The MCP spec itself says tools "represent arbitrary code execution and must be treated with appropriate caution" (MCP Spec 2025-11-25, Security section).
- Desktop apps often trust local config files implicitly.
- Users may copy-paste MCP server configurations from untrusted sources (blog posts, forums).
- Research by [Knostic (July 2025)](https://www.redhat.com/en/blog/model-context-protocol-mcp-understanding-security-risks-and-controls) found nearly 2,000 MCP servers exposed without authentication.

**How to avoid:**
1. **Allowlist approach for commands.** Only allow MCP server commands that match known patterns (e.g., `npx`, `node`, `python`, `uvx`). Block shell metacharacters, pipes, redirects.
2. **Display clear confirmation UI** when a workspace's MCP server config is first loaded or changed. Show the exact command that will be executed.
3. **Restrict filesystem access.** MCP servers configured per-workspace should only have access to that workspace's data directory. Use Tauri's capability system to enforce this.
4. **Never spawn MCP servers automatically on workspace load.** Require explicit user action ("Connect to MCP servers" button with confirmation).
5. **Validate tool schemas from MCP servers.** Do not trust tool definitions from external servers without validation. Check parameter types, reject excessive permissions.
6. **Log all MCP server spawns** with the full command and arguments for audit purposes.

**Warning signs:**
- MCP server config containing shell operators (`|`, `>`, `;`, `&&`)
- MCP servers requesting tools that access paths outside the workspace
- Unexpected processes appearing after workspace load
- MCP servers accessing network resources without user awareness

**Phase to address:** Phase 2 (MCP framework) -- security model must be defined before the first MCP server is spawned.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Global DB path in backend `main.py` (`db = DatabaseManager()`) | Simple, works for single workspace | Every new endpoint inherits the global; refactoring later touches every file | Never -- refactor to workspace-routed DB before adding workspace features |
| Keeping dual audio systems (`audio/` and `audio_v2/`) | Avoids risky refactor now | Two systems to workspace-scope for recording paths; doubles testing surface | Acceptable for v0.3.0 IF workspace scoping is applied to the active system only and the inactive system is marked deprecated |
| Hardcoding `meeting_minutes.sqlite` filename | Simple path resolution | Must be renamed/parametrized for per-workspace DBs; grep shows 6+ references in `manager.rs` alone | Never -- parameterize from the start |
| Module-level `SummaryProcessor()` in Python backend | Simple initialization | Holds its own `DatabaseManager()` instance separate from the endpoint-level `db`; two DB connections that may diverge if workspace-routed | Fix before workspace support -- SummaryProcessor must use the same workspace-routed DB |
| Storing API keys in the per-workspace database | Simpler schema; no need for a separate global store | Users must re-enter API keys for each workspace; or keys silently leak across workspaces during migration | Only acceptable if users explicitly want per-workspace keys; otherwise, extract to a global config |

## Integration Gotchas

Common mistakes when connecting to MCP servers and managing workspace file systems.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| MCP stdio transport | Writing non-JSON-RPC data to the server's stdin (debug logs, prompts) | The client MUST NOT write anything to stdin that is not a valid MCP JSON-RPC message. Use stderr for all logging. Messages are newline-delimited and MUST NOT contain embedded newlines. ([MCP Spec - Transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)) |
| MCP initialization handshake | Sending tool calls before the `initialize` handshake completes | MCP requires a capability negotiation via `initialize` request/response, followed by `initialized` notification, before any other messages. Skipping this causes silent failures. |
| MCP server stderr | Treating stderr output as error conditions | The MCP spec says the client MAY capture stderr but SHOULD NOT assume it indicates errors. Stderr is for informational/debug logging. Route it to app logs, do not surface it as errors to users. |
| Per-workspace SQLite (WAL mode) | Opening all workspace databases simultaneously at startup | Each WAL-mode SQLite DB needs 3 file descriptors (.db, -wal, -shm). With many workspaces, this exhausts process limits. Open only the active workspace's DB; close others. ([SQLite WAL docs](https://sqlite.org/wal.html)) |
| Obsidian vault path (MCP sync target) | Using `String` path manipulation instead of `PathBuf` | Obsidian vault paths may contain spaces, unicode characters, or platform-specific separators. Always use `std::path::PathBuf` and Tauri's path APIs. Known Tauri bug with spaces on Windows ([issue #6431](https://github.com/tauri-apps/tauri/issues/6431)). |
| Backend workspace routing | Passing workspace ID only in the URL path | URL-only routing misses background tasks (like `process_transcript_background`) that run after the request completes. The workspace ID must be captured and threaded into the background task closure. |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Opening all workspace SQLite DBs at startup | Slow startup, high memory baseline | Lazy-open: only connect to the active workspace's DB. Close previous workspace's pool on switch. | At 10+ workspaces with WAL mode (30+ file descriptors, connection pools per DB) |
| SQLite WAL checkpoint starvation | WAL file grows unbounded, disk fills up | Schedule periodic `PRAGMA wal_checkpoint(TRUNCATE)` on the active workspace DB. Run checkpoint on workspace close. Already implemented for single DB in `DatabaseManager::cleanup()` -- must replicate per workspace. ([SQLite WAL](https://sqlite.org/wal.html)) | When a workspace DB has sustained concurrent reads (e.g., live transcript search during recording) |
| MCP server process accumulation | Memory usage grows, system slows | Implement idle timeout for MCP servers. Track process count. Set per-workspace limit (e.g., max 3 MCP servers). | At 5+ workspaces each with 2-3 MCP servers (10-15 child processes) |
| Full-text search across workspaces | Hangs UI, spins all cores | Never search across all workspace DBs simultaneously. Search only the active workspace. Offer explicit "search all workspaces" with async/pagination. | At 50+ meetings per workspace, 5+ workspaces |
| Copying large audio files during workspace migration | Migration takes minutes, user thinks app froze | Move (rename), do not copy, when source and destination are on the same filesystem. Show progress indicator for cross-device moves. | When a workspace has 10+ meetings with recordings (each 50-500MB) |

## Security Mistakes

Domain-specific security issues beyond general application security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing API keys in per-workspace SQLite without encryption | Workspace files shared between users expose API keys in plaintext | Use OS keychain (macOS Keychain, Windows Credential Manager) for API keys. Store only a key reference in the workspace DB. Or encrypt at rest with a workspace-specific key. |
| MCP server config accepting arbitrary `command` strings | Remote code execution via shared workspace configs | Validate commands against an allowlist. Display full command for user confirmation before first spawn. Never auto-spawn on workspace import. |
| MCP servers inheriting parent process environment | API keys, secrets, PATH entries leak to MCP server processes | Spawn MCP servers with a sanitized environment. Only pass explicitly-declared env vars from workspace config. |
| No authentication between Meetily and MCP servers (stdio) | MCP spec's stdio transport has no auth layer -- the trust boundary is process isolation | Restrict MCP server filesystem access to the workspace directory. Monitor tool invocations. Log all tool calls for audit. |
| Workspace config files stored as unvalidated JSON | Path traversal via malicious workspace config (`"db_path": "../../other_workspace/meetings.db"`) | Validate all paths in workspace config. Reject paths with `..` components. Use Tauri's path traversal protection. All workspace data must reside under the workspace root directory. |

## UX Pitfalls

Common user experience mistakes when adding workspaces and MCP.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Requiring workspace setup before first use | New users face a wall of configuration before they can record their first meeting | Auto-create a "Default" workspace on first launch. Everything works as before. Workspace management is a settings/power-user feature. |
| Making workspace switching slow (DB close + open + UI refresh) | Users avoid workspaces because switching feels heavy | Pre-validate the target workspace's DB on switch. Show a lightweight loading indicator. Keep workspace metadata (name, icon, meeting count) in a global index for instant sidebar display. |
| Showing MCP errors as raw JSON-RPC error messages | Users see `{"code": -32602, "message": "Invalid params"}` and have no idea what to do | Map MCP error codes to human-readable messages. Provide "Retry" and "Open MCP Settings" actions. Log raw errors for debugging. |
| Auto-spawning MCP servers on workspace open without user awareness | Users don't understand why random processes are running | Show clear indicator when MCP servers are running per workspace. Provide a "Connected Services" panel. Require manual "Connect" action for new MCP configs. |
| Losing the "it just works" single-workspace simplicity | Power users love workspaces but casual users are confused by them | Keep a single-workspace mode as the default. Only show workspace UI after the user creates their second workspace. No workspace picker until there are 2+ workspaces. |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Workspace switching:** Often missing cleanup of the previous workspace's DB connection pool -- verify WAL checkpoint runs on switch, connection pool closes, and no stale references remain in recording state.
- [ ] **MCP server lifecycle:** Often missing Windows-specific cleanup -- verify child processes actually terminate on Windows (not just macOS). Test with `tasklist` / Task Manager after app close and crash.
- [ ] **Data migration:** Often missing settings separation -- verify API keys and model configs migrate to a global store, not duplicated into each workspace DB.
- [ ] **Per-workspace audio folders:** Often missing cross-platform path handling -- verify workspace names with spaces, unicode, and special characters produce valid folder paths on all three platforms.
- [ ] **MCP tool invocation:** Often missing user consent flow -- verify the user is prompted before any MCP tool executes, especially tools that write files or access the network. The MCP spec REQUIRES explicit user consent for tool invocation.
- [ ] **Workspace deletion:** Often missing cascade cleanup -- verify deleting a workspace removes its SQLite DB, WAL/SHM files, audio folder, MCP server configs, and any running MCP server processes.
- [ ] **Backend workspace routing:** Often missing background task scoping -- verify `process_transcript_background` and `SummaryProcessor` use the correct workspace DB, not the global instance.
- [ ] **Search functionality:** Often missing workspace scoping -- verify `search_transcripts` only searches within the active workspace, not across all databases.

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Cross-workspace data leakage (meetings in wrong workspace) | MEDIUM | Add a `workspace_id` column to the meetings table. Write a repair script that re-associates meetings with the correct workspace based on `folder_path` or creation date. Export affected meetings to JSON, reimport into correct workspace. |
| Orphaned MCP server processes | LOW | On next startup, read PID file from previous session. Kill any matching processes. Warn user if orphaned processes were found. |
| Corrupted workspace DB during migration | HIGH | Keep the original single DB as a backup (never delete it during migration). Provide a "Reset workspace" option that re-runs migration from the backup. Store the backup path in global config. |
| API keys lost during workspace split | MEDIUM | If keys were in the old single DB, they can be recovered from the backup. Provide a "Restore settings from backup" UI option. In the worst case, users re-enter API keys (annoying but not catastrophic). |
| MCP server spawning arbitrary commands | HIGH | Implement command audit logging immediately. If a malicious command was executed, the log provides forensic evidence. Add post-incident: command allowlist, explicit user approval, sandboxed execution. |
| WAL file grows unbounded (checkpoint starvation) | LOW | Run `PRAGMA wal_checkpoint(TRUNCATE)` manually via a maintenance command. Add automatic periodic checkpointing. The existing `DatabaseManager::cleanup()` pattern already handles this -- replicate for each workspace DB. |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Single-DB assumption baked everywhere | Phase 1: Database abstraction layer | All existing tests pass through the workspace-routed DB. Grep shows zero direct `DatabaseManager()` instantiations outside the workspace manager. |
| Workspace switching during recording | Phase 1: Workspace model + UI rules | Attempt to switch workspace while recording. UI should block the switch with a clear message. |
| MCP server zombie/orphan processes | Phase 2: MCP process manager | Kill the app via `kill -9` (macOS) or Task Manager (Windows). Verify no MCP server processes remain. Restart app and verify stale processes are cleaned up. |
| Data migration from single to multi-DB | Phase 1: Migration system | Run migration on a snapshot of a real pre-v0.3.0 database. Verify meeting count, transcript count, settings, and API keys all survive. Run it twice to verify idempotency. |
| MCP server security (arbitrary command execution) | Phase 2: MCP security model | Configure a workspace with a malicious MCP command (e.g., `rm -rf /tmp/test`). Verify the command is rejected or requires explicit user confirmation. |
| Cross-workspace data leakage | Phase 1: Workspace isolation | Create two workspaces with different meetings. Verify each workspace's meeting list, search results, and settings are fully isolated. |
| WAL checkpoint starvation | Phase 1: Per-workspace DB lifecycle | Record a long meeting (sustained writes + reads). Check WAL file size stays bounded. Verify checkpoint runs on workspace switch. |
| Audio file path cross-platform issues | Phase 1: Workspace filesystem | Create workspaces with names containing spaces, unicode (e.g., "My Meetings", "Reuniones"), and special chars. Verify audio recording and playback work on macOS at minimum. |
| API key exposure in workspace files | Phase 1: Settings architecture | Inspect a workspace's SQLite file. Verify API keys are not stored in it (they should be in global config or OS keychain). |
| MCP server environment leakage | Phase 2: MCP process spawning | Spawn an MCP server. From within it, check `process.env` or `os.environ`. Verify it does not contain the parent app's API keys or sensitive data. |

## Sources

**MCP Specification and Security:**
- [MCP Specification 2025-11-25 - Transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) -- HIGH confidence, authoritative spec
- [MCP Security Survival Guide (Towards Data Science)](https://towardsdatascience.com/the-mcp-security-survival-guide-best-practices-pitfalls-and-real-world-lessons/) -- MEDIUM confidence, well-researched article
- [Red Hat - MCP Security Risks and Controls](https://www.redhat.com/en/blog/model-context-protocol-mcp-understanding-security-risks-and-controls) -- MEDIUM confidence
- [Nearform - MCP Tips, Tricks and Pitfalls](https://nearform.com/digital-community/implementing-model-context-protocol-mcp-tips-tricks-and-pitfalls/) -- MEDIUM confidence
- [CData - MCP Limitations](https://www.cdata.com/blog/navigating-the-hurdles-mcp-limitations) -- LOW confidence (single source)

**MCP Process Management Issues:**
- [MetaMCP Issue #128 - Child processes not cleaned up](https://github.com/metatool-ai/metamcp/issues/128) -- HIGH confidence (primary source)
- [Claude Code Issue #15211 - Windows MCP cleanup](https://github.com/anthropics/claude-code/issues/15211) -- HIGH confidence (primary source)
- [DeployStack Process Manager docs](https://docs.deploystack.io/development/satellite/process-management) -- MEDIUM confidence
- [Tauri Issue #4949 - Windows child process kill failure](https://github.com/tauri-apps/tauri/issues/4949) -- HIGH confidence (primary source)

**MCP Rust SDK:**
- [Official Rust SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk) -- HIGH confidence
- [rmcp docs](https://docs.rs/crate/rmcp/latest) -- HIGH confidence

**SQLite and Database Migration:**
- [SQLite WAL Mode Documentation](https://sqlite.org/wal.html) -- HIGH confidence, authoritative
- [SQLite Concurrent Writes](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/) -- MEDIUM confidence
- [Turso - Per-User SQLite Databases](https://turso.tech/blog/give-each-of-your-users-their-own-sqlite-database) -- MEDIUM confidence
- [Declarative Schema Migration for SQLite](https://david.rothlis.net/declarative-schema-migration-for-sqlite/) -- MEDIUM confidence

**Tauri and Cross-Platform:**
- [Tauri v2 Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/) -- HIGH confidence
- [Tauri v2 File System Plugin](https://v2.tauri.app/plugin/file-system/) -- HIGH confidence
- [Tauri Issue #6431 - Spaces in file paths on Windows](https://github.com/tauri-apps/tauri/issues/6431) -- HIGH confidence
- [Tauri Discussion #3273 - Kill process on exit](https://github.com/tauri-apps/tauri/discussions/3273) -- MEDIUM confidence
- [Rust std::process::Child docs](https://doc.rust-lang.org/std/process/struct.Child.html) -- HIGH confidence

**Codebase Analysis:**
- `backend/app/db.py` -- Direct inspection, global `DatabaseManager` with hardcoded path
- `backend/app/main.py` -- Direct inspection, module-level `db = DatabaseManager()` and `SummaryProcessor`
- `frontend/src-tauri/src/database/manager.rs` -- Direct inspection, single SqlitePool, WAL handling
- `frontend/src-tauri/src/database/setup.rs` -- Direct inspection, single AppState initialization
- `frontend/src-tauri/src/state.rs` -- Direct inspection, single db_manager field
- `frontend/src-tauri/src/lib.rs` -- Direct inspection, global recording flag, command registration
- `frontend/src-tauri/migrations/` -- 10 migration files for schema evolution

---
*Pitfalls research for: Meetily v0.3.0 Workspaces + MCP Integration*
*Researched: 2026-02-01*
