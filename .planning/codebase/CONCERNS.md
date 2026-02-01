# Codebase Concerns

**Analysis Date:** 2026-02-01

## Tech Debt

### Audio System Duplication & Incomplete Migration
- **Issue:** Two parallel audio systems (`audio/` and `audio_v2/`) exist simultaneously
- **Files:** `frontend/src-tauri/src/audio/`, `frontend/src-tauri/src/audio_v2/`
- **Impact:**
  - Code maintainability reduced (unclear which system is active)
  - Duplicate functionality increases cognitive load for developers
  - Future modifications must update both systems or risk inconsistency
- **Fix approach:**
  1. Determine if `audio_v2` is active replacement or abandoned experiment
  2. Either complete `audio_v2` migration fully OR remove it entirely
  3. Consolidate functionality into single audio architecture
  4. See CLEANUP_PLAN.md Phase 1 for detailed removal strategy

### Legacy Code File Still in Active Repository
- **Issue:** `lib_old_complex.rs` (2,437 lines) remains in codebase
- **Files:** `frontend/src-tauri/src/lib_old_complex.rs`
- **Impact:**
  - Increases compilation time (parsed but unused)
  - Creates confusion about "current" implementation
  - Risk of regression if old patterns accidentally reintroduced
  - Comments indicate it was original monolithic implementation before refactoring
- **Fix approach:**
  1. Verify no active references to this file in current code
  2. Create branch backup before deletion
  3. Remove file and test functionality fully
  4. Commit removal with clear message explaining what was replaced

### Incomplete Placeholder Implementations in audio_v2
- **Issue:** Audio_v2 modules contain extensive TODO comments with "Phase 2/3/4" stubs
- **Files:**
  - `frontend/src-tauri/src/audio_v2/lib.rs` (lines 94, 101, 108)
  - `frontend/src-tauri/src/audio_v2/sync.rs` (lines 19, 31)
  - `frontend/src-tauri/src/audio_v2/normalizer.rs` (lines 11, 26)
  - `frontend/src-tauri/src/audio_v2/resampler.rs` (lines 10, 22, 27)
  - `frontend/src-tauri/src/audio_v2/recorder.rs` (lines 139, 163)
  - `frontend/src-tauri/src/audio_v2/limiter.rs` (lines 8, 20)
  - `frontend/src-tauri/src/audio_v2/compatibility.rs` (lines 86, 177)
- **Impact:**
  - Significant code exists with no functionality (returns Ok(()) with no actual work)
  - Creates false impression of feature completeness
  - Safety risk if code is accidentally triggered in production
- **Fix approach:**
  - Remove entire audio_v2 module if not actively being completed
  - If completing, convert TODOs to proper GitHub issues with design docs
  - Remove stub implementations with placeholder return values

## Known Bugs

### System Audio Buffer Overflow Causes Distortion
- **Symptoms:** System audio (especially in recorded output) exhibits crackling, artifacts, or complete loss of system audio in final recording
- **Files:** `frontend/src-tauri/src/audio/pipeline.rs` (lines 72-76)
- **Trigger:** During recording on macOS with heavy system audio (e.g., multiple browser tabs with video), buffer overflows trigger sample drops
- **Current mitigation:**
  - Buffer increased to 400ms (from 200ms) per line 35 comment
  - Error logging added to indicate overflow occurred
  - Samples are dropped when max buffer exceeded
- **Root cause:** System audio arrives asynchronously at variable rates. Core Audio → RNNoise buffering → channel transmission adds unpredictable latency. Pipeline tries to keep 50ms mixing windows synchronized but timing jitter causes buffer buildup.
- **Partial workaround in code:** Ring buffer attempts synchronization but this is insufficient under load

### Microphone Buffer Overflow During High-Latency System Audio Periods
- **Symptoms:** Microphone audio preserved but system audio gaps or disappears
- **Files:** `frontend/src-tauri/src/audio/pipeline.rs` (lines 67-71)
- **Trigger:** When system audio delivery stalls, microphone continues accumulating samples faster than mixing occurs, causing overflow
- **Current mitigation:** Overflow warning logged but samples still dropped
- **Impact:** Audio chunks missing from final recording, especially in system audio stream

