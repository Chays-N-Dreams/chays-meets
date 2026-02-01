# Project Research Summary

**Project:** Meetily v0.3.0 — Workspaces + MCP Integration
**Domain:** Desktop meeting assistant with workspace isolation and MCP client integration
**Researched:** 2026-02-01
**Confidence:** MEDIUM-HIGH

## Executive Summary

Meetily v0.3.0 transforms the single-database meeting assistant into a workspace-isolated system with MCP (Model Context Protocol) integration. The research reveals that workspace isolation is architecturally straightforward but requires careful migration from the current single-database design. The winning approach uses per-workspace SQLite databases with filesystem isolation, wrapped in a `WorkspaceDbPool` that manages connection pools dynamically. MCP integration via the official `rmcp` Rust SDK adds generic tool connectivity with minimal dependencies.

The recommended architecture maintains Meetily's existing audio pipeline and Whisper integration unchanged while introducing three new subsystems: workspace management, MCP client framework, and meeting note generation. The critical differentiator is **per-workspace custom system prompts** combined with **workspace context injection** — allowing the same meeting assistant to behave like different AI personalities depending on which team's workspace is active. No competitor (Granola, Fireflies, Otter, Krisp) offers this level of workspace-specific LLM customization.

The primary risk is the database migration from single to multi-database. Five critical pitfalls must be addressed: (1) single-DB assumptions baked throughout both Tauri and Python codebases, (2) workspace switching during active recording causing data corruption, (3) MCP server child processes becoming zombies/orphans, (4) migration losing existing user data, and (5) MCP server security risks from arbitrary process spawning. All are preventable with proper abstraction layers, process lifecycle management, and migration-first design. The roadmap must prioritize workspace database isolation as Phase 1 foundation work before any MCP or UI features.

## Key Findings

### Recommended Stack

The stack research identifies **one new Rust dependency** (`rmcp` 0.14) and zero frontend/backend dependencies. Workspace isolation leverages existing tools (sqlx, tauri-plugin-store, tauri-plugin-fs), while MCP integration uses the official Rust SDK under the `modelcontextprotocol` GitHub org. The decision to build custom Tauri commands wrapping `rmcp` rather than use third-party Tauri plugins reflects a key insight: immature community plugins (0% docs, unpublished packages) add risk; 50-100 lines of glue code with full control is safer.

**Core technologies:**
- `rmcp` 0.14 (Rust MCP SDK): Official MCP client with stdio transport — the canonical choice for MCP integration in Rust
- `sqlx` (existing): Per-workspace SQLite pools via `HashMap<WorkspaceId, SqlitePool>` — already in the project, supports dynamic pool creation
- `tauri-plugin-store` (existing): Per-workspace settings JSON — path-based store files work out of the box for multi-workspace
- Direct filesystem writes for Obsidian: Write `.md` files to vault path — no library or API needed; this is Obsidian's "file over app" design

**Critical version requirement:**
- `rmcp` must use features `["client", "transport-child-process", "transport-io"]` for stdio MCP servers

**Alternatives rejected:**
- `tauri-plugin-mcp-client` (Sublayer): Not published to crates.io/npm, GitHub-only
- `tauri-plugin-mcp` (airi): 0% documentation coverage, rapid version churn (11 versions in 9 months)
- SQLite `ATTACH DATABASE` for multi-workspace: Shared transaction scope defeats isolation

### Expected Features

The feature research defines a 3-tier priority structure with clear MVP scope. The research synthesized competitor analysis (Granola, Fireflies, Otter, Krisp) with the MCP ecosystem to identify table stakes, differentiators, and anti-features.

**Must have (table stakes):**
- Full filesystem isolation per workspace (own folder, DB, audio, config) — the foundation everything depends on
- Workspace CRUD + sidebar switcher — expected in every workspace-based app (Slack, Notion, VS Code)
- Default workspace migration — existing meetings must survive the upgrade with zero data loss
- Structured markdown meeting notes — the AI meeting market has converged on this (raw transcripts no longer acceptable)
- Action item extraction — every major competitor provides this; users expect "what do I need to do?" answered automatically
- Meeting metadata (date, participants) — basic record-keeping; without it, notes are orphaned context

