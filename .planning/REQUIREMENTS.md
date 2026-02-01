# Requirements: Meetily v0.3.0

**Defined:** 2026-02-01
**Core Value:** Each workspace is a self-contained meeting context — own storage, own LLM personality, own connected tools — so meeting notes are automatically organized, formatted, and delivered where they belong.

## v1 Requirements

### Workspaces

- [ ] **WS-01**: User can create a new workspace with a name and icon/color
- [ ] **WS-02**: User can rename an existing workspace
- [ ] **WS-03**: User can delete a workspace (with confirmation, deletes all workspace data)
- [ ] **WS-04**: Each workspace has its own isolated folder containing SQLite DB, audio files, config, and meeting notes
- [ ] **WS-05**: User can switch active workspace via sidebar dropdown
- [ ] **WS-06**: Workspace switching is blocked while recording is active
- [ ] **WS-07**: Existing meetings migrate into a "Default" workspace on first launch after upgrade
- [ ] **WS-08**: User can configure a custom system prompt per workspace that controls LLM summary style
- [ ] **WS-09**: User can set workspace context fields (Linear team name, Obsidian vault path, project description)
- [ ] **WS-10**: Workspace settings page accessible from workspace dropdown or sidebar

### Meeting Notes

- [ ] **MN-01**: LLM generates structured markdown meeting notes with sections (summary, key decisions, action items, discussion points)
- [ ] **MN-02**: User can fill in meeting metadata (participants, date, time) before generating notes
- [ ] **MN-03**: Meeting note files follow consistent naming convention: `<workspace-name>-<YYYY-MM-DD>.md`
- [ ] **MN-04**: Meeting notes saved to workspace folder as markdown files
- [ ] **MN-05**: Meeting notes simultaneously saved to configured Obsidian vault path (if set) with YAML frontmatter
- [ ] **MN-06**: Action items extracted from meeting with assignee mapping from participant list
- [ ] **MN-07**: User can select a meeting template per workspace (standup, retro, 1:1, brainstorm, custom)
- [ ] **MN-08**: Meeting templates define the structure and focus areas for LLM-generated notes
- [ ] **MN-09**: Obsidian vault export includes YAML frontmatter (participants, date, workspace, tags)

### MCP Integration

- [ ] **MCP-01**: Generic MCP client framework using rmcp SDK connects to any stdio-based MCP server
- [ ] **MCP-02**: Per-workspace MCP server configuration stored as JSON (server name, command, args, env vars)
- [ ] **MCP-03**: User can add, edit, and remove MCP server configs per workspace via settings UI
- [ ] **MCP-04**: User can manually trigger MCP tool invocation (e.g., "create Linear issue from action items")
- [ ] **MCP-05**: MCP server processes are properly managed (spawn on demand, cleanup on workspace switch/app close)
- [ ] **MCP-06**: Security: User sees consent prompt before first MCP server spawn per workspace
- [ ] **MCP-07**: Security: MCP server commands validated against allowlist patterns
- [ ] **MCP-08**: MCP config includes workspace context fields injected into LLM calls (team, vault, project info)
- [ ] **MCP-09**: MCP server connection status visible in workspace settings

### UI

- [ ] **UI-01**: Workspace switcher dropdown in sidebar header with active workspace indicator
- [ ] **UI-02**: Workspace creation flow (name, optional icon/color)
- [ ] **UI-03**: Workspace settings page with system prompt editor, context fields, template selection
- [ ] **UI-04**: MCP configuration panel: add/edit/remove servers, view connection status, test connections
- [ ] **UI-05**: Meeting metadata form (participants, date, time) shown before note generation
- [ ] **UI-06**: Action items displayed in meeting view with assignee badges
- [ ] **UI-07**: Visual indicator of active workspace in sidebar and window title
- [ ] **UI-08**: MCP sync button per meeting with tool selection (which MCP tool to invoke)

## v2 Requirements

### Workspaces

- **WS-V2-01**: Workspace export/import for backup and portability
- **WS-V2-02**: Workspace search across all workspaces
- **WS-V2-03**: Workspace-level analytics (meetings per week, common action items)

### MCP

- **MCP-V2-01**: Automatic MCP sync after meeting ends (configurable per workspace)
- **MCP-V2-02**: MCP tool results displayed inline in meeting view
- **MCP-V2-03**: MCP server health monitoring and auto-restart

### Meeting Notes

- **MN-V2-01**: Meeting note version history (diff between regenerations)
- **MN-V2-02**: Collaborative editing of generated notes before export
- **MN-V2-03**: Custom meeting note templates with user-defined sections

## Out of Scope

| Feature | Reason |
|---------|--------|
| Multi-user / sharing | Personal tool — single user only |
| Cloud sync between devices | Local-first architecture — no cloud storage |
| Real-time MCP actions during meetings | Post-meeting sync only for v1 — simplicity |
| MCP server hosting | User provides their own MCP servers |
| Automatic MCP sync | Manual trigger only for v1 — user controls data flow |
| Workspace permissions / access control | Single user — no auth needed |
| MCP HTTP/SSE transport | Stdio transport only for v1 — covers most MCP servers |
| Recording across workspaces | Recording locked to active workspace |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| _(populated during roadmap creation)_ | | |

**Coverage:**
- v1 requirements: 30 total
- Mapped to phases: 0 (pending roadmap)
- Unmapped: 30

---
*Requirements defined: 2026-02-01*
*Last updated: 2026-02-01 after initial definition*