### Unsafe Static Mutable State in Legacy Code
- **Symptoms:** Data races possible (though unlikely to manifest with current usage patterns)
- **Files:** `frontend/src-tauri/src/lib_old_complex.rs` (lines 38-50)
- **Unsafe variables:**
  - `MIC_BUFFER`, `SYSTEM_BUFFER`, `AUDIO_CHUNK_QUEUE`, `MIC_STREAM`, `SYSTEM_STREAM`
  - `IS_RUNNING`, `RECORDING_START_TIME`, `TRANSCRIPTION_TASK`, `AUDIO_COLLECTION_TASK`
  - `ANALYTICS_CLIENT`, `ERROR_EVENT_EMITTED`, `WHISPER_ENGINE`
  - Additional `MIC_FAILURE_COUNT` and `LAST_MIC_RECOVERY_ATTEMPT` counters
- **Trigger:** Would require concurrent modification attempts from multiple threads
- **Current mitigation:**
  - Most are wrapped with `Arc<Mutex<T>>` internally, providing safety
  - Tauri command execution is serialized, reducing concurrent access risk
- **Risk:** Medium-Low for current usage, but violates Rust safety principles

## Security Considerations

### CORS Allows All Origins in Backend API
- **Risk:** Backend API accepts requests from any origin, bypassing browser same-origin policy protections
- **Files:** `backend/app/main.py` (line 46: `allow_origins=["*"]`)
- **Current mitigation:** Documented as "for testing" only; no production safeguard in place
- **Recommendations:**
  1. Restrict `allow_origins` to `["http://localhost:3118", "http://localhost:5167"]` in development
  2. For production builds, restrict to registered frontend origin only
  3. Add environment variable `ALLOWED_ORIGINS` with safe defaults
  4. Document this is a development configuration that must change for production

### API Keys Stored in Frontend Settings Without Encryption
- **Risk:** OpenRouter, Ollama, and custom LLM provider API keys stored in plaintext in frontend database
- **Files:**
  - `frontend/src-tauri/src/database/` (settings storage)
  - `backend/app/main.py` - API endpoints accept and store keys
- **Trigger:** User enters API key in settings, it's persisted to SQLite unencrypted
- **Current mitigation:** None - keys are stored as plain text
- **Recommendations:**
  1. Implement encryption for sensitive settings (use `tauri-plugin-store` with encryption option if available)
  2. Consider storing keys in OS credential manager (Keychain on macOS, Credential Manager on Windows)
  3. At minimum, document the security implication in UI and README
  4. Add warning when user enters API key about local-only storage

### Database Migration Uses Silent Error Suppression
- **Risk:** Failed `ALTER TABLE` operations are silently ignored, potentially leaving schema inconsistent
- **Files:** `backend/app/db.py` (lines 63-66, 87-97)
- **Pattern:** `try: ALTER TABLE ... except: pass`
- **Trigger:** If migration partially succeeds or fails unexpectedly, subsequent schema operations may fail silently
- **Current mitigation:** SchemaValidator attempted but may not catch all inconsistencies
- **Recommendations:**
  1. Log all migration attempts (success and failure)
  2. Track schema version explicitly to know which migrations have run
  3. Fail loudly if expected columns are missing after migration
  4. Add database integrity check on startup

### Environment Variable Exposure in Debug Builds
- **Risk:** RUST_LOG and other sensitive environment variables visible in logs
- **Files:** Multiple backend scripts (backend/clean_start_backend.sh, backend/build-docker.sh, etc.)
- **Current mitigation:** Documentation shows optional DEBUG flags but no protection against accidental exposure
- **Recommendations:**
  1. Add .env to .gitignore verification
  2. Implement log filtering to redact API keys if accidentally logged
  3. Add pre-commit hook to prevent committing .env files

## Performance Bottlenecks

### Large Monolithic Files Slow Compilation
- **Problem:** Multiple files exceed 1000+ lines, creating compilation bottlenecks
- **Files with size concern:**
  - `frontend/src-tauri/src/lib_old_complex.rs` - 2,437 lines (legacy, should be deleted)
  - `frontend/src-tauri/src/api/api.rs` - 1,381 lines
  - `frontend/src-tauri/src/audio/recording_commands.rs` - 1,212 lines
  - `frontend/src-tauri/src/whisper_engine/whisper_engine.rs` - 1,150 lines
  - `frontend/src-tauri/src/parakeet_engine/parakeet_engine.rs` - 1,088 lines
  - `frontend/src-tauri/src/audio/pipeline.rs` - 1,079 lines