**Should have (competitive differentiators):**
- Per-workspace custom system prompt — **Meetily's killer feature**; no competitor offers per-team LLM personality customization
- Workspace context fields for LLM injection — inject structured context (team, project, vault path) automatically
- Save meeting notes directly to Obsidian vault — Obsidian users (target audience for local-first tool) desperately want this
- Generic MCP client framework — future-proof; works with any MCP server without code changes
- MCP config UI editor — Claude Desktop requires manual JSON editing; visual editor is a significant UX advantage
- Manual sync trigger per MCP server — explicit user control over when data leaves the app (privacy-first principle)

**Defer (v2+, explicitly NOT v0.3.0):**
- Automatic MCP sync on meeting end — violates privacy-first; users lose control; creates anxiety about data leakage
- Real-time MCP actions during meetings — adds cognitive load; side effects during recording are risky
- Multi-user workspace sharing — fundamentally changes architecture; Meetily is a personal tool
- Cloud sync between devices — destroys local-first privacy model; requires massive infrastructure
- Calendar integration — OAuth flows, API permissions, timezone complexity; defer until proven need

**Feature dependency insight:**
Filesystem isolation is the root dependency. Per-workspace settings depend on it. MCP config depends on per-workspace settings. Generic MCP client depends on MCP config. The dependency graph is strictly linear for the core workspace features.

### Architecture Approach

The architecture research defines a clean three-subsystem addition to the existing Meetily stack: workspace management, MCP client framework, and note generation orchestration. The existing audio pipeline, Whisper engine, and recording manager remain unchanged. The critical insight is that repositories (meetings, transcripts, summaries) do not change — they continue accepting `&SqlitePool`. The refactor happens at call sites: instead of `app.state::<AppState>().db_manager.pool()`, use `app.state::<WorkspaceState>().workspace_db_pool.active_pool().await`.

**Major components:**
1. **WorkspaceManager** — CRUD workspaces, switch active workspace, manage DB pool cache (`HashMap<WorkspaceId, SqlitePool>` with lazy-init)
2. **McpClientManager** — Spawn/stop MCP server child processes via `rmcp` stdio transport, invoke tools, manage per-workspace connections
3. **NoteGenerator** — Orchestrate LLM summary (existing SummaryService) + workspace context injection + MCP tool calls into structured markdown
4. **WorkspaceContext** (React) — Frontend state for active workspace, workspace list, config; wraps existing SidebarProvider

**Key patterns:**
- Workspace-scoped database pool: `Arc<RwLock<HashMap<String, SqlitePool>>>` with lazy-init and LRU eviction for scaling
- MCP client via stdio: `TokioChildProcess` spawns MCP servers as child processes; lazy connect on first tool invocation
- Workspace filesystem isolation: `~/Library/Application Support/Meetily/workspaces/{workspace-id}/` with own DB, config, audio, notes
- MCP server config following `claude_desktop_config.json` pattern: maximally portable, users can copy configs between Meetily and Claude Desktop

**Critical integration points:**
- WorkspaceManager replaces `AppState.db_manager` — all Tauri commands accessing DB need call site updates
- Recording always targets active workspace — workspace switching BLOCKED while recording (critical for audio pipeline integrity)
- MCP client lifecycle tied to workspace activation — gracefully disconnect previous workspace's servers, lazy-connect new workspace's
- SummaryService becomes workspace-aware — caller (NoteGenerator) provides enhanced prompt from workspace config; SummaryService itself unchanged

### Critical Pitfalls

The pitfalls research identifies five critical risks, all preventable with proper design. The common thread: the current codebase assumes a single database and a single context; workspace isolation requires threading context through every layer.

1. **Single-DB assumption baked throughout both codebases** — The Tauri `AppState` and Python `DatabaseManager` are global singletons accessed everywhere. Moving to per-workspace DBs requires auditing every access point (8+ files in Tauri, 15+ endpoints in Python). **Prevention:** Build a WorkspaceContext abstraction first; migrate all existing code to use it with a "default" workspace; then add multi-workspace support.

2. **Workspace switching during active recording causes data corruption** — Recording pipeline captures DB/path references at start; if workspace switch swaps global state mid-recording, incoming transcripts write to wrong database or fail. **Prevention:** Lock workspace switching while recording is active (UI disables switch with message). Pin recording to workspace at start time.

