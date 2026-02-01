# External Integrations

**Analysis Date:** 2026-02-01

## APIs & External Services

**LLM Providers (for meeting summarization):**
- Claude (Anthropic)
  - SDK/Client: `pydantic_ai.models.anthropic.AnthropicModel`
  - Auth: `ANTHROPIC_API_KEY` environment variable
  - Used by: `backend/app/transcript_processor.py` via Pydantic AI framework
  - Provider string: `"claude"`
  - Models: User configurable, recommended `claude-3-5-sonnet-20241022` (from CLAUDE.md)

- Groq
  - SDK/Client: `pydantic_ai.models.groq.GroqModel`
  - Auth: `GROQ_API_KEY` environment variable
  - Used by: `backend/app/transcript_processor.py`
  - Provider string: `"groq"`
  - Storage: API key stored in SQLite `settings.groqApiKey` and `transcript_settings.groqApiKey`

- OpenAI
  - SDK/Client: `pydantic_ai.models.openai.OpenAIModel`
  - Auth: `OPENAI_API_KEY` environment variable
  - Used by: `backend/app/transcript_processor.py`
  - Provider string: `"openai"`
  - Default model: `gpt-4o-2024-11-20`
  - Storage: API key stored in SQLite `settings.openaiApiKey` and `transcript_settings.openaiApiKey`

- Ollama (Local)
  - SDK/Client: `ollama.AsyncClient` (Python) and `reqwest` HTTP calls (Rust)
  - Connection: `http://localhost:11434` (configurable via `OLLAMA_HOST`)
  - Used by: `backend/app/transcript_processor.py` and Rust frontend at `frontend/src-tauri/src/ollama/ollama.rs`
  - Provider string: `"ollama"`
  - Features: Model pull, delete, context window queries, async inference
  - Storage: Optional `ollamaApiKey` in `settings` table (legacy, mostly unused)

**Analytics & Telemetry:**
- PostHog
  - SDK/Client: `posthog_rs` 0.3.7 (Rust)
  - Host: `https://us.i.posthog.com` (default, configurable)
  - Used by: `frontend/src-tauri/src/analytics/analytics.rs`
  - Configuration: `AnalyticsConfig` with `api_key`, `host`, `enabled` fields
  - Features: Event tracking, user identification, session management
  - Status: Integrated but disabled by default (`enabled: false`)

**Model & Resource Downloads:**
- Ollama Hub
  - Endpoint: `http://localhost:11434` (proxy to remote registry)
  - Used by: Model pulling and listing at `frontend/src-tauri/src/ollama/ollama.rs`
  - Features: `get_ollama_models()`, `pull_ollama_model()`, `delete_ollama_model()`

- GitHub Releases
  - Endpoint: `https://github.com/Zackriya-Solutions/meeting-minutes/releases/latest/download/latest.json`
  - Used by: Tauri updater plugin for app version management
  - Configuration: `frontend/src-tauri/tauri.conf.json` under `plugins.updater.endpoints`

## Data Storage

**Databases:**
- SQLite (Primary)
  - Provider: Local file-based relational database
  - Connection: `aiosqlite` (Python async) and `sqlx` (Rust compile-time verified)
  - Path: Configurable via `DATABASE_PATH` or `DB_PATH` env var (default: `meeting_minutes.db`)
  - Client Library: `aiosqlite==0.21.0`
  - Schema Location: `backend/app/db.py` (schema definition via SQL)
  - Tables:
    - `meetings` - Meeting metadata with timestamps
    - `transcripts` - Transcript chunks with audio timing metadata
    - `summary_processes` - Processing status and results
    - `transcript_chunks` - Processed chunks storage
    - `settings` - LLM provider configurations and API keys
    - `transcript_settings` - Transcription provider settings

