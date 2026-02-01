# Roadmap: Meetily v0.3.0 — Workspaces + MCP

## Overview

Meetily v0.3.0 transforms the single-database meeting assistant into a workspace-isolated system with MCP integration and structured meeting note generation. The roadmap proceeds foundation-first: workspace filesystem isolation and migration must land before any feature work, since every subsequent phase depends on per-workspace databases and config paths. From there, workspace management UI, workspace configuration, MCP client framework, MCP configuration, meeting note generation, file output with Obsidian integration, meeting templates, and MCP sync triggers each deliver a coherent, independently verifiable capability.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Workspace Filesystem + Database Isolation** - Per-workspace folders, SQLite databases, and migration of existing meetings to Default workspace
- [ ] **Phase 2: Workspace CRUD + Sidebar Switcher** - Create, rename, delete workspaces and switch between them via sidebar dropdown
- [ ] **Phase 3: Workspace Configuration** - Per-workspace system prompts, context fields, and settings page
- [ ] **Phase 4: MCP Client Framework + Security** - Generic MCP client via rmcp SDK with process lifecycle management and security model
- [ ] **Phase 5: MCP Configuration + Status UI** - Per-workspace MCP server config storage, editor UI, context injection, and connection status
- [ ] **Phase 6: Meeting Note Generation + Action Items** - LLM-generated structured markdown notes with metadata form and action item extraction
- [ ] **Phase 7: Meeting Note Files + Obsidian Integration** - File output with consistent naming, workspace folder saving, and Obsidian vault export with frontmatter
- [ ] **Phase 8: Meeting Templates** - Per-workspace meeting templates that define LLM note structure and focus areas
- [ ] **Phase 9: MCP Sync Trigger** - Manual MCP tool invocation from meeting view with tool selection UI

## Phase Details

### Phase 1: Workspace Filesystem + Database Isolation
**Goal**: Every workspace operates on its own isolated filesystem directory and SQLite database, and existing meetings survive the upgrade in a Default workspace
**Depends on**: Nothing (first phase)
**Requirements**: WS-04, WS-07
**Success Criteria** (what must be TRUE):
  1. Each workspace has its own directory under `~/Library/Application Support/Meetily/workspaces/{id}/` containing its own SQLite database, audio folder, config files, and notes folder
  2. On first launch after upgrade, all existing meetings, transcripts, and summaries are accessible in a "Default" workspace with zero data loss
  3. The Rust WorkspaceManager abstraction replaces direct AppState DB access so all database operations are routed through the active workspace's pool
  4. Multiple workspaces can coexist on disk without any data cross-contamination between their databases or files
**Plans**: 4 plans

Plans:
- [ ] 01-01-PLAN.md — Workspace types, filesystem operations, and split migration SQL
- [ ] 01-02-PLAN.md — WorkspaceManager struct with pool lifecycle management
- [ ] 01-03-PLAN.md — Rewire all command handlers from AppState to WorkspaceManager
- [ ] 01-04-PLAN.md — Existing data migration to Default workspace

### Phase 2: Workspace CRUD + Sidebar Switcher
**Goal**: Users can create, manage, and switch between workspaces through the sidebar, with recording-safety guarantees
**Depends on**: Phase 1
**Requirements**: WS-01, WS-02, WS-03, WS-05, WS-06, UI-01, UI-02, UI-07
**Success Criteria** (what must be TRUE):
  1. User can create a new workspace by providing a name and optional icon/color, and it appears immediately in the sidebar dropdown
  2. User can rename a workspace and the name updates across the sidebar and window title
  3. User can delete a workspace (with confirmation dialog) and all its data (DB, audio, config, notes) is removed from disk
  4. User can switch active workspace via sidebar dropdown and the meeting list, settings, and all data reflect the selected workspace
  5. Workspace switching is disabled with a visible message while a recording is in progress
**Plans**: TBD

Plans:
- [ ] 02-01: Workspace CRUD Tauri commands (create, rename, delete)
- [ ] 02-02: Sidebar workspace dropdown and switching logic
- [ ] 02-03: Recording lock for workspace switching and active workspace indicator

### Phase 3: Workspace Configuration
**Goal**: Users can customize each workspace's LLM behavior through system prompts and structured context fields, accessible from a dedicated settings page
**Depends on**: Phase 2
**Requirements**: WS-08, WS-09, WS-10, UI-03
**Success Criteria** (what must be TRUE):
  1. User can write and save a custom system prompt per workspace that changes how the LLM generates meeting summaries
  2. User can set workspace context fields (Linear team name, Obsidian vault path, project description) and these persist across app restarts
  3. Workspace settings page is accessible from the sidebar dropdown or workspace header, showing system prompt editor, context fields editor, and (placeholder for) template selection
**Plans**: TBD

Plans:
- [ ] 03-01: Workspace config schema and persistence (system prompt, context fields)
- [ ] 03-02: Workspace settings page UI (system prompt editor, context fields form)

### Phase 4: MCP Client Framework + Security
**Goal**: Meetily can connect to any stdio-based MCP server as a client, with proper process lifecycle management and security safeguards
**Depends on**: Phase 1
**Requirements**: MCP-01, MCP-05, MCP-06, MCP-07
**Success Criteria** (what must be TRUE):
  1. Meetily can spawn an MCP server process via stdio transport using the rmcp SDK, exchange initialization handshake, and list available tools
  2. MCP server child processes are properly tracked and cleaned up on workspace switch, app close, and after idle timeout (no zombie processes)
  3. Before first MCP server spawn per workspace, user sees a consent prompt showing the full command that will be executed
  4. MCP server commands are validated against an allowlist of approved patterns (e.g., npx, node, python) and rejected commands show an error
