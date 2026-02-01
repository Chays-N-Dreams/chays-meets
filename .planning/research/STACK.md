# Stack Research: Meetily v0.3.0 -- Workspaces + MCP

**Domain:** Desktop meeting assistant -- workspace isolation and MCP client integration
**Researched:** 2026-02-01
**Confidence:** MEDIUM-HIGH (verified core libraries against official docs; some version details from WebSearch only)

---

## Recommended Stack

This document covers ONLY the new libraries and patterns needed for v0.3.0 (Workspaces + MCP). The existing Tauri 2.x / Next.js / FastAPI / SQLite / Whisper stack is retained as-is.

### MCP Client (Rust-side)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| `rmcp` | `0.14.0` | Official Rust MCP SDK -- client mode for connecting to MCP servers | **Official SDK** under `modelcontextprotocol` GitHub org. Supports client + server modes, stdio + Streamable HTTP transports. Built on tokio (already in the project). Has `#[tool]` macros, typed JSON-RPC, capability negotiation. This is the canonical choice -- no reason to use community alternatives. | HIGH |
| `rmcp-macros` | `0.14.0` | Proc macros for MCP tool/handler boilerplate | Companion crate to `rmcp`. Reduces boilerplate for tool definitions if we later expose Meetily as an MCP server too. | HIGH |

**Required `rmcp` features for client use:**
```toml
rmcp = { version = "0.14", features = ["client", "transport-child-process", "transport-io"] }
```

Add `transport-streamable-http-client` only if remote MCP servers are needed (defer until post-v0.3.0).

**Key API pattern (verified from docs.rs/rmcp):**
```rust
use rmcp::{ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

// Spawn an MCP server as a child process (stdio transport)
let service = ()
    .serve(TokioChildProcess::new(
        Command::new("npx")
            .arg("-y")
            .arg("@modelcontextprotocol/server-filesystem")
            .arg("/path/to/workspace")
    )?)
    .await?;

let tools = service.list_tools(Default::default()).await?;
let result = service.call_tool(params).await?;
service.cancel().await?;
```

### MCP Client (Frontend/TypeScript-side)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| Custom Tauri commands wrapping `rmcp` | N/A | Expose MCP operations to the React UI via Tauri IPC | **Build custom commands, not a plugin.** The existing `tauri-plugin-mcp-client` by Sublayer is not published on crates.io/npm (GitHub-only, unclear maintenance). The `tauri-plugin-mcp` by airi is at v0.7.1 with 0% doc coverage. Both are too immature to depend on. Instead: wrap `rmcp` in Tauri commands (the project already does this pattern for audio). The IPC layer is thin -- `connect_mcp_server`, `list_mcp_tools`, `call_mcp_tool`, `disconnect_mcp_server`. | MEDIUM-HIGH |

**Rationale for "build vs. buy" on MCP Tauri plugin:**
- `sublayerapp/tauri-plugin-mcp-client`: Not published to crates.io or npm. GitHub-only distribution. Unknown Tauri 2.x version compatibility.
- `tauri-plugin-mcp` (airi): Published (v0.7.1) but 0% documentation coverage. 11 versions in 9 months suggests unstable API surface.
- Custom Tauri commands wrapping `rmcp`: Follows the existing project pattern (audio commands wrap Rust audio libraries). Full control over the API surface. `rmcp` is the official SDK with known-good API.

### Workspace Isolation (Database)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| `sqlx` (already in project) | `0.8` | Per-workspace SQLite database files | Already a dependency. Supports runtime pool creation via `SqlitePool::connect_with(SqliteConnectOptions::new().filename(path).create_if_missing(true))`. No new crate needed. | HIGH |

**Multi-database pattern:**
```rust
use std::collections::HashMap;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Manages one SQLite pool per workspace
pub struct WorkspaceDbManager {
    pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
}

impl WorkspaceDbManager {
    pub async fn get_or_create_pool(&self, workspace_id: &str, db_path: &Path) -> Result<SqlitePool> {
        // Check cache first
        if let Some(pool) = self.pools.read().await.get(workspace_id) {
            return Ok(pool.clone()); // Pool is Clone + Send + Sync
        }
        // Create new pool for this workspace
        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        self.pools.write().await.insert(workspace_id.to_string(), pool.clone());
        Ok(pool)
    }
}
```

