# Codebase Structure

**Analysis Date:** 2026-02-01

## Directory Layout

```
chays-meets/
├── frontend/                           # Tauri desktop app (Rust + Next.js/React)
│   ├── src/                            # Next.js/React TypeScript source
│   │   ├── app/                        # App Router pages and layouts
│   │   │   ├── page.tsx                # Home/recording interface (main entry)
│   │   │   ├── settings/               # Settings pages
│   │   │   ├── meeting-details/        # Meeting detail view pages
│   │   │   └── notes/                  # Notes editor pages
│   │   ├── components/                 # Reusable React components
│   │   │   ├── Sidebar/                # Meeting list and navigation
│   │   │   ├── ui/                     # UI library components (buttons, modals, etc)
│   │   │   ├── molecules/              # Composite UI components
│   │   │   └── MainNav/                # Top navigation
│   │   ├── contexts/                   # React Context for global state
│   │   │   ├── RecordingStateContext.tsx   # Recording lifecycle state
│   │   │   ├── TranscriptContext.tsx   # Transcript and meeting data
│   │   │   ├── ConfigContext.tsx       # User configuration and settings
│   │   │   ├── RecordingPostProcessingProvider.tsx
│   │   │   └── OllamaDownloadContext.tsx
│   │   ├── hooks/                      # Custom React hooks
│   │   │   ├── useRecordingStart.ts    # Recording start logic
│   │   │   ├── useRecordingStop.ts     # Recording stop logic
│   │   │   ├── useTranscriptRecovery.ts    # Transcript recovery
│   │   │   └── meeting-details/        # Meeting-specific hooks
│   │   ├── services/                   # Business logic services
│   │   │   ├── recordingService.ts     # Tauri invocation wrappers
│   │   │   ├── transcriptService.ts    # Transcript operations
│   │   │   ├── indexedDBService.ts     # Local IndexedDB for caching
│   │   │   └── configService.ts        # Config persistence
│   │   ├── lib/                        # Utility functions
│   │   │   ├── analytics.ts            # Analytics tracking
│   │   │   ├── builtin-ai.ts           # Built-in AI provider integration
│   │   │   └── utils.ts                # General utilities
│   │   ├── types/                      # TypeScript type definitions
│   │   │   └── index.ts                # Shared types
│   │   └── config/                     # Configuration files
│   │       └── api.ts                  # API endpoints config
│   ├── src-tauri/                      # Rust backend for Tauri
│   │   ├── src/                        # Rust source code
│   │   │   ├── lib.rs                  # Main command dispatcher and module exports
│   │   │   ├── main.rs                 # Tauri window initialization
│   │   │   ├── audio/                  # Audio processing (modularized)
│   │   │   │   ├── mod.rs              # Audio module exports
│   │   │   │   ├── devices/            # Audio device detection and management
│   │   │   │   │   ├── discovery.rs    # List and detect audio devices
│   │   │   │   │   ├── microphone.rs   # Microphone device handling
│   │   │   │   │   ├── speakers.rs     # Speaker/output device handling
│   │   │   │   │   ├── configuration.rs    # Device config structs
│   │   │   │   │   └── platform/       # Platform-specific implementations
│   │   │   │   │       ├── macos.rs    # macOS Core Audio logic
│   │   │   │   │       ├── windows.rs  # Windows WASAPI logic
│   │   │   │   │       └── linux.rs    # Linux ALSA/PulseAudio logic
│   │   │   │   ├── capture/            # Audio stream capture
│   │   │   │   │   ├── microphone.rs   # Microphone stream capture
│   │   │   │   │   ├── system.rs       # System audio capture
│   │   │   │   │   └── core_audio.rs   # macOS ScreenCaptureKit integration
│   │   │   │   ├── pipeline.rs         # Audio mixing and VAD processor (critical)
│   │   │   │   ├── recording_manager.rs    # High-level recording coordination
│   │   │   │   ├── recording_commands.rs   # Tauri command wrappers
│   │   │   │   ├── recording_state.rs  # Shared recording state (Arc/RwLock)
│   │   │   │   ├── recording_saver.rs  # Audio file accumulation and writing
│   │   │   │   ├── stream.rs           # Audio stream management
│   │   │   │   ├── transcription/      # Transcription provider abstraction
│   │   │   │   │   ├── engine.rs       # Transcription orchestrator
│   │   │   │   │   ├── whisper_provider.rs    # Whisper.cpp provider
│   │   │   │   │   ├── parakeet_provider.rs   # Parakeet STT provider
│   │   │   │   │   ├── provider.rs     # Provider trait definition
│   │   │   │   │   └── worker.rs       # Transcription worker pool
│   │   │   │   ├── vad.rs              # Voice Activity Detection processor
│   │   │   │   ├── audio_processing.rs # Audio effects (noise suppression, normalization)
│   │   │   │   ├── buffer_pool.rs      # Pre-allocated audio buffer pool
│   │   │   │   ├── batch_processor.rs  # Batch metrics processing
│   │   │   │   ├── level_monitor.rs    # Audio level monitoring
│   │   │   │   ├── permissions.rs      # OS permission requests
│   │   │   │   └── device_monitor.rs   # Device connect/disconnect detection
│   │   │   ├── whisper_engine/         # Whisper model management
│   │   │   │   └── whisper_engine.rs   # Model loading, transcription
│   │   │   ├── audio_v2/               # Alternative audio system (newer implementation)
│   │   │   │   ├── mixer.rs            # Audio mixing
│   │   │   │   ├── recorder.rs         # Recording wrapper
│   │   │   │   └── sync.rs             # Stream synchronization
│   │   │   ├── database/               # Local SQLite database
│   │   │   │   ├── manager.rs          # Database connection and migrations
│   │   │   │   ├── models.rs           # SQLx data models
│   │   │   │   ├── commands.rs         # Tauri database commands
│   │   │   │   ├── repositories/       # Data access layer
│   │   │   │   │   ├── meeting.rs      # Meeting CRUD
│   │   │   │   │   ├── transcript.rs   # Transcript CRUD
│   │   │   │   │   ├── summary.rs      # Summary persistence
│   │   │   │   │   └── setting.rs      # User settings
│   │   │   │   └── setup.rs            # Database initialization
│   │   │   ├── summary/                # Meeting summary generation
│   │   │   │   ├── mod.rs              # Module definition and config types
│   │   │   │   ├── llm_client.rs       # Multi-provider LLM client
│   │   │   │   ├── processor.rs        # Transcript chunking and processing
│   │   │   │   ├── service.rs          # Summary generation service
│   │   │   │   ├── commands.rs         # Tauri summary commands
│   │   │   │   ├── summary_engine.rs   # Summary orchestration
│   │   │   │   ├── templates/          # Summary template directory
│   │   │   │   └── template_commands.rs    # Template management commands
│   │   │   ├── parakeet_engine/        # Parakeet STT engine integration
│   │   │   ├── ollama/                 # Ollama LLM provider
│   │   │   ├── openrouter/             # OpenRouter API client
│   │   │   ├── api/                    # HTTP API client helpers
│   │   │   ├── notifications/          # Desktop notification management
│   │   │   ├── analytics/              # Analytics tracking
│   │   │   ├── console_utils/          # Developer console utilities
│   │   │   ├── state.rs                # Global app state
│   │   │   ├── tray.rs                 # System tray menu
│   │   │   ├── onboarding.rs           # Onboarding flow
│   │   │   └── utils.rs                # Utility functions
│   │   ├── migrations/                 # SQLx database migrations
│   │   ├── src-tauri.conf.json         # Tauri configuration
│   │   ├── Cargo.toml                  # Rust dependencies
│   │   └── .cargo/config               # Cargo build configuration
│   ├── public/                         # Static assets
│   ├── next.config.js                  # Next.js configuration
│   ├── tsconfig.json                   # TypeScript configuration
│   ├── tailwind.config.js              # Tailwind CSS configuration
│   └── package.json                    # Node.js dependencies (pnpm)
├── backend/                            # FastAPI Python backend
│   ├── app/                            # Main application
│   │   ├── main.py                     # FastAPI app and endpoint definitions
│   │   ├── db.py                       # SQLite database manager
│   │   ├── models.py                   # Pydantic request/response models
│   │   ├── transcript_processor.py     # Transcript processing and chunking
│   │   ├── llm_provider.py             # LLM client for summarization
│   │   └── schema_validator.py         # Database schema validation
│   ├── docker/                         # Docker configuration
│   │   └── Dockerfile                  # Container build
│   ├── whisper-custom/                 # Custom Whisper server
│   │   └── server/                     # Whisper HTTP server
│   ├── examples/                       # Example scripts
│   ├── run-docker.sh                   # Docker launcher script
│   ├── requirements.txt                # Python dependencies
│   └── .env.example                    # Environment variables template
├── scripts/                            # Utility scripts
│   └── *.sh                            # Platform-specific build/run scripts
├── .github/                            # GitHub configuration
│   ├── workflows/                      # CI/CD workflows
│   └── ISSUE_TEMPLATE/                 # Issue templates
├── .planning/                          # GSD planning documents
│   └── codebase/                       # Codebase analysis files
│       ├── ARCHITECTURE.md             # Architecture patterns and layers
│       └── STRUCTURE.md                # This file
└── CLAUDE.md                           # Project instructions for Claude Code
```

