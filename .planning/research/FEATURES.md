# Feature Research

**Domain:** Workspace-based AI meeting assistant with MCP integration (Meetily v0.3.0)
**Researched:** 2026-02-01
**Confidence:** MEDIUM-HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete or half-baked for the workspace/MCP milestone.

#### Workspace Features

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Create, rename, delete workspaces | Every workspace-based app (Slack, Notion, VS Code, Obsidian) supports basic CRUD. Without this, workspaces are just a concept. | LOW | Start with create + rename. Delete needs confirmation + data handling (archive vs destroy). |
| Workspace switcher in sidebar | Users expect instant context switching. Slack's workspace switcher, VS Code's workspace selector, and Obsidian's vault switcher all live in the sidebar or header. This is the primary navigation pattern. | LOW | Dropdown at top of sidebar is the standard pattern. Show active workspace name + icon. Keep it to 1 click to switch. |
| Full filesystem isolation per workspace | The entire point of workspaces. Users expect that Team A's data never leaks into Team B. Notion, Obsidian, and VS Code all provide real isolation. Shared-database-with-filters feels fragile and users distrust it. | MEDIUM | Own folder, own SQLite DB, own audio files, own config. This is the foundation -- everything else depends on it. Migrate existing data to a "Default" workspace. |
| Per-workspace settings | Once you have workspaces, users immediately expect per-workspace configuration. Different teams use different LLM providers, different summary styles, different integrations. | MEDIUM | System prompt, LLM provider override, output format preferences. Use workspace config file (JSON or TOML). Inherit from global defaults with workspace overrides. |
| Default workspace for backward compatibility | Users with existing meetings must not lose access. Every app that adds workspaces (Slack, Discord servers, Obsidian vaults) provides a migration path. | LOW | Auto-create "Default" workspace containing all pre-existing meetings. Make it the initially active workspace on first launch after upgrade. |
| Persistent workspace state across app restarts | Users expect the app to remember which workspace was active. This is basic UX. | LOW | Store last-active workspace in global settings. Restore on launch. |

#### Meeting Notes Features

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Structured markdown meeting notes | The AI meeting market has converged: Granola, Otter, Fireflies, Krisp all produce structured notes with sections (summary, key decisions, action items). Raw transcript dumps are no longer acceptable. | MEDIUM | LLM generates markdown with consistent sections. Template-driven via system prompt. This is the core output users judge the product by. |
| Action item extraction | Every major competitor (Granola, Fireflies, Krisp, Fellow, Read.ai) extracts action items. Users expect "What do I need to do after this meeting?" to be answered automatically. This is table stakes in 2026. | MEDIUM | LLM extracts from transcript. Include: task description, assignee (if mentioned), deadline (if mentioned). Mark as TODO items in markdown. Human review is essential -- never auto-assign without confirmation. |
| Meeting metadata (date, participants) | Users need to know who was in the meeting and when it happened. This is basic record-keeping. Without it, meeting notes are orphaned context. | LOW | Form before/after recording: date (auto-filled), time (auto-filled), participants (text list or tags). Store in DB and include in note frontmatter. |
| Meeting note file saving to workspace folder | If workspaces have folders, notes should live there. Users expect to find their files where they belong. | LOW | Save `<team>-<date>.md` to workspace folder. Also store in DB for app-level querying. File is the source of truth for external consumption. |
| Searchable meeting history | Otter, Fireflies, and Granola all provide searchable archives. Users expect to find past meetings by keyword, date, or participant. | MEDIUM | Full-text search across meeting notes within active workspace. SQLite FTS5 is well-suited here. |

#### MCP Features

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| MCP server configuration (JSON) | The MCP ecosystem standardized on JSON config (`mcpServers` format). Power users expect to paste config directly. Claude Desktop, VS Code, and Cursor all use this pattern. | LOW | Follow the `claude_desktop_config.json` pattern exactly: `{ "mcpServers": { "name": { "command": "...", "args": [...], "env": {...} } } }`. Per-workspace config file. |
| Manual sync trigger | Users expect explicit control over when data leaves the app. This is especially important for a privacy-first product. Granola uses Zapier for post-meeting actions; the pattern is "generate notes first, then push when ready." | LOW | Button per MCP server: "Sync to Linear", "Save to Obsidian". User clicks, action executes, result shown. No auto-sync in v1. |