- **Impact:**
  - Incremental compilation slower (more code to re-analyze on changes)
  - Harder to find specific functionality within file
  - Increased risk of unintended side effects from changes
- **Improvement path:**
  1. Extract logical submodules from api.rs (separate endpoint groups)
  2. Split recording_commands.rs into device_management, state_management, error_handling submodules
  3. Move whisper model management into separate module from engine.rs
  4. Extract pipeline mixing logic into separate buffer and mixer modules

### Whisper Model Download and GPU Memory Stalls UI
- **Problem:** Model downloading and loading blocks UI thread waiting for response
- **Files:** `frontend/src-tauri/src/whisper_engine/whisper_engine.rs` (model loading)
- **Cause:** Model files are large (1GB+), GPU vram initialization blocks UI during startup
- **Symptoms:** UI becomes unresponsive during model load (can take 30+ seconds on first load)
- **Improvement path:**
  1. Implement progress callback during model load (currently only during download)
  2. Add background loading to avoid blocking UI
  3. Consider lazy-loading model only when recording starts, not on app startup

### Transcription Processing Locks CPU During Heavy Batch Operations
- **Problem:** `whisper_engine/parallel_processor.rs` and `parakeet_engine` serialize long batches of chunks
- **Files:**
  - `frontend/src-tauri/src/whisper_engine/parallel_processor.rs` (480 lines)
  - `frontend/src-tauri/src/parakeet_engine/parakeet_engine.rs` (1,088 lines)
- **Impact:** During processing of 1000+ chunk batches, CPU usage maxes out, UI responsiveness degrades
- **Improvement path:**
  1. Implement chunk processing batching (process in smaller groups, yield to other tasks)
  2. Add CPU usage monitoring and throttling
  3. Consider moving CPU-heavy transcription to worker thread

### SQLite Database Lacks Indexing on Common Queries
- **Problem:** No explicit indexes created on frequently queried fields
- **Files:** `backend/app/db.py` (table creation, no CREATE INDEX statements visible)
- **Trigger:** Performance degrades as meeting count grows (100+)
- **Improvement path:**
  1. Add indexes on `meeting_id` in transcripts table
  2. Add indexes on `created_at` for date-based queries
  3. Add composite indexes for common WHERE + ORDER BY patterns
  4. Measure query performance before/after indexing

## Fragile Areas

### Audio Pipeline Timing Assumptions Brittle on Variable System Load
- **Files:** `frontend/src-tauri/src/audio/pipeline.rs`
- **Why fragile:**
  - 50ms mixing window size assumes consistent audio arrival rates
  - System load (GC pauses, CPU throttling) causes timing jitter
  - 400ms buffer is pragmatic but arbitrary limit - may still overflow under extreme load
  - Comments indicate this was discovered through painful debugging (line 31-34 "CRITICAL FIX")
- **Safe modification:**
  1. Test changes with system under load (e.g., video conferencing + recording + local Whisper)
  2. Measure buffer fill levels before/after changes
  3. Add extensive logging of timing metrics during modification
  4. Never remove buffer overflow checks
  5. Validate on macOS (Core Audio most problematic), Windows, and Linux

### Transcription Model Switching Without Proper Cleanup
- **Files:** `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`
- **Why fragile:** Switching Whisper models requires unloading old context to free GPU memory
- **Potential issue:** If user switches models while recording/transcribing, old model may remain in GPU memory
- **Safe modification:**
  1. Verify recording stops before allowing model switch
  2. Ensure `current_context` is properly dropped and GPU memory released
  3. Test on low-VRAM systems (2GB GPU) to catch memory leaks
  4. Add GPU memory usage monitoring

### Device Detection on macOS Uses ScreenCaptureKit Which Can Permission-Fail Silently
- **Files:** `frontend/src-tauri/src/audio/devices/platform/macos.rs` (or core_audio.rs)
- **Why fragile:**
  - ScreenCaptureKit requires screen recording permission (separate from microphone)
  - Permission denial returns different error on each macOS version
  - Fallback mechanisms may silently succeed with empty device list
- **Safe modification:**
  1. Test permission denial scenarios (revoke permission via System Preferences)
  2. Verify user gets explicit error message, not silent failure
  3. Add logs for each permission check step
  4. Test on macOS Sonoma, Ventura, Monterey