## Directory Purposes

**frontend/src/**
- Purpose: React/TypeScript UI code
- Contains: Page components, reusable components, hooks, contexts, services, types
- Key files: `page.tsx` (main recording interface), `contexts/` (global state)

**frontend/src-tauri/src/**
- Purpose: Rust backend implementation for Tauri app
- Contains: Audio capture, transcription, database access, Tauri command handlers
- Key files: `lib.rs` (command dispatcher), `audio/` (audio system), `database/` (persistence)

**frontend/src-tauri/src/audio/**
- Purpose: Comprehensive audio subsystem for recording and transcription
- Contains: Device discovery, capture streams, pipeline mixing, VAD, Whisper integration
- Key files: `recording_manager.rs` (orchestrator), `pipeline.rs` (mixing), `transcription/` (STT)

**backend/app/**
- Purpose: FastAPI server for meeting persistence and summarization
- Contains: REST endpoints, database layer, LLM clients, transcript processing
- Key files: `main.py` (endpoints), `db.py` (database), `llm_provider.py` (summarization)

## Key File Locations

**Entry Points:**
- `frontend/src/app/page.tsx`: Main recording interface (Next.js page)
- `frontend/src-tauri/src/main.rs`: Tauri window initialization
- `frontend/src-tauri/src/lib.rs`: Tauri command dispatcher (defines all available commands)
- `backend/app/main.py`: FastAPI application and all REST endpoints

**Configuration:**
- `frontend/src-tauri/src-tauri.conf.json`: Tauri app config (window settings, permissions)
- `frontend/src/config/api.ts`: Frontend API endpoint URLs
- `backend/.env`: Backend environment variables (database path, API keys)
- `frontend/src-tauri/.cargo/config`: Rust build profile configuration

**Core Logic:**
- `frontend/src-tauri/src/audio/recording_manager.rs`: Main recording orchestrator
- `frontend/src-tauri/src/audio/pipeline.rs`: Audio mixing and VAD (critical path)
- `frontend/src-tauri/src/audio/transcription/`: Transcription provider abstraction
- `frontend/src-tauri/src/summary/llm_client.rs`: LLM provider routing
- `frontend/src-tauri/src/database/manager.rs`: Database connection pool and migrations
- `frontend/src/contexts/RecordingStateContext.tsx`: Global recording state
- `backend/app/transcript_processor.py`: Transcript chunking and processing

**Testing:**
- Rust tests: Located in each module file with `#[cfg(test)]` blocks (in-source tests)
- Frontend: Integration tests would be in `frontend/__tests__/` (not heavily used)
- Backend: Tests could be added to `backend/tests/` directory (not found)

## Naming Conventions

**Files:**

- TypeScript components: PascalCase for components (e.g., `RecordingControls.tsx`)
- TypeScript utilities: camelCase for functions and services (e.g., `recordingService.ts`)
- Rust files: snake_case (e.g., `recording_manager.rs`, `audio_processing.rs`)
- Python files: snake_case (e.g., `transcript_processor.py`, `llm_provider.py`)

**Directories:**

- Feature-based grouping: Group by domain (e.g., `audio/`, `database/`, `summary/`)
- Platform-specific: Suffix with platform (e.g., `platform/macos.rs`, `platform/windows.rs`)
- Internal modules: Use `mod.rs` for directory index, re-export public items

**TypeScript Types:**

- Interfaces prefixed with capitalized name (e.g., `RecordingState`, `TranscriptUpdate`)
- Enums in PascalCase (e.g., `RecordingStatus`, `LLMProvider`)
- Context types suffixed with `ContextType` (e.g., `SidebarContextType`)

**Rust Types:**

- Structs in PascalCase (e.g., `RecordingManager`, `AudioDevice`)
- Enums in PascalCase (e.g., `DeviceType`, `RecordingStatus`)
- Modules in snake_case (e.g., `recording_manager`, `audio_processing`)

## Where to Add New Code

**New Feature (Audio):**
- Primary code: `frontend/src-tauri/src/audio/[feature_name].rs`
- Tauri command wrapper: Add to `frontend/src-tauri/src/audio/recording_commands.rs`
- Frontend integration: Add hook in `frontend/src/hooks/` and dispatch in component

**New Feature (Summary/LLM):**
- LLM provider: `frontend/src-tauri/src/summary/llm_client.rs` (add provider match arm)
- Backend endpoint: `backend/app/main.py` (add route handler)
- Frontend UI: `frontend/src/app/settings/` (add configuration UI)

**New Component:**
- Implementation: `frontend/src/components/[category]/ComponentName.tsx`
- Shared components: `frontend/src/components/shared/`
- UI library components: `frontend/src/components/ui/`

**Utilities:**
- Frontend helpers: `frontend/src/lib/utils.ts` or `frontend/src/services/`
- Rust utilities: `frontend/src-tauri/src/utils.rs`
- Python utilities: `backend/app/` with descriptive module names

**Database Migrations:**
- SQLite (Tauri side): `frontend/src-tauri/migrations/` (SQLx migrations)
- Schema changes: Add `V{YYYYMMDD}_{description}.sql` file
- Rust models: Update `frontend/src-tauri/src/database/models.rs`

**Endpoints:**
- Backend API: `backend/app/main.py` in appropriate section (meetings, transcripts, summaries)
- Response types: Add Pydantic models at top of `main.py`
- Database: Add methods to `backend/app/db.py` DatabaseManager class

## Special Directories

**frontend/src-tauri/migrations/**
- Purpose: SQLx database migration files
- Generated: SQLx generates schema from SQL files
- Committed: Yes, version control migrations
- Format: `V{YYYYMMDD}_{description}.sql`

**frontend/src-tauri/src/audio_v2/**
- Purpose: Alternative/newer audio implementation (in development)
- Generated: No
- Committed: Yes (work in progress)

**backend/whisper-custom/**
- Purpose: Custom Whisper HTTP server wrapper (for non-Rust environments)
- Generated: No (source code)
- Committed: Yes

**frontend/public/**
- Purpose: Static assets (icons, fonts, images)
- Generated: No
- Committed: Yes

**frontend/src-tauri/logs/**
- Purpose: Runtime log files and diagnostic output
- Generated: Yes (at runtime)
- Committed: No (in .gitignore)

**frontend/src-tauri/icons/**
- Purpose: App icon sources for different platforms
- Generated: No (source)
- Committed: Yes

## Navigation Guide by Task Type

**Recording Flow Bug:**
- Start: `frontend/src/app/page.tsx` (UI that triggers recording)
- → `frontend/src/hooks/useRecordingStart.ts` (start logic)
- → `frontend/src-tauri/src/audio/recording_commands.rs` (Tauri command)
- → `frontend/src-tauri/src/audio/recording_manager.rs` (orchestration)

**Transcription Issue:**
- Start: `frontend/src-tauri/src/audio/transcription/` (provider selection)
- → `frontend/src-tauri/src/whisper_engine/` or parakeet_engine/ (model management)
- → Check `frontend/src-tauri/src/audio/pipeline.rs` (VAD filtering)

**Database/Persistence:**
- Tauri side: `frontend/src-tauri/src/database/` (manager, repositories)
- Backend side: `backend/app/db.py` (DatabaseManager)
- Migrations: `frontend/src-tauri/migrations/` (add SQL migration if schema changes needed)

**LLM/Summarization:**
- Rust client: `frontend/src-tauri/src/summary/llm_client.rs`
- Service layer: `frontend/src-tauri/src/summary/service.rs`
- Backend: `backend/app/main.py` (POST /api/process-transcript)
- Frontend: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`

**Audio Device Detection:**
- Device discovery: `frontend/src-tauri/src/audio/devices/discovery.rs`
- Platform-specific: `frontend/src-tauri/src/audio/devices/platform/{macos,windows,linux}.rs`
- Configuration: `frontend/src-tauri/src/audio/devices/configuration.rs`

**Global State Changes:**
- Recording state: `frontend/src/contexts/RecordingStateContext.tsx`
- Meeting list: `frontend/src/components/Sidebar/SidebarProvider.tsx`
- Config: `frontend/src/contexts/ConfigContext.tsx`
- Transcripts: `frontend/src/contexts/TranscriptContext.tsx`

---

*Structure analysis: 2026-02-01*