**Plans**: TBD

Plans:
- [ ] 04-01: rmcp SDK integration and McpClientManager for stdio transport
- [ ] 04-02: MCP process lifecycle management (spawn, track, cleanup, platform-specific handling)
- [ ] 04-03: MCP security model (consent prompt, command allowlist validation)

### Phase 5: MCP Configuration + Status UI
**Goal**: Users can configure MCP servers per workspace through a visual editor, with connection status feedback and context field injection
**Depends on**: Phase 3, Phase 4
**Requirements**: MCP-02, MCP-03, MCP-08, MCP-09, UI-04
**Success Criteria** (what must be TRUE):
  1. Each workspace stores MCP server configuration as JSON (server name, command, args, env vars) and configs persist across restarts
  2. User can add, edit, and remove MCP server configurations per workspace through a visual settings panel (no manual JSON editing required)
  3. Workspace context fields (team, vault path, project info) are available for injection into LLM calls when generating notes in that workspace
  4. MCP server connection status (connected, disconnected, error) is visible per server in workspace settings
**Plans**: TBD

Plans:
- [ ] 05-01: MCP config schema and per-workspace JSON persistence
- [ ] 05-02: MCP configuration panel UI (add/edit/remove servers, test connections)
- [ ] 05-03: Context field injection pipeline and connection status display

### Phase 6: Meeting Note Generation + Action Items
**Goal**: Users can generate structured, context-aware markdown meeting notes with action items extracted and assigned to participants
**Depends on**: Phase 3
**Requirements**: MN-01, MN-02, MN-06, UI-05, UI-06
**Success Criteria** (what must be TRUE):
  1. LLM generates structured markdown meeting notes with distinct sections: summary, key decisions, action items, and discussion points
  2. User can fill in meeting metadata (participants, date, time) before triggering note generation, and this metadata appears in the generated notes
  3. Action items are automatically extracted from the meeting with assignees mapped from the participant list
  4. Action items are displayed in the meeting view with assignee badges showing who is responsible for each item
**Plans**: TBD

Plans:
- [ ] 06-01: NoteGenerator service with workspace-aware LLM prompt construction
- [ ] 06-02: Meeting metadata form UI and action item extraction with assignee mapping
- [ ] 06-03: Action item display in meeting view with assignee badges

### Phase 7: Meeting Note Files + Obsidian Integration
**Goal**: Meeting notes are automatically saved as markdown files to the workspace folder and optionally to a configured Obsidian vault with proper frontmatter
**Depends on**: Phase 6
**Requirements**: MN-03, MN-04, MN-05, MN-09
**Success Criteria** (what must be TRUE):
  1. Meeting note files follow consistent naming: `<workspace-name>-<YYYY-MM-DD>.md` and are saved to the workspace's notes folder
  2. If the workspace has an Obsidian vault path configured, notes are simultaneously written to that vault directory
  3. Obsidian vault exports include YAML frontmatter with participants, date, workspace name, and tags
**Plans**: TBD

Plans:
- [ ] 07-01: Meeting note file writer with consistent naming and workspace folder output
- [ ] 07-02: Obsidian vault integration with YAML frontmatter generation

### Phase 8: Meeting Templates
**Goal**: Users can select meeting templates per workspace that shape how the LLM structures and focuses generated notes
**Depends on**: Phase 6
**Requirements**: MN-07, MN-08
**Success Criteria** (what must be TRUE):
  1. User can select a meeting template per workspace from built-in options (standup, retro, 1:1, brainstorm) or a custom template
  2. Selected template changes the structure and focus areas of LLM-generated notes (e.g., standup emphasizes blockers and progress; retro emphasizes what went well/poorly)
**Plans**: TBD

Plans:
- [ ] 08-01: Meeting template system (built-in templates, custom template support, LLM prompt integration)

### Phase 9: MCP Sync Trigger
**Goal**: Users can manually push meeting data to connected MCP servers from the meeting view
**Depends on**: Phase 5, Phase 6
**Requirements**: MCP-04, UI-08
**Success Criteria** (what must be TRUE):
  1. User can click a sync button on a meeting to invoke an MCP tool (e.g., "create Linear issue from action items") and see the result
  2. Sync button shows available MCP tools for the workspace and lets the user select which tool to invoke
**Plans**: TBD

Plans:
- [ ] 09-01: MCP tool invocation from meeting view with tool selection UI

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9

Note: Phase 4 depends only on Phase 1 (not Phase 2 or 3), so Phases 2-3 (workspace UI/config) and Phase 4 (MCP framework) could theoretically be parallelized after Phase 1. However, for a solo developer workflow, sequential execution is recommended. Phase 5 depends on both Phase 3 and Phase 4. Phase 9 depends on both Phase 5 and Phase 6.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace Filesystem + DB Isolation | 0/4 | Planning complete | - |
| 2. Workspace CRUD + Sidebar Switcher | 0/3 | Not started | - |
| 3. Workspace Configuration | 0/2 | Not started | - |
| 4. MCP Client Framework + Security | 0/3 | Not started | - |
| 5. MCP Configuration + Status UI | 0/3 | Not started | - |
| 6. Meeting Note Generation + Action Items | 0/3 | Not started | - |
| 7. Meeting Note Files + Obsidian Integration | 0/2 | Not started | - |
| 8. Meeting Templates | 0/1 | Not started | - |
| 9. MCP Sync Trigger | 0/1 | Not started | - |
