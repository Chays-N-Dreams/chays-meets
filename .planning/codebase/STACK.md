# Technology Stack

**Analysis Date:** 2026-02-01

## Languages

**Primary:**
- Rust 1.77+ - Desktop application backend and audio processing, Tauri framework runtime
- TypeScript 5.7.2 - Next.js frontend and React UI components
- Python 3.8+ - FastAPI backend server for meeting persistence and LLM integration

**Secondary:**
- Swift - macOS-specific screen capture and audio permissions (ScreenCaptureKit)
- WASM - Browser-compatible transcription in frontend (Whisper runtime)
- SQL - SQLite schema and migrations

## Runtime

**Environment:**
- Node.js 20+ - Frontend development and Next.js runtime
- Python 3.8+ - Backend FastAPI server runtime
- Rust compiler 1.77+ - Tauri desktop application compilation

**Package Manager:**
- pnpm (frontend) - monorepo-compatible, faster alternative to npm
- pip (Python) - Standard Python package management
- Cargo (Rust) - Rust package management with workspace support

## Frameworks

**Core Frontend:**
- Next.js 14.2.25 - React framework with SSR, deployed as static export
- React 18.2.0 - UI component library and state management
- Tauri 2.6.2 - Desktop application wrapper, replaces Electron
- Tauri Plugins 2.3.0+ - File system, notifications, stores, updater, process management

**UI Components & Styling:**
- Radix UI 1.4.3 - Unstyled accessible component primitives
- TailwindCSS 3.4.1 - Utility-first CSS framework
- shadcn/ui - Pre-styled components built on Radix UI
- BlockNote 0.36.0 - Rich text editor for meeting notes
- TipTap 2.10.4 - Extensible rich text editor framework
- Remirror 3.0.1 - Markdown and rich text editing with extensions
- Framer Motion 11.15.0 - Animation library for UI transitions
- Lucide React 0.469.0 - Icon library

**Backend Server:**
- FastAPI 0.115.9 - Async Python web framework for REST API
- Uvicorn 0.34.0 - ASGI server implementation
- Pydantic 2.11.5 - Data validation and serialization
- Pydantic AI 0.2.15 - AI model integration framework (Claude, Groq, OpenAI, Ollama)

**Audio & Transcription:**
- Whisper.cpp (Rust binding: whisper-rs 0.13.2) - Local speech-to-text transcription
- Parakeet (ONNX Runtime) - Fast ONNX-based transcription model
- Silero-rs (git fork) - Voice Activity Detection (VAD) for filtering non-speech audio
- cpal 0.15.3 - Cross-platform audio capture from microphone and system
- Symphonia 0.5.4 - Audio codec support (AAC, MP4, SIMD optimization)
- EBU R128 0.1 - Professional broadcast loudness normalization
- NNNoiseless 0.5 - Neural network-based noise suppression
- Rubato 0.15.0 - Audio resampling for sample rate alignment

**State Management:**
- React Context API - Global application state (meetings, sidebar, config)
- Tauri Store Plugin 2.4.0 - Persistent key-value storage for settings
- RwLock + Arc (Rust) - Thread-safe state for concurrent audio processing

**Testing:**
- Jest (configured but minimal coverage observed)
- Criterion - Rust benchmarking framework

## Key Dependencies

**Critical (Direct Integration):**
- `@tauri-apps/api` 2.6.0 - IPC communication between React frontend and Rust backend
- `@tauri-apps/plugin-fs` 2.4.0 - File system access for saving recordings and meetings
- `@tauri-apps/plugin-store` 2.4.0 - Persistent user settings (API keys, model preferences)
- `@tauri-apps/plugin-notification` 2.3.1 - Desktop notifications for recording events
- `aiosqlite` 0.21.0 - Async SQLite client for Python backend
- `posthog-rs` 0.3.7 - Analytics event tracking

**Infrastructure & Communication:**
- `reqwest` 0.11 - HTTP client for Rust (API calls, model downloads)
- `tokio` 1.32.0 - Async runtime for Rust with full feature set
- `sqlx` 0.8 - SQL toolkit with compile-time query verification (frontend Rust only)
- `axon-ai/pydantic-ai` - AI model abstraction layer (Claude, Groq, OpenAI, Ollama)
- `ollama` 0.5.2 - Python client for Ollama API

