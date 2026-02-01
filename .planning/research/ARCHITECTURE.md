# Architecture Research: Workspace Isolation + MCP Client Integration

**Domain:** Desktop meeting assistant with workspace isolation and MCP tool integration
**Researched:** 2026-02-01
**Confidence:** HIGH (existing codebase well-understood; MCP SDK patterns verified via official sources)

## System Overview

### Current Architecture (v0.2.0)

```
+---------------------------------------------------------------+
|                    Frontend (Tauri Desktop App)                |
|  +------------------+  +-----------------+  +----------------+|
|  |   Next.js UI     |  |  Rust Backend   |  | Whisper Engine ||
|  |  (React/TS)      |<>|  (Audio + IPC)  |<>|  (Local STT)   ||
|  +------------------+  +-----------------+  +----------------+|
|         ^ Tauri Commands/Events    ^ Audio Pipeline           |
+---------|--------------------------|--------------------------+
          | HTTP                     |
          v                          |
+---------|--------------------------|-------------------------+
|              Backend (FastAPI)     |                         |
|  +-----------+  +-----------------+--+  +----------------+  |
|  |  SQLite   |<>|  Meeting Manager   |<>|  LLM Provider  |  |
|  | (single)  |  |  (CRUD + Summary)  |  | (Ollama/etc.)  |  |
|  +-----------+  +--------------------+  +----------------+  |
+-------------------------------------------------------------+
```

**Key constraint:** Currently a single SQLite database (`meeting_minutes.sqlite`) in `app_data_dir`, a single `AppState` holding one `DatabaseManager`, and all meetings in a flat list.

### Proposed Architecture (v0.3.0)

```
+------------------------------------------------------------------------+
|                     Frontend (Tauri Desktop App)                       |
|  +------------------+  +-------------------+  +----------------------+ |
|  |   Next.js UI     |  |   Rust Core       |  |  Audio Pipeline      | |
|  |  +------------+  |  |  +-------------+  |  |  (unchanged)         | |
|  |  | Workspace  |  |  |  | Workspace   |  |  +----------------------+ |
|  |  | Context    |<--->  | Manager     |  |                           |
|  |  +------------+  |  |  +------+------+  |  +----------------------+ |
|  |  | MCP Config |  |  |         |         |  |  Whisper Engine      | |
|  |  | Panel      |<--->  +------v------+  |  |  (unchanged)         | |
|  |  +------------+  |  |  | MCP Client  |  |  +----------------------+ |
|  |  | Note Gen   |  |  |  | Manager     |  |                           |
|  |  | Panel      |  |  |  +------+------+  |                           |
|  |  +------------+  |  |         |         |                           |
|  +------------------+  +---------|---------+                           |
|         ^                        |                                     |
+---------|---------Tauri IPC------|---------stdio-child-processes-------+
          |                        |                |
          v                        v                v
  +---------------+    +------------------+   +------------+
  | Workspace     |    |  Per-Workspace   |   | MCP Server |
  | Filesystem    |    |  SQLite DB       |   | Processes  |
  | (folders)     |    |  (via sqlx)      |   | (stdio)    |
  +---------------+    +------------------+   +------------+
```

## Component Responsibilities