3. **MCP server child processes become zombies or orphans** — Rust `std::process::Child` has no `Drop` implementation; processes spawned for MCP servers continue running after app crash. Known issue on Windows (Tauri bug #4949, Claude Code issue #15211). **Prevention:** Process supervisor pattern with PID tracking; Windows Job Objects; cleanup in Tauri exit handler; idle timeout.

4. **Migration from single DB to per-workspace DBs loses existing user data** — Users have meetings/transcripts/settings in `meeting_minutes.sqlite`; botched migration loses data permanently. **Prevention:** Auto-create "Default" workspace containing all existing data; never modify original DB in-place; separate global (API keys) vs. per-workspace data upfront; write migration test with real pre-v0.3.0 snapshot.

5. **MCP server security — spawning arbitrary processes from user config** — Per-workspace MCP config specifies commands to spawn; malicious config = code execution. MCP spec itself says tools "represent arbitrary code execution and must be treated with appropriate caution." **Prevention:** Allowlist commands (npx, node, python only); display full command for user confirmation; never auto-spawn on workspace load; restrict filesystem access to workspace directory.

## Implications for Roadmap

Based on research, the dependency structure dictates a strict 3-phase approach. Phase 1 is pure foundation (workspace isolation, migration, database abstraction). Phase 2 adds MCP framework (client, config, process management). Phase 3 delivers user-facing features (note generation, Obsidian sync, UI polish). This ordering prevents the #1 risk (baking single-DB assumptions into new code) and ensures migration happens before feature work.

### Phase 1: Workspace Foundation + Database Migration
**Rationale:** Everything depends on workspace isolation. Building MCP or note features before this leads to single-DB assumptions baked into new code. The architecture research shows repositories stay unchanged if we do this first; defer it and we refactor twice.

**Delivers:**
- Per-workspace SQLite databases with WAL mode
- Workspace CRUD (create, rename, delete)
- Default workspace auto-created with existing meetings
- Workspace switcher in sidebar
- Persistent active workspace state
- Filesystem isolation (`~/Library/Application Support/Meetily/workspaces/{id}/`)

**Addresses features:**
- Filesystem isolation per workspace (must-have)
- Workspace CRUD + switcher (must-have)
- Default workspace migration (must-have)
- Persistent workspace state across restarts (must-have)

**Avoids pitfalls:**
- Single-DB assumption baked everywhere (critical pitfall #1)
- Migration losing user data (critical pitfall #4)
- Cross-workspace data leakage (technical debt)

**Research flags:** Standard patterns; skip phase-specific research. sqlx multi-pool pattern is well-documented.

### Phase 2: MCP Client Framework + Security Model
**Rationale:** MCP integration is independent of note generation. Building the framework first (with security hardening) allows Phase 3 to use it safely. The pitfalls research shows MCP process lifecycle is complex; addressing it early prevents zombie process accumulation during Phase 3 development.

**Delivers:**
- `rmcp`-based MCP client with stdio transport
- Per-workspace MCP server configuration (JSON)
- MCP process lifecycle manager (spawn, health-check, cleanup)
- Platform-specific cleanup (Windows Job Objects, macOS process groups)
- Command validation and user confirmation UI
- MCP connection status display per workspace

**Uses stack:**
- `rmcp` 0.14 with `["client", "transport-child-process", "transport-io"]`
- Existing `serde`/`serde_json` for config parsing

**Addresses features:**
- MCP server configuration JSON (should-have)
- Generic MCP client framework (should-have)
- MCP config UI editor (should-have)

**Avoids pitfalls:**
- MCP server zombie/orphan processes (critical pitfall #3)
- MCP server security — arbitrary command execution (critical pitfall #5)
- Process accumulation at scale (performance trap)

**Research flags:** Moderate complexity; phase-specific research recommended for Windows process cleanup (Job Objects) and security validation patterns.

### Phase 3: Meeting Note Generation + Obsidian Integration
**Rationale:** With workspace foundation (Phase 1) and MCP framework (Phase 2) in place, note generation becomes orchestration: pull workspace config, inject context into LLM, write files, optionally trigger MCP sync. This phase delivers the user-facing value.

**Delivers:**
- Workspace-aware structured markdown generation
- Per-workspace custom system prompt
- Workspace context fields injected into LLM
- Action item extraction with participant mapping
- Meeting metadata form (date, time, participants)
- File saving to workspace folder with consistent naming
- Direct Obsidian vault integration (filesystem write)
- Manual MCP sync trigger (e.g., "Push to Linear")

**Implements architecture:**
- NoteGenerator component (orchestrates SummaryService + MCP)
- File writer with dual output (workspace folder + Obsidian vault)
- Meeting note generation UI with metadata form

**Addresses features:**
- Per-workspace custom system prompt (differentiator, must-have)
- Workspace context fields (differentiator, must-have)
- Structured markdown notes (must-have)
- Action item extraction (must-have)
- Meeting metadata form (must-have)
- Meeting note file saving (must-have)
- Save to Obsidian vault path (differentiator, must-have)
- Manual sync trigger (should-have)

**Avoids pitfalls:**
- Auto-sync privacy violations (anti-feature, explicitly avoided)

**Research flags:** Low complexity; standard patterns. LLM prompt engineering for context injection is domain-specific but well-documented in Meetily's existing summary code.

### Phase 4 (Optional): Polish + Advanced Features
**Rationale:** Post-MVP enhancements after core workspace + MCP story is proven. These features refine the experience but aren't essential for launch.

**Delivers:**
- Meeting templates per workspace (standup, retro, 1:1)
- Participant-to-assignee mapping (fuzzy match)
- Searchable meeting history (FTS5 within workspace)
- Cross-platform testing and edge case handling

**Deferred to v0.4+:**
- Cross-meeting intelligence (vector search)
- MCP Apps UI rendering
- Auto-sync (opt-in)
- Calendar integration via MCP

### Phase Ordering Rationale

- **Phase 1 must come first:** The architecture research shows that repositories are unchanged IF workspace abstraction is built first. Defer it and we refactor every new feature written in Phases 2-3. The dependency graph is strict: MCP config depends on per-workspace settings which depend on filesystem isolation.

- **Phase 2 before Phase 3:** MCP framework is independent of note generation. Building it first with security hardening prevents the pitfall of arbitrary command execution. The process lifecycle complexity (zombie cleanup, platform-specific handling) is isolated to one phase instead of discovered mid-Phase 3.

- **Migration happens in Phase 1, not later:** The pitfalls research emphasizes this. Migrating existing meetings to "Default" workspace before building note generation ensures all code paths are workspace-aware from the start. Retroactive migration would require revisiting every feature.

- **MCP sync is manual-only in v0.3.0:** The feature research explicitly identifies auto-sync as an anti-feature. Manual triggers in Phase 3 validate the MCP integration pattern before considering auto-sync in a future version.

### Research Flags

**Phases needing deeper research during planning:**
- **Phase 2 (MCP Framework):** Windows-specific process cleanup (Job Objects API, testing on Windows 10/11), MCP security validation patterns (command allowlisting, sandboxing approaches)
- **Phase 3 (Note Generation):** Obsidian frontmatter best practices (nested YAML issues, Properties compatibility), action item extraction prompt engineering

**Phases with standard patterns (skip research-phase):**
- **Phase 1 (Workspace Foundation):** sqlx multi-pool pattern is well-documented; filesystem isolation is a design decision; migration patterns are standard SQLite operations
- **Phase 3 (Obsidian Integration):** Direct filesystem writes; no API, no library — trivially simple based on research

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | `rmcp` verified via official docs (docs.rs, GitHub); sqlx multi-pool pattern confirmed; all other dependencies already in project. Version 0.14 release confirmed (2026-01-23). |
| Features | MEDIUM-HIGH | Table stakes validated against 4 major competitors (Granola, Fireflies, Otter, Krisp). Differentiators (per-workspace system prompt, context injection) are novel — no direct competitor data, but grounded in LLM prompt engineering best practices. |
| Architecture | HIGH | Existing codebase analyzed directly; patterns verified against Tauri v2 docs, sqlx docs, MCP spec. Component boundaries map cleanly to existing structure. Critical integration points (WorkspaceManager replacing AppState) audited in current code. |
| Pitfalls | MEDIUM-HIGH | Critical pitfalls #1, #4 verified via codebase inspection (single-DB assumption in `state.rs`, `main.py`). Pitfalls #3, #5 verified via primary sources (Claude Code issue #15211, MetaMCP issue #128, MCP spec security section). Windows-specific cleanup gaps need validation. |

**Overall confidence:** MEDIUM-HIGH

The stack and architecture recommendations are highly confident (verified official sources, inspected codebase). Feature priorities are confident for table stakes (competitor consensus) but medium for differentiators (novel features, no direct comparisons). Pitfall prevention strategies are confident for Tauri/SQLite (well-documented) but medium for Windows MCP cleanup (requires platform-specific testing).

### Gaps to Address

Research was thorough but identified areas needing validation during implementation:

- **Windows MCP process cleanup:** Research confirms the problem (Tauri issue #4949, Claude Code issue #15211) but solutions (Job Objects) require Windows-specific testing. Validate on Windows 10/11 during Phase 2. Fallback: PID file + manual cleanup on startup if Job Objects fail.

- **Workspace deletion cascade:** Research identifies the need (delete DB, WAL/SHM, audio folder, configs, running MCP processes) but cross-platform testing is required. macOS/Linux folder deletion is straightforward; Windows file locking may block deletion of open files. Phase 1 must include manual testing of workspace deletion with active MCP connections.

- **API key storage model:** Research notes the choice between per-workspace (simpler schema) vs. global (better UX). Current codebase stores keys in the single DB. Decision needed in Phase 1: extract to global config (OS keychain preferred) or accept per-workspace re-entry. Research recommendation: global config for provider keys, per-workspace for server-specific keys (MCP server Linear API key).

- **MCP server environment sanitization:** Pitfall #5 prevention requires spawning MCP servers with sanitized environment (no parent process secrets). Research identifies the need but not the implementation pattern. Phase 2 planning should research Rust `std::process::Command::env_clear()` + explicit env passing.

- **Large meeting history migration performance:** Migration from single DB to "Default" workspace involves copying audio files (50-500MB each). Research notes the need for move-not-copy and progress indicators but lacks cross-device testing. Phase 1 should include migration testing with a realistic dataset (10+ meetings, 2GB+ total).

## Sources

### Primary (HIGH confidence)
- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18) — Protocol specification, transports, security model
- [rmcp Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) — API patterns, version 0.14.0 release confirmation
- [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/) — Client creation flow, transport options, ServiceExt trait
- [sqlx Pool documentation](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html) — Pool is Send+Sync+Clone, dynamic creation
- [Tauri v2 State Management](https://v2.tauri.app/develop/state-management/) — Manager API patterns
- [SQLite WAL Mode Documentation](https://sqlite.org/wal.html) — Concurrency, checkpoint behavior
- [Obsidian Developer Documentation](https://docs.obsidian.md/Plugins/Vault) — Vault structure, file-based integration
- Direct codebase inspection: `state.rs`, `main.py`, `manager.rs`, `db.py`, `lib.rs` — Verified single-DB assumption, global state patterns

### Secondary (MEDIUM confidence)
- [Reclaim.ai: Top 18 AI Meeting Assistants 2026](https://reclaim.ai/blog/ai-meeting-assistants) — Competitor feature comparison
- [Granola.ai Official Site](https://www.granola.ai/) + [Granola 2.0 Workspace Features](https://quantumzeitgeist.com/granola-2-0-the-ai-powered-workspace-revolutionizing-team-collaboration/) — Competitor analysis for workspace patterns
- [Claude Code Docs: MCP Configuration](https://code.claude.com/docs/en/mcp) — MCP config JSON format
- [MCP Security Survival Guide (Towards Data Science)](https://towardsdatascience.com/the-mcp-security-survival-guide-best-practices-pitfalls-and-real-world-lessons/) — Security patterns
- [Red Hat: MCP Security Risks and Controls](https://www.redhat.com/en/blog/model-context-protocol-mcp-understanding-security-risks-and-controls) — Risk assessment
- [MetaMCP Issue #128](https://github.com/metatool-ai/metamcp/issues/128) + [Claude Code Issue #15211](https://github.com/anthropics/claude-code/issues/15211) — MCP process cleanup issues
- [Tauri Issue #4949](https://github.com/tauri-apps/tauri/issues/4949) — Windows child process kill failure
- [Turso: Per-User SQLite Databases](https://turso.tech/blog/give-each-of-your-users-their-own-sqlite-database) — Multi-database patterns

### Tertiary (LOW confidence, used for validation only)
- [DEV Community: 5 Essential Features of a Productivity App 2026](https://dev.to/anas_kayssi/5-essential-features-of-a-productivity-app-in-2026-408g) — Workspace UX patterns
- [dannb.org: Obsidian Meeting Note Template](https://dannb.org/blog/2023/obsidian-meeting-note-template/) — Markdown frontmatter examples

---
*Research completed: 2026-02-01*
*Ready for roadmap: yes*