**Audio Processing Libraries:**
- `ffmpeg-sidecar` - FFmpeg integration for audio format conversion
- `ringbuf` 0.4.8 - Lock-free ring buffer for concurrent audio stream buffering
- `realfft` 3.4.0 - Fast Fourier Transform for audio analysis
- `ndarray` 0.16 - Multi-dimensional array operations

**Data & Serialization:**
- `serde` 1.0 + `serde_json` 1.0 - Serialization framework (Rust)
- `pydantic` 2.11.5 - Python data validation and serialization
- `pandas` 2.2.3 - Data manipulation (backend meeting aggregation)

**Utilities:**
- `zod` 3.25.71 - TypeScript schema validation
- `date-fns` 4.1.0 - Date/time manipulation
- `lodash` 4.17.21 - Utility functions
- `clsx` 2.1.1 - CSS class name utility
- `uuid` 1.0 - UUID generation for resource IDs
- `chrono` 0.4.31 - Rust date/time handling
- `dotenv` (Python) - Environment variable loading
- `python-dotenv` 1.1.0 - Python .env file support

## Configuration

**Environment Variables:**
Frontend (`frontend/.env.example`):
- `TAURI_SIGNING_PRIVATE_KEY` - Code signing for desktop updates
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - Private key passphrase

Backend (`backend/`):
- `ANTHROPIC_API_KEY` - Claude API authentication
- `GROQ_API_KEY` - Groq model API key (optional)
- `OPENAI_API_KEY` - OpenAI API authentication
- `OLLAMA_HOST` - Ollama server URL (default: http://localhost:11434)
- `DATABASE_PATH` - SQLite database file location (default: meeting_minutes.db)
- `DB_PATH` - Alternative database path configuration
- `HOST` - FastAPI server host (default: 0.0.0.0)
- `PORT` - FastAPI server port (default: 5167)
- `CHUNK_SIZE` - Transcript processing chunk size (default: 5000)
- `CHUNK_OVERLAP` - Overlap between chunks for context (default: 1000)

**Build Configuration:**
- `frontend/tauri.conf.json` - Tauri app configuration, CSP, updater, bundle settings
- `frontend/tsconfig.json` - TypeScript compiler options with Next.js plugin
- `frontend/src-tauri/Cargo.toml` - Rust dependencies with platform-specific features
- `Cargo.toml` - Workspace configuration for Rust project

**Development Configuration:**
- `frontend/src-tauri/.cargo/config` - Rust build profiles
- `frontend/src-tauri/Info.plist` - macOS app metadata and permissions
- `frontend/src-tauri/entitlements.plist` - macOS sandboxing and capabilities

## Platform Requirements

**Development:**
- macOS: Xcode (for Metal GPU, CoreML acceleration), ScreenCaptureKit capability
- Windows: Visual Studio Build Tools with C++ workload, WASAPI audio stack
- Linux: cmake, llvm, libomp, ALSA/PulseAudio

**Production:**
- Desktop app: Cross-platform binary (macOS universal, Windows x64, Linux x64/ARM)
- Backend API: Can run standalone or via Docker
- Whisper models: Stored in platform-specific app data directories or backend

**Hardware Acceleration (GPU):**
- macOS: Metal (automatic) + CoreML (optional)
- Windows: CUDA (NVIDIA, optional) or Vulkan (AMD/Intel, optional)
- Linux: CUDA (NVIDIA, optional), Vulkan, or HIP/ROCm (AMD)
- CPU: OpenBLAS (Windows/Linux) or native scalar (fallback)

**External Services:**
- Ollama server (http://localhost:11434) - For local LLM inference
- Claude API (claude.anthropic.com) - Paid LLM integration
- Groq API - Inference as a service provider
- OpenAI API - GPT models for summarization
- OpenRouter API - LLM provider aggregation

---

*Stack analysis: 2026-02-01*