| Component | Responsibility | Location | Communicates With |
|-----------|----------------|----------|-------------------|
| **WorkspaceManager** | CRUD workspaces, switch active workspace, manage DB pools | Rust (`src-tauri/src/workspace/`) | DatabaseManager, MCP Client Manager, Frontend via Tauri commands |
| **WorkspaceDbPool** | Maintain `HashMap<WorkspaceId, SqlitePool>`, lazy-init per workspace | Rust (`src-tauri/src/workspace/db.rs`) | WorkspaceManager, all Repositories |
| **McpClientManager** | Spawn/stop MCP server child processes, invoke tools, manage lifecycle | Rust (`src-tauri/src/mcp/`) | WorkspaceManager (for config), LLM pipeline (for tool results) |
| **NoteGenerator** | Orchestrate LLM summary + MCP tool calls into structured markdown | Rust (`src-tauri/src/notes/`) | SummaryService (existing), McpClientManager, WorkspaceManager (for config) |
| **WorkspaceContext** | Frontend state for active workspace, workspace list, config | React (`src/contexts/WorkspaceContext.tsx`) | WorkspaceManager via Tauri commands |
| **WorkspaceConfigPanel** | UI for system prompt, MCP servers, context fields | React (`src/components/WorkspaceSettings/`) | WorkspaceContext |
| **McpConfigEditor** | UI for adding/editing/removing MCP server entries | React (`src/components/McpConfig/`) | WorkspaceContext, McpClientManager via Tauri commands |
| **AudioPipeline** | Unchanged -- audio capture, mixing, VAD, transcription | Rust (`src-tauri/src/audio/`) | RecordingManager (existing) |
| **SummaryService** | Existing LLM summarization -- now workspace-aware | Rust (`src-tauri/src/summary/`) | WorkspaceManager (for system prompt + template) |

## Recommended Project Structure

### Rust Side (New Modules)

```
frontend/src-tauri/src/
+-- workspace/                 # NEW: Workspace management
|   +-- mod.rs                # Module exports
|   +-- manager.rs            # WorkspaceManager: CRUD, active workspace state
|   +-- db.rs                 # WorkspaceDbPool: HashMap<WorkspaceId, SqlitePool>
|   +-- models.rs             # Workspace, WorkspaceConfig structs
|   +-- commands.rs           # Tauri commands: create/switch/delete workspace
|   +-- migration.rs          # Per-workspace DB migration runner
|   +-- filesystem.rs         # Create/delete workspace folders
+-- mcp/                       # NEW: MCP client integration
|   +-- mod.rs                # Module exports
|   +-- client.rs             # MCP client lifecycle (connect, disconnect, list_tools, call_tool)
|   +-- manager.rs            # McpClientManager: active MCP connections per workspace
|   +-- config.rs             # McpServerConfig parsing, validation
|   +-- commands.rs           # Tauri commands: connect/disconnect/invoke MCP servers
|   +-- transport.rs          # TokioChildProcess wrapper for stdio servers
+-- notes/                     # NEW: Meeting note generation
|   +-- mod.rs                # Module exports
|   +-- generator.rs          # Orchestrate LLM + MCP into structured markdown
|   +-- file_writer.rs        # Write .md files to workspace folder + Obsidian vault
|   +-- commands.rs           # Tauri commands: generate notes, save notes
```

### Frontend Side (New Components/Contexts)

```
frontend/src/
+-- contexts/
|   +-- WorkspaceContext.tsx    # NEW: Active workspace state, workspace list
+-- components/
|   +-- WorkspaceSettings/     # NEW: Workspace config UI
|   |   +-- WorkspaceSettings.tsx
|   |   +-- SystemPromptEditor.tsx
|   |   +-- ContextFieldsEditor.tsx
|   +-- McpConfig/             # NEW: MCP server configuration UI
|   |   +-- McpServerList.tsx
|   |   +-- McpServerEditor.tsx
|   +-- Sidebar/
|   |   +-- WorkspaceSwitcher.tsx  # NEW: Dropdown in sidebar
|   +-- MeetingNotes/          # NEW: Note generation UI
|       +-- NoteGenerator.tsx
|       +-- MetadataForm.tsx
```

### Structure Rationale

- **`workspace/` as top-level module:** Workspace management is a cross-cutting concern that touches database, filesystem, and config. It deserves its own module rather than being bolted onto `database/`.
- **`mcp/` separate from `summary/`:** MCP is a general-purpose tool integration, not summary-specific. A Linear integration via MCP has nothing to do with LLM summarization. Keeping it separate allows future non-summary MCP uses.
- **`notes/` separate from `summary/`:** The existing `summary/` module handles raw LLM summarization. `notes/` orchestrates the higher-level workflow: apply workspace system prompt, inject context fields, call MCP tools, write files. It _uses_ `summary/` but is not the same thing.
- **WorkspaceContext wraps SidebarProvider:** The existing `SidebarProvider` manages meetings for the current context. `WorkspaceContext` provides the active workspace ID, and `SidebarProvider` filters meetings by it.