**Key design decisions:**
- WAL journal mode: Enables concurrent reads during writes (important when transcription is writing while UI reads).
- `create_if_missing(true)`: New workspaces auto-create their database.
- Pool caching: Avoids reconnecting on every operation. Pools are `Clone + Send + Sync`.
- Migration per workspace: Each database gets the same schema via `sqlx::migrate!`.

### Workspace Isolation (Filesystem)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| `tauri-plugin-fs` (already in project) | `2.4.0` | Read/write workspace files (meeting notes, audio, config) | Already a dependency. Provides scoped filesystem access from the frontend. | HIGH |
| `tauri-plugin-store` (already in project) | `2.4.0` | Per-workspace settings JSON | Already a dependency. Supports multiple store files by path -- use `workspace-{id}/settings.json` pattern. | HIGH |
| `dirs` (already in project) | `5.0.1` | Resolve platform-appropriate app data directories | Already a dependency. Used to locate workspace root folder. | HIGH |

**Workspace folder structure:**
```
~/Library/Application Support/Meetily/        # macOS (via dirs crate)
  workspaces/
    {workspace-id}/
      workspace.db                            # SQLite database
      settings.json                           # tauri-plugin-store file
      mcp-servers.json                        # MCP server configuration
      audio/                                  # Meeting recordings
      notes/                                  # Generated markdown meeting notes
```

### Workspace Configuration (MCP Servers)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| `serde` + `serde_json` (already in project) | `1.0` | Parse/write MCP server configuration JSON | Already dependencies. The MCP server config format (Claude Desktop's `mcpServers` JSON) is simple serde-compatible JSON. | HIGH |

**MCP server configuration format (per workspace):**
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/vault"],
      "env": {}
    },
    "obsidian-notes": {
      "command": "node",
      "args": ["/path/to/custom-server.js"],
      "env": {
        "VAULT_PATH": "/path/to/obsidian/vault"
      }
    }
  }
}
```

This format is identical to Claude Desktop's `claude_desktop_config.json` format -- widely understood by the MCP community and maximally portable. Users can copy server configs between Meetily and Claude Desktop.

### Markdown Generation (Meeting Notes)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| Direct string formatting | N/A | Generate meeting note markdown from LLM output | **No library needed.** Meeting notes are generated by the LLM (Ollama/Claude/Groq), which already outputs markdown. The Rust/Python code just needs to write the string to a `.md` file. String templates with `format!()` (Rust) or f-strings (Python) handle frontmatter injection. | HIGH |
| `comrak` | `0.49.0` | Markdown validation and sanitization (optional, for post-processing) | Only add if we need to parse/validate/transform LLM-generated markdown before writing. Comrak is the gold standard for GFM-compatible Markdown parsing and rendering in Rust. Used by crates.io, docs.rs, GitLab, Deno. Supports AST manipulation + `format_commonmark()` for roundtripping. | MEDIUM |
| `chrono` (already in project) | `0.4` | Timestamp frontmatter in meeting notes | Already a dependency. Used for `date: 2026-02-01T10:30:00Z` in YAML frontmatter. | HIGH |

**Meeting note generation pattern:**
```rust
fn generate_meeting_note(
    meeting_name: &str,
    date: &DateTime<Utc>,
    participants: &[String],
    summary: &str,        // LLM-generated markdown
    action_items: &str,   // LLM-generated markdown
) -> String {
    format!(
        r#"---
title: "{meeting_name}"
date: {date}
participants: [{participants}]
tags: [meeting-notes, meetily]
---

# {meeting_name}

## Summary

{summary}

## Action Items

{action_items}
"#,
        meeting_name = meeting_name,
        date = date.to_rfc3339(),
        participants = participants.iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", "),
        summary = summary,
        action_items = action_items,
    )
}
```

### Obsidian Vault Integration

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| Direct filesystem write | N/A | Write `.md` files to Obsidian vault directory | **No library or API needed.** Obsidian vaults are plain folders of `.md` files. Write markdown to the vault folder and Obsidian auto-detects changes. This is the canonical integration method -- Obsidian's "file over app" philosophy means any program can write to a vault. | HIGH |
| `tauri-plugin-dialog` (already in project) | `2.3.0` | Folder picker for selecting Obsidian vault path | Already a dependency. User picks their vault folder once during workspace setup. | HIGH |

**Integration is trivially simple:**
1. User selects Obsidian vault path via folder picker dialog
2. Store path in workspace `settings.json`
3. After meeting note generation, write `.md` file to `{vault_path}/Meetily/{meeting-name}.md`
4. Obsidian auto-detects the new file

No API, no plugin, no special protocol. Just write files.

### Frontend (React) -- New Components

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| `@radix-ui/react-select` (already in project) | `2.2.5` | Workspace switcher dropdown | Already a dependency. Use for workspace selection UI. | HIGH |
| `@radix-ui/react-dialog` (already in project) | `1.1.14` | Workspace creation dialog, MCP config modal | Already a dependency. | HIGH |
| `@radix-ui/react-tabs` (already in project) | `1.1.12` | MCP server config panel tabs | Already a dependency. | HIGH |
| `react-hook-form` (already in project) | `7.59.0` | MCP server config form, workspace metadata form | Already a dependency with `zod` validation. | HIGH |
| `zod` (already in project) | `3.25.71` | Schema validation for MCP config JSON, workspace settings | Already a dependency. | HIGH |

**No new frontend dependencies needed.** The existing Radix UI + react-hook-form + zod stack covers all UI requirements for workspace management and MCP configuration.

---

## New Dependencies Summary

### Rust (Cargo.toml additions)

```toml
# MCP Client -- Official Rust SDK
rmcp = { version = "0.14", features = ["client", "transport-child-process", "transport-io"] }
```

That is it. One new crate. Everything else is already in the project.

### Optional Rust additions (defer until needed)

```toml
# Only if markdown validation/transformation is needed
comrak = "0.49"

