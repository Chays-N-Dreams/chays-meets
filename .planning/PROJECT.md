# Meetily v0.3.0 — Workspaces + MCP

## What This Is

Meetily is a privacy-first AI meeting assistant that captures, transcribes, and summarizes meetings locally. This milestone adds **workspace isolation** and **MCP (Model Context Protocol) integration** so each project/team gets its own context, system prompt, and connected tools — with meeting notes automatically saved as markdown files to configurable destinations like Obsidian vaults.

## Core Value

Each workspace is a self-contained meeting context — own storage, own LLM personality, own connected tools — so meeting notes are automatically organized, formatted, and delivered where they belong without manual effort.

## Requirements

### Validated

- ✓ Audio capture (mic + system) with professional mixing — existing
- ✓ Local Whisper transcription with GPU acceleration — existing
- ✓ Real-time transcript display during recording — existing
- ✓ LLM-based meeting summarization (Ollama, Claude, Groq, OpenRouter) — existing
- ✓ Meeting storage and retrieval (SQLite) — existing
- ✓ Cross-platform audio device support (macOS, Windows, Linux) — existing
- ✓ Desktop app with Tauri (Rust + Next.js) — existing
- ✓ Audio recording with WAV output — existing
- ✓ Voice Activity Detection filtering for transcription — existing
- ✓ Configurable audio devices and LLM providers — existing

### Active

**Workspaces:**
- [ ] Create, rename, and delete workspaces
- [ ] Sidebar dropdown for switching active workspace
- [ ] Full filesystem isolation per workspace (own folder, DB, audio, config)
- [ ] Custom system prompt per workspace (controls LLM summary style + context)
- [ ] Workspace-specific context fields (Linear team, Obsidian vault path, etc.)
- [ ] Meeting metadata form (participants, date, time) before generating notes
- [ ] Consistent meeting note file naming: `<team>-<meeting-date>.md`
- [ ] Default workspace for backward compatibility with existing meetings

**MCP Integration:**
- [ ] Generic MCP client framework (any MCP server can be connected)
- [ ] Per-workspace MCP server configuration (JSON config)
- [ ] UI editor for MCP server config (add/edit/remove servers, set context fields)
- [ ] Manual sync trigger — user clicks to push meeting notes to connected platforms
- [ ] Save meeting note markdown files directly to configured Obsidian vault path
- [ ] LLM context injection from workspace config (team, vault, project info)

**Meeting Notes Enhancement:**
- [ ] LLM generates structured markdown meeting notes
- [ ] Meeting note files saved locally in workspace folder AND to Obsidian vault
- [ ] Action item extraction with assignee mapping from participant list
- [ ] Meeting templates per workspace (standup, retro, 1:1, brainstorm)

**UI Polish:**
- [ ] Workspace switcher in sidebar
- [ ] Meeting metadata input form
- [ ] MCP server configuration panel
- [ ] Workspace settings page (system prompt, context fields, templates)
- [ ] Visual indicator of active workspace and connected MCP servers

### Out of Scope

- Multi-user/sharing — personal tool, single user only
- Automatic MCP sync — manual trigger only for v1 (auto-sync deferred)
- Real-time MCP actions during meetings — post-meeting sync only
- Cloud sync between devices — local-first architecture stays
- MCP server hosting — user provides their own MCP servers
- Workspace permissions/access control — single user, no auth needed

## Context

**Existing Architecture:**
- Three-tier: Tauri desktop (Rust + Next.js) → FastAPI backend → LLM providers
- Audio pipeline: cpal capture → ring buffer mixing → VAD filtering → Whisper transcription
- Storage: SQLite via sqlx (Rust) and aiosqlite (Python), single DB currently
- State: React Context (frontend), Arc<RwLock<T>> (Rust)

**Technical Debt to Address:**
- Dual audio systems (audio/ and audio_v2/) — may need cleanup before workspace scoping
- Legacy lib_old_complex.rs (2,437 lines) — should be removed
- Single SQLite DB assumed everywhere — needs workspace-scoping

**MCP Protocol:**
- MCP defines a standard client-server protocol for LLM tool use
- Servers expose tools, resources, and prompts via JSON-RPC over stdio or HTTP
- Configuration follows claude_desktop_config.json pattern: server name, command, args, env
- Per-workspace configs extend this with context fields (team, vault path, etc.)

**User Workflow:**
1. Create workspace for a team/project (e.g., "Alpha Team")
2. Configure system prompt: "Summarize meetings for Alpha Team. Focus on action items and decisions."
3. Add MCP servers: Linear (team: Alpha), Obsidian (vault: ~/Vaults/work/alpha/)
4. Add meeting metadata: participants, date, time
5. Record meeting → transcribe → generate summary
6. LLM writes `alpha-team-2026-02-01.md` to workspace folder and Obsidian vault
7. User clicks "Sync to Linear" to create issues from action items

## Constraints

- **Architecture**: Must integrate with existing Tauri + FastAPI architecture — no rewrites
- **Privacy**: All processing stays local (transcription, summarization) — MCP servers are user-provided
- **Platform**: macOS primary, Windows/Linux secondary — workspace paths must be cross-platform
- **Storage**: SQLite per workspace — no shared state between workspaces except global settings
- **MCP Protocol**: Follow official MCP specification for client implementation
- **Backward Compatibility**: Existing meetings must be accessible in a "Default" workspace

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Full filesystem isolation per workspace | Clean separation, easy backup/export, no cross-contamination | — Pending |
| Generic MCP client (not specific integrations) | Future-proof, user configures any MCP server | — Pending |
| Manual sync trigger (not automatic) | User controls when data leaves the app, simpler v1 | — Pending |
| JSON + UI config for MCP | Power users edit JSON, casual users use UI | — Pending |
| Per-workspace SQLite DB | Strongest isolation, simplest migration path | — Pending |
| Meeting note files saved to Obsidian vault directly | Obsidian handles its own sync, we just write files | — Pending |
| Sidebar dropdown for workspace switching | Minimal UI change, fits existing layout | — Pending |

---
*Last updated: 2026-02-01 after initialization*
