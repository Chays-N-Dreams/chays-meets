# Architecture

**Analysis Date:** 2026-02-01

## Pattern Overview

**Overall:** Three-tier distributed architecture with Tauri IPC bridge between frontend and native backend.

**Key Characteristics:**
- Tauri 2.x desktop application (Rust + Next.js/React) with native audio/transcription processing
- FastAPI backend server for meeting persistence, LLM summarization, and coordination
- Local-first design: transcription and audio processing happen on-device via Whisper.cpp
- Event-driven communication: Rust emits events to frontend via Tauri; frontend invokes Tauri commands
- Multi-provider LLM abstraction: Ollama, Claude, Groq, OpenRouter with fallback support

## Layers

**Presentation Layer (Next.js/React):**
- Purpose: User interface for recording, viewing meetings, and configuration
- Location: `frontend/src/` (TSX/TS files)
- Contains: React components, pages, hooks, contexts for global state
- Depends on: Tauri commands (via `@tauri-apps/api`), contexts (ConfigContext, RecordingStateContext, TranscriptContext)
- Used by: End users via Tauri desktop window

**Tauri/Rust Backend (Native Layer):**
- Purpose: Core meeting recording, audio processing, transcription, and Rust ↔ Frontend IPC
- Location: `frontend/src-tauri/src/` (Rust source)
- Contains: Audio capture, mixing, VAD, Whisper transcription, database persistence, command handlers
- Depends on: Audio devices (cpal), Whisper models, local SQLite, Ollama/LLM APIs
- Used by: Frontend (via Tauri commands/events), FastAPI backend (via HTTP for meeting sync)

**Backend API (FastAPI):**
- Purpose: Meeting storage, LLM-based summarization, transcript management
- Location: `backend/app/main.py`
- Contains: REST endpoints, LLM client, database models, transcript processing
- Depends on: SQLite (aiosqlite), LLM providers (Ollama/OpenRouter/Claude), Whisper server
- Used by: Tauri app (HTTP calls to `http://localhost:5167`), optional external clients

**Audio Processing Pipeline (Rust):**
- Purpose: Synchronized capture, mixing, noise reduction, VAD filtering of mic + system audio
- Location: `frontend/src-tauri/src/audio/`
- Contains: Ring buffer mixing, RMS-based ducking, VAD processor, FFmpeg integration
- Depends on: cpal (audio capture), rubato (resampling), onnxruntime (VAD)
- Used by: Recording manager for creating mixed audio files and filtered transcription streams

## Data Flow

**Recording Workflow:**

1. Frontend calls `start_recording` command (Tauri) with device names and meeting title
2. Rust recording manager starts audio capture from microphone and system audio devices
3. Audio pipeline manager creates two parallel streams:
   - **Recording path**: Raw audio → Professional mixer (RMS-based ducking, clipping prevention) → WAV accumulation → saved to disk
   - **Transcription path**: Raw audio → VAD filter (voice activity detection) → sent to Whisper engine
4. Whisper engine processes VAD-filtered audio chunks, emits `transcript-update` events to frontend
5. Frontend receives events, updates UI with live transcripts via TranscriptContext
6. Stop button: Rust finalizes audio file, waits for transcription completion, emits `transcription-complete`
7. Frontend saves meeting + transcripts to backend API via HTTP POST

**State Management Flow:**

1. RecordingStateContext provides single source of truth for recording state (idle, recording, stopping, processing, saving)
2. RecordingStatus enum tracks lifecycle: IDLE → STARTING → RECORDING → STOPPING → PROCESSING_TRANSCRIPTS → SAVING → COMPLETED
3. Frontend hooks (useRecordingStart, useRecordingStop) dispatch status changes to context
4. Sidebar context manages list of meetings, current meeting, and summary polling
5. ConfigContext manages audio device selection, LLM provider settings, transcript model preferences

**Summarization Flow:**

1. Frontend calls `api_process_transcript` with transcript text and LLM provider config
2. Rust summary service chunks transcript text (sliding window to preserve context)
3. Chunks sent to LLM provider (Ollama local or cloud provider)
4. Summary response stored in database, polled via `startSummaryPolling` in SidebarProvider
5. Results include structured sections: key points, action items, summary

## Key Abstractions

