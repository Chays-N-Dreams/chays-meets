# Phase 1: Workspace Filesystem + DB Isolation - Context

**Gathered:** 2026-02-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Per-workspace filesystem structure, SQLite database isolation, and migration of existing meetings into a Default workspace. This phase builds the infrastructure foundation — no UI, no MCP, no meeting notes. Just the data layer that everything else depends on.

</domain>

<decisions>
## Implementation Decisions

### Folder structure
- Workspace folders use UUID names with a `manifest.json` inside containing the human-readable name and metadata
- Each workspace folder is fully self-contained: `db.sqlite`, `audio/`, `notes/`, `config.json`, `mcp-config.json`
- Workspaces root directory is user-configurable, defaulting to standard Tauri app data path (`~/Library/Application Support/Meetily/workspaces/`)
- Global registry file (`workspaces.json`) at the workspaces root for fast lookup, but app can rebuild it by scanning directory and reading each manifest if registry is corrupted

### Database pool management
- Only the active workspace's SQLite pool is open at any time — close pool on switch, open new one
- Frontend state (meeting list, current meeting, sidebar state) is cached per workspace and restored on return
- Global settings (LLM API keys, audio device preferences, app theme) stored in a separate `global.sqlite` outside workspace folders — workspace DBs only contain workspace-specific data (meetings, transcripts, summaries)

### Workspace identity
- Each workspace has a UUID (folder name) and manifest metadata: display name, optional emoji icon, accent color, description field, created date, last-modified date
- All workspaces are equal — no protected "Default" workspace. Migrated meetings go into a workspace named "Default" but it can be renamed or deleted like any other
- If user deletes all workspaces, app shows a "Create your first workspace" empty state — no recording possible without a workspace
- Workspace order in sidebar is user-customizable via drag-to-reorder (order stored in global registry)

### Claude's Discretion
- DB migration strategy: independent per-workspace migrations vs shared schema version (Claude picks based on safety and complexity)
- Additional manifest metadata fields beyond the basics discussed
- Migration safety mechanisms (backup of original DB before migration)
- Exact schema for `workspaces.json` registry file
- How to handle workspace root directory change (move existing workspaces or leave in place)

</decisions>

<specifics>
## Specific Ideas

- The "both" approach to workspace discovery (registry + directory scan fallback) is important for resilience — if the registry gets corrupted, the app should self-heal
- User wants to browse workspace folders on disk, so the internal structure should be clean and discoverable (manifest.json explains what the folder is)
- Cached per-workspace state means switching back to a workspace should feel instant — you're right where you left off

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-workspace-filesystem-db-isolation*
*Context gathered: 2026-02-01*