### Differentiators (Competitive Advantage)

Features that set Meetily apart. Not expected by default, but create significant value.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Per-workspace custom system prompt | **This is Meetily's killer differentiator.** No competitor offers per-team LLM personality. A standup for the infrastructure team should produce different notes than a 1:1 with a report. The system prompt controls tone, focus areas, output structure, and injected context (team info, project context). Granola has "Recipes" (templates) but not full system prompt customization with workspace context injection. | MEDIUM | System prompt stored per workspace. Injected into LLM call along with workspace context fields (team name, project, vault path). This is where workspace isolation pays off -- each team gets its own AI assistant personality. |
| Workspace context fields for LLM injection | Inject structured context (Linear team, Obsidian vault path, project name, team members) into the LLM prompt automatically. The LLM knows "this is the Alpha Team standup" without the user repeating it every time. No competitor does this at the workspace level. | LOW | Key-value pairs in workspace config. Merged into system prompt template. Example: `team: Alpha`, `project: Widget Redesign`, `vault: ~/Vaults/work/alpha/`. |
| Save meeting notes directly to Obsidian vault | Obsidian users (the target audience for a local-first tool) desperately want this. Multiple MCP servers exist for Obsidian (mcp-obsidian, obsidian-mcp-server), but writing files directly to a vault path is simpler and more reliable than going through MCP for this specific use case. The file IS the integration. | LOW | Write markdown file to configured vault path. Include YAML frontmatter (date, participants, tags, meeting type). Use flat YAML -- Obsidian Properties does not handle nested YAML well. Obsidian auto-discovers new files. No plugin or API needed. |
| Meeting templates per workspace (standup, retro, 1:1) | Granola has "Recipes" for this. Fellow has meeting templates. But combining templates with workspace context (so the standup template for Team Alpha includes Team Alpha's project info) is unique. Templates control LLM output structure, not just formatting. | MEDIUM | Templates are system prompt variants. Each workspace can have multiple templates. User selects template before generating notes. Template includes section structure + focus areas. Example: standup template focuses on blockers and commitments; retro template focuses on what worked, what did not, action items. |
| Generic MCP client framework | Future-proof architecture. Instead of building Linear integration, Obsidian integration, Slack integration as separate features, build a generic MCP client that works with any MCP server. User configures the servers they want. This scales infinitely without code changes. | HIGH | Implement MCP client using `@modelcontextprotocol/sdk`. Support stdio transport (for local MCP servers like `npx` commands). Per-workspace server config. Discovery of available tools. Manual trigger to call specific tools with meeting context. |
| UI editor for MCP server config | JSON config is power-user territory. A UI that lets users add/edit/remove MCP servers visually (server name, command, args, environment variables) makes MCP accessible to non-technical users. Claude Desktop does not have this -- it requires manual JSON editing. | MEDIUM | Form-based editor that reads/writes the JSON config. Show server status (connected/disconnected). Test connection button. This is a significant UX advantage over raw JSON editing. |
| Participant-to-assignee mapping for action items | When extracting action items, map mentioned names to participant list. "John will handle the deployment" becomes an action item assigned to "John Smith" from the participants list. Krisp shows action items organized by assignee. Granola requires Zapier for task management. | MEDIUM | LLM extracts assignee names from transcript. Fuzzy match against participant list. Present as suggestion -- user confirms or corrects before finalizing. Never auto-assign to external systems without explicit confirmation. |
| Consistent file naming: `<team>-<meeting-date>.md` | Obsidian users organize by naming convention. Automated, consistent naming means meeting notes are immediately findable and sortable in the vault. The Obsidian community strongly favors `YYYY-MM-DD` date prefixes for sortability. | LOW | Format: `<workspace-name>-<YYYY-MM-DD>.md` or `<YYYY-MM-DD>-<workspace-name>-<meeting-type>.md`. User configurable per workspace. Default to date-first for filesystem sortability. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems. Explicitly do NOT build these for v0.3.0.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Automatic MCP sync on meeting end | "I want my notes in Linear/Obsidian automatically!" | Violates privacy-first principle. Users lose control over what leaves the app. Data might be incomplete or wrong (LLM hallucinated an action item). Creates anxiety about accidental data leakage. Granola learned this -- they use manual Zapier triggers, not automatic push. | Manual sync trigger with preview. User reviews notes, clicks "Sync", sees what will be sent, confirms. Add auto-sync as opt-in in a future version after users trust the manual flow. |
| Real-time MCP actions during meetings | "Create Linear issues live while recording!" | Adds cognitive load during meetings. Network errors during recording create UX problems. Side effects during recording are risky (what if the meeting context changes?). Post-meeting is the right time for actions. | Post-meeting sync only. Generate notes, review, then trigger MCP actions. Focus during the meeting should be on listening, not task management. |
| Multi-user workspace sharing | "I want my team to see the same workspace!" | Fundamentally changes the architecture from single-user local-first to multi-user synced. Requires auth, permissions, conflict resolution, real-time sync. Meetily is a personal tool. Adding sharing is a different product. | Keep workspaces personal. Users share outputs (markdown files, synced Linear issues) through existing tools. The file-based output IS the sharing mechanism. |
| Cloud sync between devices | "I want my meeting notes on my phone!" | Destroys the local-first privacy model. Requires cloud infrastructure, accounts, encryption-at-rest, sync conflict resolution. Massive scope increase. | Obsidian Sync handles cross-device sync for vault files. Linear handles cross-device for issues. Meetily writes to these systems -- they handle sync. Meetily stays local. |
| Built-in calendar integration for auto-populating metadata | "Pull meeting title and participants from my calendar!" | Requires OAuth flows with Google/Microsoft, calendar API permissions, dealing with recurring events, timezone complexity. Significant scope for a v0.3.0 feature. | Manual metadata entry for v0.3.0. Populate date/time automatically from system clock. Participant list is manual text entry. Calendar integration can be a future MCP server (calendar MCP servers already exist). |
| Speaker diarization in action item assignment | "Automatically know WHO said what and assign action items based on voice!" | Speaker diarization accuracy varies wildly. Requires enrollment/training per speaker. Misattribution of action items is worse than no attribution. Creates a false sense of accuracy that erodes trust. | Use participant list + LLM context for assignment suggestions. The LLM can infer "John will handle deployment" from transcript text without needing to know which voice said it. |
| MCP server hosting/management | "Run MCP servers for me!" | Meetily is a desktop app, not an infrastructure platform. Hosting MCP servers requires process management, crash recovery, resource monitoring. Users should run their own servers or use remote ones. | Document how to set up common MCP servers (Linear, Obsidian, filesystem). Provide example configs. Link to MCP server registries. The MCP ecosystem handles server distribution. |
| Workspace import/export | "Export my workspace to share with a teammate!" | Implies sharing, which is out of scope. Export format is undefined. Import creates duplication concerns. | Workspaces are folder-based. Users can manually copy folders. Meeting notes are markdown files -- inherently portable. No special export format needed. |

## Feature Dependencies

```
[Filesystem Isolation (workspace folders + per-workspace DB)]
    |
    +-- [Workspace CRUD (create/rename/delete)]
    |       |
    |       +-- [Workspace Switcher UI]
    |       |
    |       +-- [Default Workspace Migration]
    |
    +-- [Per-Workspace Settings (system prompt, config)]
    |       |
    |       +-- [Workspace Context Fields]
    |       |       |
    |       |       +-- [LLM Context Injection]
    |       |
    |       +-- [Meeting Templates per Workspace]
    |       |
    |       +-- [Per-Workspace MCP Config]
    |               |
    |               +-- [MCP Server Configuration UI]
    |               |
    |               +-- [Generic MCP Client Framework]
    |                       |
    |                       +-- [Manual Sync Trigger]
    |
    +-- [Meeting Note File Saving (to workspace folder)]
            |
            +-- [Structured Markdown Generation]
            |       |
            |       +-- [Action Item Extraction]
            |       |       |
            |       |       +-- [Participant-to-Assignee Mapping]
            |       |
            |       +-- [Consistent File Naming]
            |
            +-- [Save to Obsidian Vault Path]

[Meeting Metadata Form]
    |
    +-- [Participant List]
    |       |
    |       +-- [Participant-to-Assignee Mapping]
    |
    +-- [Date/Time Auto-fill]
```

### Dependency Notes

- **Filesystem Isolation is the foundation:** Everything depends on workspaces having their own folders and databases. Build this first or nothing else works properly.
- **Per-Workspace Settings requires Filesystem Isolation:** Settings are stored in workspace config files within workspace folders.
- **MCP Config requires Per-Workspace Settings:** MCP server configs are part of workspace settings.
- **Generic MCP Client requires MCP Config:** The client reads config to know which servers to connect to.
- **Structured Markdown requires Per-Workspace Settings:** The system prompt (from workspace settings) controls markdown output structure.
- **Action Item Extraction requires Structured Markdown:** Action items are a section within the structured notes.
- **Participant-to-Assignee Mapping requires both Meeting Metadata Form AND Action Item Extraction:** Needs participant list to map against extracted names.
- **Save to Obsidian Vault requires Meeting Note File Saving:** It is file saving to a different path.
- **LLM Context Injection requires Workspace Context Fields:** Context fields are the data; injection is the mechanism.

## MVP Definition

### Launch With (v0.3.0 Core)

Minimum viable workspace + notes experience. What is needed to validate the concept.

- [ ] **Filesystem isolation per workspace** -- Own folder, DB, audio, config per workspace. This is the architectural foundation.
- [ ] **Workspace CRUD + switcher** -- Create, rename, delete workspaces. Sidebar dropdown to switch. Persistent last-active state.
- [ ] **Default workspace migration** -- Existing meetings automatically available in "Default" workspace. Zero data loss on upgrade.
- [ ] **Per-workspace custom system prompt** -- The core differentiator. Each workspace configures how the LLM summarizes meetings.
- [ ] **Workspace context fields** -- Key-value pairs (team, project, vault path) injected into LLM calls.
- [ ] **Structured markdown meeting notes** -- LLM generates notes with consistent sections (summary, decisions, action items).
- [ ] **Action item extraction** -- LLM identifies action items with assignee and deadline when mentioned.
- [ ] **Meeting metadata form** -- Date (auto), time (auto), participants (manual entry), meeting type.
- [ ] **Meeting note file saving** -- Save markdown to workspace folder with consistent naming (`<team>-<YYYY-MM-DD>.md`).
- [ ] **Save to Obsidian vault path** -- Write the same markdown file to a configured Obsidian vault directory. Direct filesystem write, no MCP needed for this.

### Add After Validation (v0.3.x)

Features to add once core workspaces are working and users confirm the model.

- [ ] **Meeting templates per workspace** -- Standup, retro, 1:1, brainstorm templates that control LLM output structure. Add after confirming the system prompt approach works well.
- [ ] **Per-workspace MCP server config (JSON)** -- JSON config file per workspace following `mcpServers` format. Power user feature -- add after workspace settings UI exists.
- [ ] **MCP config UI editor** -- Visual editor to add/edit/remove MCP servers. Add after JSON config is proven to work.
- [ ] **Generic MCP client framework** -- `@modelcontextprotocol/sdk` client connecting to configured servers via stdio transport. Add after config is stable.
- [ ] **Manual sync trigger** -- Button to push meeting data to connected MCP servers. Add after MCP client works.
- [ ] **Participant-to-assignee mapping** -- Fuzzy match action item assignees against participant list. Add after action item extraction is validated.

### Future Consideration (v0.4+)

Features to defer until workspace + MCP model is established.

- [ ] **Cross-meeting intelligence** -- Query across meeting history ("What did we decide about pricing?"). Granola 2.0 does this. Requires significant vector search infrastructure.
- [ ] **MCP Apps UI rendering** -- MCP Apps extension (January 2026) allows servers to return interactive UI components. Future opportunity but high complexity.
- [ ] **Automatic sync (opt-in)** -- After users trust manual sync, offer auto-sync as opt-in per workspace. Requires robust error handling and retry logic.
- [ ] **Calendar integration via MCP** -- Use a calendar MCP server to auto-populate meeting metadata. Avoids building OAuth flows directly.
- [ ] **Workspace templates** -- Pre-configured workspace blueprints ("Engineering Team" with standup/retro templates, Linear MCP, relevant system prompt).

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Filesystem isolation per workspace | HIGH | MEDIUM | P1 |
| Workspace CRUD + switcher | HIGH | LOW | P1 |
| Default workspace migration | HIGH | LOW | P1 |
| Per-workspace custom system prompt | HIGH | LOW | P1 |
| Workspace context fields | HIGH | LOW | P1 |
| Structured markdown notes | HIGH | MEDIUM | P1 |
| Action item extraction | HIGH | MEDIUM | P1 |
| Meeting metadata form | MEDIUM | LOW | P1 |
| Meeting note file saving | HIGH | LOW | P1 |
| Save to Obsidian vault path | HIGH | LOW | P1 |
| Meeting templates per workspace | MEDIUM | MEDIUM | P2 |
| MCP server config (JSON) | MEDIUM | LOW | P2 |
| MCP config UI editor | MEDIUM | MEDIUM | P2 |
| Generic MCP client framework | MEDIUM | HIGH | P2 |
| Manual sync trigger | MEDIUM | MEDIUM | P2 |
| Participant-to-assignee mapping | MEDIUM | MEDIUM | P2 |
| Searchable meeting history | MEDIUM | MEDIUM | P2 |
| Cross-meeting intelligence | LOW | HIGH | P3 |
| MCP Apps UI rendering | LOW | HIGH | P3 |
| Auto-sync (opt-in) | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for v0.3.0 launch. These define what "workspaces" means.
- P2: Should have in v0.3.x. These complete the MCP story and polish the experience.
- P3: Nice to have, v0.4+ consideration.

## Competitor Feature Analysis

| Feature | Granola | Krisp | Fireflies | Otter | Meetily v0.3.0 (Planned) |
|---------|---------|-------|-----------|-------|--------------------------|
| Bot-free recording | Yes (device audio) | Yes (virtual mic/speaker) | No (bot joins) | No (bot joins) | Yes (device audio via cpal) |
| Local/private processing | Partial (local capture, cloud AI) | Yes (on-device ASR) | No (cloud) | No (cloud) | Yes (local Whisper + optional local LLM) |
| Structured notes | Yes (AI-generated) | Yes (AI-generated) | Yes (AI-generated) | Yes (AI-generated) | Yes (LLM-generated markdown) |
| Action items | Yes (basic) | Yes (with assignee/deadline) | Yes (with CRM sync) | Yes (basic) | Yes (with assignee mapping from participants) |
| Templates / Recipes | Yes ("Recipes") | No | Limited | No | Yes (per-workspace templates) |
| Workspace isolation | Partial (shared folders in 2.0) | No | No | No | **Yes (full filesystem isolation)** |
| Custom system prompt per workspace | No | No | No | No | **Yes (unique to Meetily)** |
| Context injection (team, project) | No | No | No | No | **Yes (workspace context fields)** |
| MCP integration | No | No | No | No | **Yes (generic MCP client)** |
| Obsidian vault integration | No (via Zapier) | No | No | No | **Yes (direct file write)** |
| Linear integration | No (via Zapier) | No | No | No | **Yes (via MCP server)** |
| Cross-meeting search | Yes (2.0 Folders) | No | Yes (AskFred) | Yes (search) | Deferred to v0.4+ |
| Pricing model | $18/mo | $8/mo | $18/mo | $8.33/mo | Free (open source) |

**Key competitive insight:** Meetily's differentiation is not in transcription quality or AI summary quality (these are converging across all tools). It is in **workspace-level context isolation**, **per-workspace LLM personality via system prompts**, and **generic MCP integration** for pushing outputs to any connected system. No competitor offers this combination. The privacy-first, fully-local architecture is the foundation that makes workspace isolation meaningful -- data truly stays separated.

## Sources

### Competitor & Market Research
- [Reclaim.ai: Top 18 AI Meeting Assistants 2026](https://reclaim.ai/blog/ai-meeting-assistants) -- MEDIUM confidence (comparison guide)
- [Fellow.ai: 22 Best AI Meeting Assistants 2026](https://fellow.ai/blog/ai-meeting-assistants-ultimate-guide/) -- MEDIUM confidence (comparison guide)
- [Meetergo: 7 Best AI Note Taker Apps 2026](https://meetergo.com/en/magazine/best-ai-note-taker-apps) -- MEDIUM confidence (bot fatigue trend)
- [Krisp.ai: Best AI Meeting Assistants 2026](https://krisp.ai/blog/best-ai-meeting-assistant/) -- MEDIUM confidence (benchmark data)
- [Granola.ai Official Site](https://www.granola.ai/) -- HIGH confidence (official product info)
- [Granola 2.0 Workspace Features](https://quantumzeitgeist.com/granola-2-0-the-ai-powered-workspace-revolutionizing-team-collaboration/) -- MEDIUM confidence (third-party report)
- [Granola AI Review 2026](https://work-management.org/productivity-tools/granola-ai-review/) -- MEDIUM confidence
- [Fellow vs Granola Comparison](https://meetingnotes.com/blog/fellow-vs-granola-ai-notetakers) -- MEDIUM confidence
- [Krisp AI Meeting Assistant](https://krisp.ai/ai-meeting-assistant/) -- HIGH confidence (official product page)

### MCP Protocol & Ecosystem
- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25) -- HIGH confidence (official spec)
- [MCP TypeScript SDK (npm)](https://www.npmjs.com/package/@modelcontextprotocol/sdk) -- HIGH confidence (official package)
- [MCP Apps Extension (January 2026)](https://blog.modelcontextprotocol.io/posts/2026-01-26-mcp-apps/) -- HIGH confidence (official blog)
- [Linear MCP Server (Official)](https://linear.app/docs/mcp) -- HIGH confidence (official docs)
- [Obsidian MCP Server (mcp-obsidian)](https://github.com/MarkusPfundstein/mcp-obsidian) -- HIGH confidence (open source project)
- [Obsidian MCP Server (cyanheads)](https://github.com/cyanheads/obsidian-mcp-server) -- HIGH confidence (open source project)

### Workspace & UX Patterns
- [DEV Community: 5 Essential Features of a Productivity App 2026](https://dev.to/anas_kayssi/5-essential-features-of-a-productivity-app-in-2026-408g) -- LOW confidence (opinion piece)
- [Shift.com: 2026 Most Innovative Apps](https://shift.com/blog/2026-most-innovative-apps/) -- MEDIUM confidence (Spaces pattern)
- [UX Planet: Best Practices for Designing a Sidebar](https://uxplanet.org/best-ux-practices-for-designing-a-sidebar-9174ee0ecaa2) -- MEDIUM confidence (UX guidance)
- [Obsidian Forum: Workspace Switcher Dropdown Mockup](https://forum.obsidian.md/t/workspace-switcher-drop-down-menu-mockup/23785) -- MEDIUM confidence (community pattern)

### Meeting Notes & Templates
- [AWS: Meeting Summarization and Action Item Extraction](https://aws.amazon.com/blogs/machine-learning/meeting-summarization-and-action-item-extraction-with-amazon-nova/) -- MEDIUM confidence (technical approach)
- [AFFiNE: How AI Gets Action Items from Meeting Notes](https://affine.pro/blog/how-to-get-action-items-from-meeting-notes-ai-tips) -- MEDIUM confidence
- [dannb.org: Obsidian Meeting Note Template](https://dannb.org/blog/2023/obsidian-meeting-note-template/) -- HIGH confidence (practical Obsidian workflow)
- [Obsidian Help: Properties/Frontmatter](https://help.obsidian.md/Editing+and+formatting/Properties) -- HIGH confidence (official docs)
- [GitHub Gist: Daily Scrum Markdown Template](https://gist.github.com/Potherca/63c33e405947c45403766aa285a37ad1) -- MEDIUM confidence

### MCP Configuration Patterns
- [Claude Code Docs: MCP Configuration](https://code.claude.com/docs/en/mcp) -- HIGH confidence (official docs)
- [Claude Help: Getting Started with Local MCP Servers](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop) -- HIGH confidence (official docs)
- [FastMCP: MCP JSON Configuration](https://gofastmcp.com/integrations/mcp-json-configuration) -- MEDIUM confidence

---
*Feature research for: Meetily v0.3.0 Workspaces + MCP*
*Researched: 2026-02-01*