## Architectural Patterns

### Pattern 1: Workspace-Scoped Database Pool

**What:** Maintain a `HashMap<WorkspaceId, SqlitePool>` wrapped in `Arc<RwLock<...>>`. Each workspace gets its own SQLite file with independent WAL, migrations, and connection pool. The active workspace's pool is the one used by all repository queries.

**When to use:** Every database operation (meetings, transcripts, summaries, settings).

**Trade-offs:**
- PRO: Complete isolation -- deleting a workspace means deleting a folder
- PRO: No migration headaches with multi-tenant schemas
- PRO: WAL contention only within a single workspace
- CON: Pool memory overhead (minor -- SQLite pools are lightweight)
- CON: Must ensure consistent schema across all workspace DBs via shared migrations

**Example:**

```rust
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WorkspaceDbPool {
    pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
    active_workspace_id: Arc<RwLock<String>>,
}

impl WorkspaceDbPool {
    /// Get the pool for the active workspace
    pub async fn active_pool(&self) -> Result<SqlitePool, String> {
        let active_id = self.active_workspace_id.read().await.clone();
        let pools = self.pools.read().await;
        pools.get(&active_id)
            .cloned()
            .ok_or_else(|| format!("No pool for workspace: {}", active_id))
    }

    /// Lazily create a pool for a workspace
    pub async fn get_or_create(&self, workspace_id: &str, db_path: &str) -> Result<SqlitePool, sqlx::Error> {
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(workspace_id) {
                return Ok(pool.clone());
            }
        }
        let pool = SqlitePool::connect(db_path).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        self.pools.write().await.insert(workspace_id.to_string(), pool.clone());
        Ok(pool)
    }
}
```

**Confidence:** HIGH -- sqlx `Pool` is `Clone + Send + Sync`, multiple pools are a documented pattern ([sqlx docs](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html)).

### Pattern 2: MCP Client via rmcp stdio Transport

**What:** Use the official `rmcp` Rust SDK to spawn MCP servers as child processes using `TokioChildProcess`. Each workspace's configured MCP servers are spawned on-demand (lazy) or on workspace activation.

**When to use:** MCP server connection, tool listing, tool invocation.

**Trade-offs:**
- PRO: Official SDK, well-maintained, async-first
- PRO: stdio transport is the standard for local MCP servers
- PRO: Runs in the same Tauri process -- no extra network hops
- CON: Child process lifecycle management adds complexity
- CON: MCP server crashes must be detected and handled gracefully

**Example:**

```rust
use rmcp::transport::TokioChildProcess;
use rmcp::ServiceExt;
use tokio::process::Command;

pub struct McpConnection {
    pub server_name: String,
    pub service: rmcp::Service,  // The connected client service
}

impl McpConnection {
    pub async fn connect(name: &str, command: &str, args: &[String], env: &HashMap<String, String>) -> Result<Self> {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }

        let transport = TokioChildProcess::new(cmd)?;
        let service = ().serve(transport).await?;

        Ok(McpConnection {
            server_name: name.to_string(),
            service,
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let tools = self.service.list_tools(Default::default()).await?;
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<CallToolResult> {
        let result = self.service.call_tool(CallToolRequestParams {
            name: name.into(),
            arguments: arguments.as_object().cloned(),
            ..Default::default()
        }).await?;
        Ok(result)
    }
}
```