### Recording File Path Assumptions About Filesystem
- **Files:** `frontend/src-tauri/src/audio/recording_saver.rs`
- **Why fragile:**
  - Path building may assume Unix-style paths on Windows
  - Directory creation order matters (parent must exist)
  - No validation that directory is writable before attempting save
- **Safe modification:**
  1. Use `std::fs::create_dir_all()` to ensure parent directories exist
  2. Test on both Unix and Windows paths with special characters
  3. Verify write permissions before attempting save
  4. Handle out-of-disk-space errors explicitly

### Test Coverage Gaps

**Untested area:** Audio mixing and pipeline synchronization
- **What's not tested:** End-to-end audio mixing with realistic timing jitter
- **Files:** `frontend/src-tauri/src/audio/pipeline.rs` (entire 1079-line file)
- **Risk:** Buffer overflow bugs only discovered through user reports, not in development
- **Priority:** High - this is critical path code handling real-time audio

**Untested area:** Transcription error recovery
- **What's not tested:** Whisper model crashes, out-of-memory conditions, recovery from partial transcription
- **Files:** `frontend/src-tauri/src/audio/transcription/worker.rs`
- **Risk:** App crashes instead of graceful error recovery in edge cases
- **Priority:** High - user data loss if recovery fails

**Untested area:** Database migration edge cases
- **What's not tested:** Partial migration states, corruption detection, schema version mismatch recovery
- **Files:** `backend/app/db.py`
- **Risk:** Data loss if migration partially succeeds
- **Priority:** Medium - mostly affects existing installs with old database versions

**Untested area:** CORS and security boundaries
- **What's not tested:** Cross-origin requests, API key handling, session/auth persistence
- **Files:** `backend/app/main.py` (CORS middleware, authentication endpoints)
- **Risk:** Security vulnerabilities only caught in production
- **Priority:** Medium - affects production deployment security

**Untested area:** Parakeet ONNX model loading on unsupported hardware
- **What's not tested:** Graceful fallback when ONNX runtime unavailable
- **Files:** `frontend/src-tauri/src/parakeet_engine/parakeet_engine.rs`
- **Risk:** App crash instead of fallback to Whisper
- **Priority:** Medium - impacts systems without full ONNX support

## Scaling Limits

### Whisper Model GPU Memory Consumption
- **Current capacity:** Tested with 4GB+ dedicated GPU VRAM (RTX 3060+), fallback to CPU on smaller systems
- **Limit:**
  - Large models (1.5GB+) won't load on systems with <2GB dedicated VRAM
  - CPU fallback is 10-100x slower depending on model size
  - Parallel processing mode multiplies memory usage proportionally
- **Scaling path:**
  1. Implement model quantization (FP16 instead of FP32) to reduce VRAM by ~50%
  2. Implement streaming transcription (process audio without loading entire model)
  3. Consider external transcription service API fallback for users with low VRAM
  4. Add VRAM detection and automatic model selection

### Database File Size for Long Recording Sessions
- **Current capacity:** SQLite suitable for 100k+ transcript chunks (~1GB database file)
- **Limit:** Beyond 10GB, query performance degrades significantly without proper indexing
- **Scaling path:**
  1. Add database indexing (see Performance Bottlenecks section)
  2. Implement data archival (move old meetings to separate archive database)
  3. Consider migration to more robust database (PostgreSQL) for multi-user deployment
  4. Implement periodic database cleanup (remove incomplete recording sessions)

### Concurrent Recording Sessions
- **Current capacity:** Single recording session supported (mutual exclusion enforced)
- **Limit:** Cannot start second recording without stopping first (by design)
- **Scaling path:**
  1. If concurrent sessions needed, redesign state management from static mutexes to session-based state
  2. Implement separate recording pipeline per session
  3. Add session multiplexing to audio device management
  4. Track per-session transcription and file paths independently

### API Request Rate Limiting
- **Current capacity:** No rate limiting implemented in FastAPI
- **Limit:** Single slow client can stall entire API for others
- **Scaling path:**
  1. Implement request rate limiting (FastAPI SlowAPI middleware)
  2. Add timeout for long-running operations (model transcription requests)
  3. Implement request queuing with priority (user requests > background tasks)
  4. Add circuit breaker for external LLM service calls