# Only if remote MCP servers are needed (post-v0.3.0)
# Add to rmcp features: "transport-streamable-http-client"
```

### Frontend (package.json additions)

```
None. Zero new npm packages.
```

### Backend (requirements.txt additions)

```
None. Zero new Python packages.
```

The existing `aiosqlite` + `FastAPI` backend does not need changes for workspace isolation. Workspace databases are managed by the Tauri/Rust side. The backend continues serving as the LLM summarization layer.

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| MCP SDK (Rust) | `rmcp` (official) | `mcp-sdk-rs`, `mcp_client_rs`, `mcp_rust_sdk`, `mcpkit`, Prism SDK | `rmcp` is the official SDK under `modelcontextprotocol` org. Others are community forks with smaller ecosystems. No reason to use unofficial when official exists and is actively maintained. |
| MCP Tauri plugin | Custom commands wrapping `rmcp` | `tauri-plugin-mcp-client` (Sublayer), `tauri-plugin-mcp` (airi) | Sublayer plugin not published to crates.io/npm. Airi plugin has 0% docs, 11 versions in 9 months (unstable). Custom commands are 50-100 lines of glue code and give full control. |
| Multi-DB management | `HashMap<String, SqlitePool>` with `RwLock` | SQLite `ATTACH DATABASE` | ATTACH shares a single connection's transaction scope, making isolation harder. Separate pools give true isolation -- each workspace is an independent database file with independent connections. |
| Markdown generation | `format!()` string templates | `comrak` AST builder, `pulldown-cmark`, `tera` templates | LLM output is already markdown. Writing it to a file with YAML frontmatter prepended is string formatting, not AST manipulation. Only add `comrak` if we later need to parse/validate/transform the markdown. |
| Obsidian integration | Direct filesystem writes | Obsidian REST API plugin, Obsidian Local REST API | External plugins require user to install extra Obsidian plugins. Filesystem writes work universally with zero setup -- this is how Obsidian is designed to work. |
| Workspace config storage | `tauri-plugin-store` (per-workspace JSON) | Custom JSON file management, `rusqlite` config table | `tauri-plugin-store` is already in the project, provides cross-language access (Rust + JS), auto-saves, and handles file I/O edge cases. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `tauri-plugin-mcp-client` (Sublayer) | Not published to package registries. GitHub-only. Unknown Tauri 2 compatibility. | Custom Tauri commands wrapping `rmcp` |
| `tauri-plugin-mcp` v0.7.x (airi) | 0% documentation coverage. Rapid version churn (11 versions in 9 months). API may break. | Custom Tauri commands wrapping `rmcp` |
| SQLite `ATTACH DATABASE` for workspace isolation | Shared transaction scope across attached databases. Makes true isolation difficult. Complicates migration management. | Separate `SqlitePool` per workspace |
| Obsidian REST API plugins | Requires user to install third-party Obsidian plugins. Adds a network dependency for what should be a file write. | Direct `.md` file writes to vault folder |
| Building a custom MCP protocol implementation | MCP is a complex protocol (JSON-RPC 2.0, capability negotiation, tool schemas, progress tracking). Reimplementing it is error-prone. | `rmcp` official SDK handles all protocol complexity |
| Workspace state in a single global SQLite database | Defeats the purpose of isolation. Cannot easily export/delete/backup individual workspaces. Cannot have workspace-specific schemas. | One SQLite file per workspace |

---

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `rmcp` 0.14 | `tokio` 1.x | rmcp is built on tokio. Project already uses tokio 1.32+. Compatible. |
| `rmcp` 0.14 | `serde` 1.x, `serde_json` 1.x | rmcp uses serde for JSON-RPC serialization. Already in project. Compatible. |
| `rmcp` 0.14 | MCP spec 2025-06-18 | Official SDK tracks the latest MCP specification. |
| `sqlx` 0.8 | Per-workspace pool pattern | sqlx 0.8 supports dynamic `SqlitePool` creation. No version upgrade needed. |
| `tauri-plugin-store` 2.4 | Per-workspace store files | Store files are path-based. Multiple stores with different paths work out of the box. |
| `tauri` 2.6.2 | All recommended additions | No Tauri upgrade needed. All patterns use existing Tauri 2 APIs. |

---

## MCP Protocol Reference

The MCP specification (version 2025-06-18) defines:

- **Wire format:** JSON-RPC 2.0
- **Transports:** stdio (local processes) and Streamable HTTP (remote servers, replaces deprecated SSE)
- **Server capabilities:** Resources, Prompts, Tools
- **Client capabilities:** Sampling, Roots, Elicitation
- **Connection lifecycle:** initialize -> capability negotiation -> message exchange -> shutdown
- **Security model:** User consent required for all data access and tool execution

For Meetily v0.3.0, we use **stdio transport only** (spawn MCP servers as child processes). This is the simplest, most reliable transport and matches how Claude Desktop, VS Code, and other MCP hosts work.

---

## Sources

- [modelcontextprotocol/rust-sdk (GitHub)](https://github.com/modelcontextprotocol/rust-sdk) -- Official Rust MCP SDK, verified v0.14.0 release (2026-01-23). HIGH confidence.
- [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/) -- API documentation for rmcp crate. Verified client creation flow, transport options, ServiceExt trait. HIGH confidence.
- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18) -- Authoritative protocol specification. Verified transports, capabilities, JSON-RPC format. HIGH confidence.
- [MCP Server Configuration Format](https://modelcontextprotocol.io/docs/develop/connect-local-servers) -- Claude Desktop `mcpServers` JSON format. Verified structure. HIGH confidence.
- [sublayerapp/tauri-plugin-mcp-client (GitHub)](https://github.com/sublayerapp/tauri-plugin-mcp-client) -- Evaluated and rejected. Not published to registries. MEDIUM confidence.
- [tauri-plugin-mcp on crates.io](https://crates.io/crates/tauri-plugin-mcp) -- Evaluated v0.7.1. 0% doc coverage noted. MEDIUM confidence.
- [sqlx Pool documentation](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html) -- Verified Pool is Send+Sync+Clone, supports dynamic creation. HIGH confidence.
- [sqlx SqliteConnectOptions](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html) -- Verified `filename()`, `create_if_missing()`, journal mode options. HIGH confidence.
- [Tauri Store Plugin](https://v2.tauri.app/plugin/store/) -- Verified path-based store creation, shared Rust/JS access. HIGH confidence.
- [comrak on crates.io](https://crates.io/crates/comrak) -- Verified v0.49.0, AST + format_commonmark roundtrip. MEDIUM confidence (version from WebSearch).
- [Obsidian Developer Documentation](https://docs.obsidian.md/Plugins/Vault) -- Confirmed vaults are plain folders of .md files. HIGH confidence.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) -- Verified Manager API for Rust-side state. HIGH confidence.

---

*Stack research for: Meetily v0.3.0 (Workspaces + MCP)*
*Researched: 2026-02-01*