**File Storage:**
- Local Filesystem (Production)
  - Meeting recordings: User's app data directory per platform
  - macOS: `~/Library/Application Support/Meetily/`
  - Windows: `%APPDATA%\Meetily\`
  - Linux: `~/.config/Meetily/`
  - Access: Tauri `@tauri-apps/plugin-fs` 2.4.0
  - File types: WAV (audio recordings), JSON (transcripts/summaries)

- Tauri Asset Protocol
  - Scope: `$APPDATA/**` for bundled resources
  - Used for: Template files in `frontend/src-tauri/templates/`
  - Security: Asset protocol restricted to app data directory

**Caching:**
- Model Metadata Cache (In-Memory)
  - Implementation: `ModelMetadataCache` in `frontend/src-tauri/src/ollama/metadata.rs`
  - TTL: 5 minutes (300 seconds)
  - Purpose: Cache Ollama model information to reduce server load

- Tauri Store (Persistent Key-Value)
  - Plugin: `@tauri-apps/plugin-store` 2.4.0
  - Purpose: Store user settings, preferences, API configurations
  - Location: Platform-specific app config directory

## Authentication & Identity

**Auth Provider:**
- None (Open/Unauthenticated)
  - Implementation: No user login system
  - User identity: Local machine-based (no multi-user support)
  - API Keys: Stored locally in SQLite, managed by user through UI settings

**API Key Management:**
- Storage: SQLite `settings` and `transcript_settings` tables
- Encryption: Stored in plain text (not encrypted at rest)
- Retrieval: `DatabaseManager.get_api_key()` in `backend/app/db.py`
- Storage Methods:
  - `async def save_api_key(provider: str, api_key: str)` - Persists provider-specific keys
  - `async def get_api_key(provider: str)` - Retrieves provider-specific keys

**CORS Configuration:**
- Backend: `allow_origins=["*"]` (all origins allowed)
- Location: `backend/app/main.py` CORSMiddleware setup
- Status: Development/testing configuration, should be restricted for production

## Monitoring & Observability

**Error Tracking:**
- PostHog (optional)
  - Status: Integrated but disabled by default
  - Implementation: `AnalyticsClient.track_error()` method available
  - Used for: User session tracking and event monitoring

**Logs:**
- Console/File Output
  - Backend: Python `logging` module with formatted output
  - Format: `%(asctime)s - %(levelname)s - [%(filename)s:%(lineno)d - %(funcName)s()] - %(message)s`
  - Level: DEBUG by default
  - Location: Terminal stdout/stderr

- Rust Logging
  - Backend: `log` crate with `env_logger`
  - Macros: Performance-aware `perf_debug!()` and `perf_trace!()` (zero-cost in release)
  - Configured: `RUST_LOG` environment variable

- Tauri Logger Plugin
  - Plugin: `tauri-plugin-log` 2.6.0 (macOS only)
  - Features: Colored output, structured logging

## CI/CD & Deployment

**Hosting:**
- Desktop: Standalone Tauri application (bundled)
- Backend: Docker containers or standalone Python
- Distribution: GitHub Releases via Tauri updater

**CI Pipeline:**
- GitHub Actions
  - Configuration: `.github/workflows/` directory
  - Triggers: Manual dispatch for release workflows
  - Jobs: Build, sign, release to GitHub

**Update Mechanism:**
- Tauri Updater Plugin 2.3.0
  - Endpoint: GitHub Releases latest.json
  - Public Key: Configured in `tauri.conf.json`
  - Artifacts: Signed and versioned releases

**Docker Support:**
- Backend Containers:
  - `Dockerfile.server-cpu` - Python + CPU transcription
  - `Dockerfile.server-gpu` - Python + GPU support
  - `Dockerfile.server-macos` - macOS-specific backend
  - Scripts: `run-docker.sh`, `run-docker.ps1` for orchestration
  - Compose: `docker-compose.yml` for multi-service setup

## Environment Configuration

**Required Environment Variables:**

*For Backend API:*
```
# LLM Providers
ANTHROPIC_API_KEY=your-key          # For Claude integration
GROQ_API_KEY=your-key               # For Groq integration
OPENAI_API_KEY=your-key             # For OpenAI integration

# Ollama (Local LLM)
OLLAMA_HOST=http://localhost:11434  # Default to localhost

# Database
DATABASE_PATH=./meetings.db          # SQLite file location

# Server
HOST=0.0.0.0                         # Listen on all interfaces
PORT=5167                            # API port

# Transcription
CHUNK_SIZE=5000                      # Processing chunk size
CHUNK_OVERLAP=1000                   # Chunk context overlap
```

*For Frontend/Desktop:*
```
# Code Signing (CI/CD only)
TAURI_SIGNING_PRIVATE_KEY=base64-encoded-key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=key-password
```

**Secrets Location:**
- Development: `.env` files (git-ignored)
- Production: Environment variables set in deployment
- Desktop App: User provides via UI settings modal
- Backend: `.env` file or environment variables

## Webhooks & Callbacks

**Incoming:**
- Tauri Events (Frontend → Rust Backend)
  - Protocol: IPC (Inter-Process Communication)
  - Examples: `start_recording`, `stop_recording`, `generate_summary`
  - Location: `frontend/src-tauri/src/lib.rs` command handlers

- HTTP POST Endpoints (React → FastAPI Backend)
  - `/api/transcripts/save` - Save transcripts
  - `/api/transcripts/generate` - Generate summary
  - `/api/meetings` - Meeting CRUD operations
  - `/api/configs/model` - Save model configuration
  - Protocol: HTTP/JSON over localhost

**Outgoing:**
- Tauri Events (Rust Backend → Frontend)
  - `transcript-update` - Streaming transcript segments
  - `recording-status` - Recording state changes
  - `summary-completed` - Summary generation done
  - `parakeet-download-progress` - Model download progress
  - Protocol: Native Tauri event system

- HTTP Requests (Backend → LLM APIs)
  - Anthropic API: POST /messages
  - Groq API: Chat completion endpoint
  - OpenAI API: Chat completion endpoint
  - Ollama API: Local /api/generate endpoint

## Service Endpoints

**Development/Default:**
- Frontend (Next.js Dev): `http://localhost:3118`
- Backend API: `http://localhost:5167`
- Backend Swagger Docs: `http://localhost:5167/docs`
- Backend ReDoc: `http://localhost:5167/redoc`
- Whisper Server: `http://localhost:8178` (external service, optional)
- Ollama Server: `http://localhost:11434`

**Tauri CSP Configuration:**
- Connect sources allowed: `localhost:11434`, `localhost:5167`, `localhost:8178`, `https://api.ollama.ai`
- Asset sources: `asset: https://asset.localhost data:`
- Image sources: `'self' asset: https://asset.localhost data:`

## Model Management

**Whisper Models (Speech-to-Text):**
- Supported sizes: tiny, tiny.en, base, base.en, small, small.en, medium, medium.en, large-v1, large-v2, large-v3, large-v3-turbo
- Storage locations:
  - Development: `frontend/models/` or `backend/whisper-server-package/models/`
  - Production: Platform-specific app data directory
- Loading: Automatic GPU detection (Metal/CUDA/Vulkan) with CPU fallback

**Ollama Models:**
- Discovery: `GET http://localhost:11434/api/tags`
- Pull command: `ollama pull <model-name>`
- Management: Frontend UI at `frontend/src/app/_components/SettingsModal.tsx`

**LLM Model Configuration:**
- Storage: SQLite `settings` table (provider, model, whisperModel)
- Configuration file format: JSON with provider, model, and API key fields

---

*Integration audit: 2026-02-01*