**Confidence:** HIGH -- `rmcp` is the official MCP Rust SDK ([GitHub](https://github.com/modelcontextprotocol/rust-sdk)), `TokioChildProcess` is the documented stdio transport.

### Pattern 3: Workspace Filesystem Isolation

**What:** Each workspace gets a dedicated folder under `app_data_dir/workspaces/{workspace_id}/` containing its SQLite database, config file, and meeting audio/notes.

**When to use:** All workspace-scoped file operations.

**Layout:**
```
~/Library/Application Support/Meetily/
+-- global_settings.json       # App-wide settings (theme, language, etc.)
+-- models/                    # Whisper/Parakeet models (shared)
+-- workspaces/
    +-- default/               # Default workspace (backward compat)
    |   +-- workspace.json     # Workspace config (system prompt, MCP servers, context fields)
    |   +-- meeting_minutes.sqlite
    |   +-- meetings/
    |   |   +-- {meeting-id}/
    |   |       +-- audio.mp4
    |   |       +-- transcript.json
    |   |       +-- notes.md
    |   +-- notes/             # Generated meeting note files
    +-- alpha-team/
    |   +-- workspace.json
    |   +-- meeting_minutes.sqlite
    |   +-- meetings/
    |   +-- notes/
    +-- ...
```

**Trade-offs:**
- PRO: Clean isolation -- backup = copy folder, delete = delete folder
- PRO: Existing meeting folder structure preserved within workspace
- CON: Must update all path resolution to be workspace-aware
- CON: Migrating existing meetings into "default" workspace requires careful handling

**Confidence:** HIGH -- filesystem layout is a design decision, not a technology question.

### Pattern 4: MCP Server Configuration (claude_desktop_config.json Pattern)

**What:** Store MCP server configuration per workspace in a JSON format following the established convention from Claude Desktop's `claude_desktop_config.json`.

**When to use:** Workspace settings for MCP server connections.

**Config format:**
```json
{
  "name": "Alpha Team",
  "system_prompt": "Summarize meetings for the Alpha Team...",
  "context_fields": {
    "team": "Alpha Team",
    "obsidian_vault": "~/Vaults/work/alpha/"
  },
  "mcp_servers": {
    "linear": {
      "command": "npx",
      "args": ["-y", "@anthropic/linear-mcp-server"],
      "env": {
        "LINEAR_API_KEY": "lin_api_xxx"
      }
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "~/Vaults/work/alpha/"]
    }
  },
  "templates": {
    "default": "daily_standup"
  }
}
```

**Trade-offs:**
- PRO: Users familiar with Claude Desktop config can reuse knowledge
- PRO: Power users can hand-edit JSON, casual users use UI
- PRO: Extensible -- add fields without schema migration
- CON: Must validate before spawning (bad config = crash)

**Confidence:** HIGH -- follows established MCP convention ([MCP docs](https://modelcontextprotocol.io/docs/develop/connect-local-servers)).

## Data Flow

### Flow 1: Workspace Switching

```
User clicks workspace in sidebar
    |
    v
[WorkspaceSwitcher.tsx]  -- invoke('switch_workspace', { id: 'alpha-team' })
    |
    v
[workspace::commands::switch_workspace]
    |
    +-- 1. Check if recording is active --> BLOCK if yes (cannot switch during recording)
    |
    +-- 2. WorkspaceDbPool::get_or_create('alpha-team', path)
    |       --> Lazy-init SQLite pool if not already open
    |       --> Run migrations if needed
    |
    +-- 3. McpClientManager::deactivate_workspace('previous-id')
    |       --> Gracefully disconnect MCP servers for previous workspace
    |
    +-- 4. McpClientManager::activate_workspace('alpha-team')
    |       --> Load workspace.json MCP config
    |       --> Spawn configured MCP servers (lazy -- on first tool call)
    |
    +-- 5. Update active_workspace_id in state
    |
    +-- 6. Emit 'workspace-switched' Tauri event
    |
    v
[WorkspaceContext.tsx] listens for 'workspace-switched'
    |
    +-- Update active workspace in React state
    +-- SidebarProvider.refetchMeetings() --> now queries active workspace DB
    +-- Reset currentMeeting to null
```

### Flow 2: Meeting Note Generation + File Save

```
User clicks "Generate Notes" after recording ends
    |
    v
[NoteGenerator.tsx]  -- invoke('generate_meeting_notes', { meeting_id, metadata })
    |
    v
[notes::commands::generate_meeting_notes]
    |
    +-- 1. Get active workspace config (system_prompt, context_fields, template)
    |
    +-- 2. Build enhanced prompt:
    |       system_prompt + context_fields + meeting_metadata + template instructions
    |
    +-- 3. Call SummaryService::process_transcript_background(...)
    |       --> Uses workspace-specific pool for DB operations
    |       --> Uses enhanced prompt (not raw custom_prompt)
    |
    +-- 4. On completion, get markdown result
    |
    +-- 5. Generate filename: {team}-{meeting-date}.md
    |
    +-- 6. Write file to workspace/notes/ folder
    |
    +-- 7. If obsidian_vault configured in context_fields:
    |       --> Copy/write .md to obsidian_vault path
    |
    +-- 8. (Optional) If MCP sync requested:
    |       --> McpClientManager::call_tool('linear', 'create_issue', action_items)
    |
    +-- 9. Emit 'notes-generated' event with file paths
    |
    v
[Frontend] shows success notification with links to generated files
```

### Flow 3: MCP Tool Invocation (Manual Sync)

```
User clicks "Sync to Linear" button on meeting notes
    |
    v
[MeetingNotes.tsx]  -- invoke('mcp_invoke_tool', { server: 'linear', tool: 'create_issue', args })
    |
    v
[mcp::commands::invoke_tool]
    |
    +-- 1. McpClientManager::get_connection('linear')
    |       --> If not connected, spawn from workspace config (lazy connect)
    |       --> Connect via TokioChildProcess stdio
    |       --> Initialize handshake (MCP initialize/initialized)
    |
    +-- 2. connection.call_tool('create_issue', arguments)
    |       --> JSON-RPC 2.0 over stdio
    |       --> Wait for response
    |
    +-- 3. Return result to frontend
    |
    v
[Frontend] shows sync result (success/error)
```

### State Management Flow

```
[Tauri Managed State]
    |
    +-- AppState { db_manager }        --> EXISTING: Replace with WorkspaceDbPool
    |
    +-- WorkspaceState {               --> NEW
    |       active_workspace_id: Arc<RwLock<String>>,
    |       workspace_db_pool: WorkspaceDbPool,
    |       workspace_configs: Arc<RwLock<HashMap<String, WorkspaceConfig>>>,
    |   }
    |
    +-- McpState {                     --> NEW
    |       connections: Arc<RwLock<HashMap<String, HashMap<String, McpConnection>>>>,
    |       // workspace_id -> server_name -> connection
    |   }
    |
    +-- WhisperState { ... }           --> EXISTING: Unchanged (shared across workspaces)
    +-- NotificationState { ... }      --> EXISTING: Unchanged (global)
    +-- RecordingState { ... }         --> EXISTING: Unchanged (single active recording)


[React Context Tree]
    |
    +-- WorkspaceProvider              --> NEW: Wraps everything
    |   |
    |   +-- ConfigProvider             --> EXISTING: Now workspace-scoped
    |   |   |
    |   |   +-- SidebarProvider        --> EXISTING: Filters by active workspace
    |   |   |   |
    |   |   |   +-- RecordingStateProvider  --> EXISTING: Unchanged
    |   |   |   |   |
    |   |   |   |   +-- App content
```

## Critical Integration Points

### Integration 1: WorkspaceManager replaces AppState.db_manager

**Current:** `AppState { db_manager: DatabaseManager }` holds a single `SqlitePool`.
**Proposed:** `WorkspaceState { workspace_db_pool: WorkspaceDbPool }` holds multiple pools.
**Migration path:**
1. All existing repository calls use `pool` directly: `MeetingsRepository::get_all(&pool)`.
2. Repositories do NOT change -- they accept `&SqlitePool` as before.
3. The change is at the call site: instead of `app.state::<AppState>().db_manager.pool()`, use `app.state::<WorkspaceState>().workspace_db_pool.active_pool().await`.
4. This means every Tauri command that accesses the DB needs updating, but repositories stay clean.

**Confidence:** HIGH -- this is a mechanical refactor, not an architectural change.

### Integration 2: Recording Always Targets Active Workspace

**Current:** `RecordingManager` saves to a meeting folder resolved from `recording_preferences`.
**Proposed:** Meeting folder is resolved as `workspaces/{active_workspace_id}/meetings/{meeting_id}/`.
**Constraint:** Workspace switching is BLOCKED while recording. This is critical for audio pipeline integrity -- the recording state, audio streams, and VAD pipeline must not be disrupted mid-recording.

**Confidence:** HIGH -- blocking workspace switch during recording is the safe approach.

### Integration 3: MCP Client Lifecycle Tied to Workspace Activation

**Current:** N/A (no MCP support).
**Proposed:** When switching workspaces:
1. Gracefully disconnect all MCP servers for the previous workspace
2. Load MCP config for the new workspace
3. Do NOT auto-connect servers -- lazy connect on first tool invocation
4. Clean up child processes on app exit

**Rationale for lazy connect:** MCP servers are child processes consuming system resources. A workspace with 3 configured servers should not spawn 3 processes just because the user switched to it. Spawn only when a tool is actually invoked.

**Confidence:** HIGH -- lazy initialization is standard for resource management.

### Integration 4: SummaryService Becomes Workspace-Aware

**Current:** `SummaryService::process_transcript_background(pool, ...)` takes a `SqlitePool` and `custom_prompt`.
**Proposed:** The caller (notes generator) provides:
- The active workspace's pool (from `WorkspaceDbPool`)
- An enhanced prompt built from workspace config (system_prompt + context_fields)
- The workspace's default template

**SummaryService itself does NOT change.** The workspace awareness lives in the `notes/` orchestration layer, which constructs the right inputs and passes them to the existing `SummaryService`.

**Confidence:** HIGH -- minimal changes to existing code.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Global MCP Client Singleton

**What people do:** Create a single MCP client manager that all workspaces share, with server names as the only differentiator.
**Why it's wrong:** Two workspaces might configure the same MCP server (e.g., "filesystem") with different arguments (different vault paths). A global singleton would overwrite one workspace's config with another's.
**Do this instead:** Scope MCP connections by workspace ID: `HashMap<WorkspaceId, HashMap<ServerName, McpConnection>>`. Each workspace has its own set of connections.

### Anti-Pattern 2: Shared Database with Workspace Column

**What people do:** Add a `workspace_id` column to every table instead of separate databases.
**Why it's wrong:** Violates the isolation requirement. A bug in query filtering exposes data cross-workspace. Migration complexity increases. Cannot backup/delete a single workspace cleanly.
**Do this instead:** One SQLite file per workspace. Same schema, different files. Repository code unchanged.

### Anti-Pattern 3: Auto-Connecting MCP Servers on Workspace Switch

**What people do:** Spawn all configured MCP servers immediately when a workspace becomes active.
**Why it's wrong:** MCP servers are child processes consuming memory and CPU. If a user has 5 workspaces with 2 servers each, auto-connecting means 10 child processes running. Most will never be used in a given session.
**Do this instead:** Lazy connect -- spawn the MCP server child process only when the user invokes a tool from that server.

### Anti-Pattern 4: Workspace State in React Only

**What people do:** Manage workspace switching entirely in React state, making API calls to different endpoints per workspace.
**Why it's wrong:** The Rust backend needs to know the active workspace to resolve database paths, MCP configs, and file locations. Frontend-only state creates a split-brain where the backend might use the wrong workspace.
**Do this instead:** Active workspace is Rust-managed state (`Arc<RwLock<String>>`). Frontend reflects it but does not own it. Workspace switching is a Tauri command, not a React state update.

### Anti-Pattern 5: MCP Client in the FastAPI Backend

**What people do:** Run the MCP client in the Python backend since it already handles LLM interactions.
**Why it's wrong:** MCP stdio transport spawns child processes. Running them from the Python backend means: (a) the backend must always be running for MCP to work, (b) cross-process communication doubles (Tauri->Python->MCP vs Tauri->MCP), (c) the Python backend is being phased toward Rust-side summary engine.
**Do this instead:** MCP client lives in the Tauri Rust process. Direct `TokioChildProcess` spawning with no intermediary.

## Scaling Considerations

| Concern | 1-3 Workspaces | 10-20 Workspaces | 50+ Workspaces |
|---------|----------------|-------------------|----------------|
| DB Pool Memory | Negligible (~3 pools) | Moderate -- lazy-init helps, close inactive pools after timeout | Implement LRU pool eviction (close least-recently-used pools) |
| MCP Processes | 0-3 child processes | Lazy-connect prevents bloat | Hard cap on concurrent MCP connections per workspace |
| Filesystem | 3 folders | 20 folders -- no issue | May need workspace archival/export feature |
| UI Performance | Fast list render | Still fast (sidebar is virtualized) | May need search/filter in workspace switcher |
| Startup Time | Instant (only default workspace pool) | Instant (lazy-init) | Instant (lazy-init) -- first DB access per workspace adds ~50ms |

### First Bottleneck: DB Pool Memory at Scale

At 20+ workspaces, keeping all pools open wastes memory. Solution: implement an LRU cache with a max of 5 concurrent pools. Close pools unused for >5 minutes. Re-open on next access (~50ms penalty).

### Second Bottleneck: MCP Server Process Count

If multiple workspaces have servers connected simultaneously, child process count can grow. Solution: enforce per-workspace connection limits and disconnect idle servers after a timeout (e.g., 10 minutes of no tool calls).

## Build Order (Dependency Chain)

The components have strict build-order dependencies:

```
Phase 1: Workspace Foundation
   [workspace/models.rs]         -- Define Workspace, WorkspaceConfig structs
   [workspace/filesystem.rs]     -- Create/delete workspace folders
   [workspace/db.rs]             -- WorkspaceDbPool with HashMap<WorkspaceId, SqlitePool>
   [workspace/migration.rs]      -- Per-workspace migration runner
   [workspace/manager.rs]        -- CRUD operations, active workspace state
   [workspace/commands.rs]       -- Tauri commands

Phase 2: Frontend Workspace UI  (can start after Phase 1 commands exist)
   [WorkspaceContext.tsx]         -- React context for workspace state
   [WorkspaceSwitcher.tsx]        -- Sidebar dropdown
   Refactor SidebarProvider       -- Filter meetings by active workspace

Phase 3: Database Migration      (requires Phase 1)
   Migrate existing meetings to "default" workspace
   Update all Tauri commands to use WorkspaceDbPool.active_pool()
   Update RecordingManager to use workspace-scoped paths

Phase 4: MCP Client Framework    (independent of Phase 1-3, can parallel)
   [mcp/config.rs]               -- Parse MCP server config
   [mcp/transport.rs]            -- TokioChildProcess wrapper
   [mcp/client.rs]               -- Connect, list_tools, call_tool
   [mcp/manager.rs]              -- Per-workspace connection management
   [mcp/commands.rs]             -- Tauri commands

Phase 5: Workspace Config UI     (requires Phase 1 + Phase 4)
   [WorkspaceSettings.tsx]        -- System prompt, context fields
   [McpServerList.tsx]            -- List/add/remove MCP servers
   [McpServerEditor.tsx]          -- Edit server config

Phase 6: Meeting Note Generator   (requires Phase 1 + Phase 4 + existing summary/)
   [notes/generator.rs]          -- Orchestrate LLM + workspace context
   [notes/file_writer.rs]        -- Write .md to workspace + Obsidian vault
   [notes/commands.rs]            -- Tauri commands
   [NoteGenerator.tsx]            -- Frontend UI
   [MetadataForm.tsx]             -- Meeting metadata input
```

**Critical path:** Phase 1 -> Phase 3 -> Phase 6. The MCP client (Phase 4) can be developed in parallel.

## External Integration Points

### MCP Servers (External)

| Server | Integration Pattern | Notes |
|--------|---------------------|-------|
| Any stdio MCP server | `TokioChildProcess` via `rmcp` | User-configured, spawned as child process |
| Any HTTP MCP server | `SseClientTransport` via `rmcp` (future) | v0.3.0 focuses on stdio; HTTP can be added later |

### Obsidian Vault (External)

| Integration | Pattern | Notes |
|-------------|---------|-------|
| Save meeting notes | Direct file write to vault path | No Obsidian API needed -- just write `.md` to the configured folder path |
| Sync via MCP | Optional -- `@modelcontextprotocol/server-filesystem` | Alternative to direct write if user prefers MCP-based file access |

### LLM Providers (Existing)

| Provider | Change Needed | Notes |
|----------|--------------|-------|
| Ollama, Claude, Groq, OpenRouter, CustomOpenAI | None to provider code | Workspace system prompt injected at prompt construction time, not provider level |

## Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `workspace/` <-> `database/repositories/` | Pass `&SqlitePool` | Repositories unchanged -- pool source changes |
| `workspace/` <-> `mcp/` | WorkspaceConfig provides MCP server list | MCP manager reads config, workspace manager writes it |
| `mcp/` <-> `notes/` | NoteGenerator calls McpClientManager::call_tool() | For manual sync (e.g., "push to Linear") |
| `notes/` <-> `summary/` | NoteGenerator calls SummaryService | Uses existing summary pipeline with workspace-enhanced prompt |
| Frontend WorkspaceContext <-> SidebarProvider | WorkspaceContext provides active workspace ID | SidebarProvider uses it to filter meetings |
| Frontend <-> Rust | Tauri commands + events | All workspace/MCP operations go through Tauri IPC |

## Key Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| MCP SDK | `rmcp` (official Rust SDK) | Official, well-maintained, async-first, supports stdio transport. [GitHub](https://github.com/modelcontextprotocol/rust-sdk) |
| MCP Transport | stdio via `TokioChildProcess` | Standard for local MCP servers, runs in-process, no network overhead |
| Workspace DB | Per-workspace SQLite file via `sqlx::SqlitePool` | Complete isolation, easy backup, same migration tooling |
| Workspace Config | JSON file per workspace (`workspace.json`) | Flexible, user-editable, extensible without DB migration |
| Note File Format | Markdown (.md) | Universal, Obsidian-compatible, version-control friendly |
| MCP Client Location | Rust side (Tauri process) | Direct child process management, no Python intermediary |

## Sources

- [MCP Specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25) -- Protocol specification
- [rmcp Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) -- MCP client implementation
- [rmcp docs.rs](https://docs.rs/rmcp) -- API documentation
- [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk) -- Reference for protocol patterns
- [MCP Client Development Guide](https://github.com/cyanheads/model-context-protocol-resources/blob/main/guides/mcp-client-development-guide.md)
- [Claude Desktop MCP Config](https://modelcontextprotocol.io/docs/develop/connect-local-servers) -- Configuration format reference
- [SQLite Isolation](https://sqlite.org/isolation.html) -- SQLite concurrency model
- [sqlx Pool docs](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html) -- Connection pool API
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) -- Tauri managed state patterns
- [Tauri Plugin Store](https://v2.tauri.app/plugin/store/) -- Key-value persistence

---
*Architecture research for: Meetily v0.3.0 Workspace Isolation + MCP Client Integration*
*Researched: 2026-02-01*