**AudioDevice:**
- Purpose: Unified interface to audio input/output devices across macOS, Windows, Linux
- Examples: `frontend/src-tauri/src/audio/devices/discovery.rs`, `devices/platform/macos.rs`
- Pattern: Platform-specific discovery, unified AudioDevice struct with device name, channels, sample rate

**TranscriptionProvider:**
- Purpose: Abstraction layer for different transcription engines (Whisper, Parakeet)
- Examples: `frontend/src-tauri/src/audio/transcription/whisper_provider.rs`, `parakeet_provider.rs`
- Pattern: Provider trait with transcribe() method, engine manager routes audio chunks to active provider

**LLMProvider:**
- Purpose: Unified interface to multiple LLM services (Ollama, Claude, Groq, OpenRouter)
- Examples: `frontend/src-tauri/src/summary/llm_client.rs`
- Pattern: Client factory pattern, request/response marshaling for each provider's API format

**DatabaseManager:**
- Purpose: Async SQLite operations for meeting/transcript persistence
- Location: `frontend/src-tauri/src/database/manager.rs` (Tauri/Rust side) and `backend/app/db.py` (FastAPI side)
- Pattern: Connection pooling (sqlx for Rust, aiosqlite for Python), migration-based schema management

**RecordingState:**
- Purpose: Shared atomic state for recording lifecycle across async tasks
- Location: `frontend/src-tauri/src/audio/recording_state.rs`
- Pattern: Arc<RwLock<T>> for async-safe shared state, Arc<AtomicBool> for simple flags

## Entry Points

**Frontend (Next.js):**
- Location: `frontend/src/app/page.tsx`
- Triggers: Application startup, user clicks "New Call"
- Responsibilities: Render recording interface, manage global state via contexts, dispatch Tauri commands

**Tauri Main:**
- Location: `frontend/src-tauri/src/main.rs`
- Triggers: Application launch
- Responsibilities: Initialize Tauri window, set up logging, register Tauri commands

**Tauri Commands (Command Dispatcher):**
- Location: `frontend/src-tauri/src/lib.rs`
- Triggers: Frontend invoke calls
- Responsibilities: Parse command arguments, delegate to modules (audio, summary, database), emit events

**Audio Recording Command:**
- Location: `frontend/src-tauri/src/audio/recording_commands.rs` → `recording_manager.rs`
- Triggers: Frontend `invoke('start_recording', {...})`
- Responsibilities: Validate devices, spawn audio capture tasks, coordinate pipeline and transcription

**Backend API:**
- Location: `backend/app/main.py`
- Triggers: HTTP requests from Tauri app (POST /api/save-transcript, GET /api/meetings)
- Responsibilities: CRUD operations for meetings, transcript processing, LLM summarization

## Error Handling

**Strategy:** Layered error propagation with user-friendly frontend messages.

**Patterns:**
- Rust: `anyhow::Result<T>` for internal functions, convert to `Result<T, String>` for Tauri commands
- Frontend: Try-catch wrapping invoke() calls, toast notifications for user feedback
- Backend: FastAPI HTTPException with status codes, detailed error logging

**Critical Paths:**
- Audio capture errors → frontend shows "Microphone not available" modal, fallback to mic-only recording
- Transcription timeout → frontend shows "Transcription taking longer than expected"
- LLM provider unavailable → summary generation shows provider error, suggests alternatives

## Cross-Cutting Concerns

**Logging:**
- Rust: `log` crate with conditional compilation (perf_debug! and perf_trace! zero out in release builds)
- Frontend: console.log for development, error toast notifications for user-facing issues
- Backend: Python logging with formatted output including filename:line:function

**Validation:**
- Audio: Device availability checked before recording start, sample rate validation
- Transcripts: VAD-filtered to prevent empty chunks, minimum audio duration enforced
- LLM requests: Chunk size limits (2000 tokens), provider API key validation on save

**Authentication:**
- LLM providers: API keys stored in ConfigContext (frontend) or database (backend)
- Ollama: Local network access assumed, no auth required
- Backend API: CORS configured to allow all origins (development only, restrict for production)

**Database Transactions:**
- Tauri: SQLx automatic transaction rollback on error, migration-based schema versioning
- Backend: Async database operations with connection pooling, row-level locking for concurrent access

---

*Architecture analysis: 2026-02-01*