## Dependencies at Risk

### whisper-rs Git Dependency on Custom Branch
- **Risk:** Dependence on uncommitted changes in specific git revision may become unavailable
- **Impact:** Cannot build without GitHub connectivity if revision hash is lost
- **Files:** `frontend/src-tauri/Cargo.toml` (check dependencies section for git URLs)
- **Migration plan:**
  1. Audit which features from git version are essential vs. backport-able
  2. Submit upstream PRs to official whisper-rs for critical features
  3. If awaiting merge, pin to known-good release once features land
  4. Document specific features required from custom branch

### Parakeet ONNX Runtime (ort v2.0.0-rc10)
- **Risk:** Release candidate version, API stability not guaranteed
- **Files:** `frontend/src-tauri/Cargo.toml` (ort = "2.0.0-rc.10")
- **Impact:** Breaking changes possible in future ONNX Runtime updates
- **Migration plan:**
  1. Monitor ort crate releases for stable v2.0.0 release
  2. Pin to stable once available
  3. Evaluate migration path if v2.0.0 introduces breaking changes
  4. Test Parakeet functionality thoroughly after ort updates

### Silero.rs Git Dependency
- **Risk:** Depends on forked community implementation at specific git revision
- **Files:** `frontend/src-tauri/Cargo.toml` (silero_rs git dependency)
- **Migration plan:**
  1. Check if original silero-rs crate is actively maintained
  2. If yes, migrate back to official crate when possible
  3. Document why fork is necessary
  4. Monitor fork for compatibility issues with newer Rust versions

### Nnnoiseless (Noise Suppression)
- **Risk:** Single-maintainer community project, no activity guarantees
- **Files:** `frontend/src-tauri/Cargo.toml` (nnnoiseless = "0.5")
- **Impact:** If unmaintained, won't receive security updates or compatibility fixes for new Rust versions
- **Alternative:** No direct alternative exists; loss of real-time noise suppression if this breaks
- **Mitigation:**
  1. Test nnnoiseless with new Rust versions periodically
  2. Keep fallback path if module fails to load (process without noise suppression)
  3. Monitor maintenance status and consider forking if necessary

## Missing Critical Features

### Audio Device Hotplug Handling
- **Problem:** If user plugs in USB microphone or changes default device during recording, app doesn't detect change
- **Blocks:** Recording on dynamically-switched devices, user experience degrades
- **Current state:** Device list updates, but active recording continues on old device

### Meeting Persistence Across Sessions
- **Problem:** Frontend stores meetings locally in SQLite, but sharing/sync not implemented
- **Blocks:** Users with multiple devices cannot share meetings or sync recording history
- **Current state:** Each installation has separate database

### LLM Summarization Streaming Response
- **Problem:** Summary generation returns whole response at once after waiting, no progress indication
- **Blocks:** User feels app is frozen during 30+ second summary generation
- **Current state:** Summarization happens in background task with single completion event

### Batch Operations on Meetings
- **Problem:** Cannot bulk-delete, bulk-export, or bulk-process multiple meetings
- **Blocks:** Users with 100+ meetings cannot efficiently manage collection
- **Current state:** Only single-meeting CRUD operations available

### Structured Export Formats
- **Problem:** Can export transcripts to text, but not to common formats (PDF, Markdown with metadata, JSON-LD for archival)
- **Blocks:** Integration with note-taking systems, archival systems
- **Current state:** Plain text export only

## Summary of Critical Action Items

**Immediate (High Priority):**
1. Delete or complete `audio_v2` module to eliminate duplication
2. Remove `lib_old_complex.rs` legacy file
3. Restrict CORS configuration for production deployments
4. Document database security implications (unencrypted API keys)

**Short-term (Medium Priority):**
1. Add proper error handling for audio buffer overflows (not just dropping samples)
2. Implement encryption for stored API keys or migrate to OS credential store
3. Add comprehensive audio pipeline tests (especially timing under load)
4. Split large monolithic files (api.rs, recording_commands.rs, etc.) into submodules

**Long-term (Lower Priority):**
1. Migrate Parakeet to stable ort release
2. Implement model quantization for GPU memory efficiency
3. Add database indexing for performance at scale
4. Implement request rate limiting in FastAPI backend
5. Add audio device hotplug detection and reconnection

---

*Concerns audit: 2026-02-01*
