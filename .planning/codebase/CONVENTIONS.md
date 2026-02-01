# Coding Conventions

**Analysis Date:** 2026-02-01

## Naming Patterns

**Files:**
- TypeScript/React: camelCase with descriptive names (e.g., `usePermissionCheck.ts`, `RecordingStateContext.tsx`)
- Rust: snake_case (e.g., `recording_manager.rs`, `audio_capture.rs`)
- Python: snake_case (e.g., `database_manager.py`, `transcript_processor.py`)
- Hooks: prefix with `use` followed by camelCase (e.g., `useRecordingStop.ts`, `useModalState.ts`)
- Services: suffix with `Service` (e.g., `recordingService`, `transcriptService`, `indexedDBService`)
- Contexts: suffix with `Context` (e.g., `RecordingStateContext`, `TranscriptContext`, `ConfigContext`)

**Functions:**
- TypeScript: camelCase (e.g., `checkPermissions()`, `syncWithBackend()`, `handleRecordingStop()`)
- Rust: snake_case (e.g., `start_recording()`, `list_audio_devices()`, `get_current_backend()`)
- Async functions: no special prefix, follow same pattern (e.g., `async fn start_recording()`)
- Callback handlers: prefix with `handle` (e.g., `handleRecordingToggle()`, `handleRecordingStart()`)

**Variables:**
- TypeScript: camelCase (e.g., `isRecording`, `meetingTitle`, `transcriptsRef`)
- State variables: clear intent (e.g., `status`, `isProcessing`, `isStopping`)
- Boolean prefixes: `is`, `has`, `can`, `should` (e.g., `isRecording`, `hasMicrophone`, `canTranscribe`)
- Rust: snake_case (e.g., `is_recording`, `audio_sender`, `stream_manager`)

**Types:**
- TypeScript interfaces: PascalCase with suffix (e.g., `RecordingState`, `PermissionStatus`, `SidebarContextType`)
- TypeScript enums: PascalCase (e.g., `RecordingStatus`, `SummaryStatus`)
- Rust structs: PascalCase (e.g., `RecordingManager`, `AudioDevice`, `AudioCaptureBackend`)
- Union types/type aliases: PascalCase (e.g., `SummaryStatus = 'idle' | 'processing' | 'completed'`)

**Constants:**
- TypeScript: UPPER_SNAKE_CASE for true constants, camelCase for config objects
- Rust: UPPER_SNAKE_CASE for static values (e.g., `RECORDING_FLAG`)
- Environment prefixes: descriptive with env var clarity (e.g., `DATABASE_PATH`, `RUST_LOG`)

## Code Style

**Formatting:**
- TypeScript/JavaScript: Enforced by ESLint config extending Next.js core-web-vitals and TypeScript rules
  - Config: `frontend/eslint.config.mjs`
  - Extends: `next/core-web-vitals` and `next/typescript`
  - No explicit Prettier config detected - follows Next.js defaults
  - Line length: Standard (implied ~100-120 chars based on codebase)
- Rust: Standard Rust conventions via `rustfmt`
  - Edition: 2021 (`Cargo.toml` edition field)
  - Uses standard formatting with no custom `rustfmt.toml` overrides observed
- Python: PEP 8 conventions (inferred from codebase structure)

**Linting:**
- TypeScript: ESLint with Next.js rules (`eslint.config.mjs`)
  - Run: `pnpm lint` (from `package.json`)
  - Enforces TypeScript strict checking
  - Web vitals optimizations
- Rust: `cargo check` (implicit via build process)
- Python: No explicit linter configured, relies on PEP 8 adherence

**Indentation:**
- TypeScript/JavaScript: 2 spaces (observed throughout codebase)
- Rust: 4 spaces (standard Rust convention)
- Python: 4 spaces (standard Python convention)

## Import Organization

**Order (TypeScript/React):**
1. React and third-party framework imports (e.g., `import { useState } from 'react'`)
2. Third-party library imports (e.g., `import { motion } from 'framer-motion'`)
3. Tauri API imports (e.g., `import { invoke } from '@tauri-apps/api/core'`)
4. Application component imports (e.g., `import { RecordingControls } from '@/components/RecordingControls'`)
5. Application hook imports (e.g., `import { usePermissionCheck } from '@/hooks/usePermissionCheck'`)
6. Application context imports (e.g., `import { useRecordingState } from '@/contexts/RecordingStateContext'`)
7. Service imports (e.g., `import { recordingService } from '@/services/recordingService'`)
8. Utility/library imports (e.g., `import Analytics from '@/lib/analytics'`)
9. External library utilities at end (e.g., `import { toast } from 'sonner'`)

Example from `frontend/src/app/page.tsx`:
```typescript
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { RecordingControls } from '@/components/RecordingControls';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { useRecordingState, RecordingStatus } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { StatusOverlays } from '@/app/_components/StatusOverlays';
import Analytics from '@/lib/analytics';
import { indexedDBService } from '@/services/indexedDBService';
import { toast } from 'sonner';
import { useRouter } from 'next/navigation';
```

**Order (Rust):**
1. Standard library imports (`use std::`)
2. Third-party crate imports (`use serde::`, `use log::`)
3. Module imports from same crate (`use super::`, `use crate::`)
4. Re-exports and macros

Example from `frontend/src-tauri/src/lib.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use log::{error as log_error, info as log_info};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;
use audio::{list_audio_devices, AudioDevice, trigger_audio_permission};
```

**Path Aliases:**
- TypeScript: `@/` resolves to `frontend/src/` (defined in `tsconfig.json`)
  - `@/components/` → components
  - `@/hooks/` → hooks
  - `@/contexts/` → contexts
  - `@/services/` → services
  - `@/lib/` → lib utilities
  - `@/app/` → app routes

**Order (Python):**
1. Standard library imports
2. Third-party imports (FastAPI, aiosqlite, etc.)
3. Local application imports

Example from `backend/app/main.py`:
```python
from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import logging
from dotenv import load_dotenv
from db import DatabaseManager
```

## Error Handling

**TypeScript/React Patterns:**
- Try-catch blocks for async operations with descriptive error messages
- Instance checks for error objects: `error instanceof Error ? error.message : 'Unknown error'`
- Toast notifications for user-facing errors via `sonner` library: `toast.error('message')`
- Console logging for debugging: `console.error()`, `console.warn()`, `console.log()`
- Guard refs to prevent concurrent operations: `stopInProgressRef.current`

Example from `frontend/src/hooks/useRecordingStop.ts`:
```typescript
try {
  // Operation
} catch (error) {
  console.error('Failed to request permissions:', error);
  setStatus({
    // ... error state
  });
}
```

**Rust Patterns:**
- `anyhow::Result<T>` for error propagation with context
- `map_err()` to add context to errors
- Return `Err(anyhow::anyhow!())` for new errors with custom messages
- Logging at appropriate levels: `log_info!()`, `log_error!()`, `warn!()`, `debug!()`
- Performance macros: `perf_debug!()`, `perf_trace!()` (zero-cost in release builds)

Example from `frontend/src-tauri/src/audio/recording_manager.rs`:
```rust
pub async fn start_recording(
    &mut self,
    microphone_device: Option<Arc<AudioDevice>>,
    system_device: Option<Arc<AudioDevice>>,
    auto_save: bool,
) -> Result<mpsc::UnboundedReceiver<AudioChunk>> {
    info!("Starting recording manager (auto_save: {})", auto_save);
    // ...
    match audio::recording_commands::start_recording_with_devices_and_meeting(/* ... */) {
        Ok(_) => {
            log_info!("Recording started successfully");
            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start audio recording: {}", e);
            Err(format!("Failed to start recording: {}", e))
        }
    }
}
```

**Python Patterns:**
- Try-except blocks with exception type handling
- Logging with context: `logger.error(f"message: {str(e)}", exc_info=True)`
- HTTP exceptions for API errors: `HTTPException(status_code=400, detail="message")`
- Database errors logged with full traceback for debugging

Example from `backend/app/main.py`:
```python
try:
    # Operation
except Exception as e:
    logger.error(f"Failed to initialize: {str(e)}", exc_info=True)
    raise
```

## Logging

**Framework:**
- TypeScript: `console` for browser logging (no centralized logger configured)
  - `console.log()` for informational messages
  - `console.error()` for errors
  - `console.warn()` for warnings
- Rust: `log` crate with `env_logger`
  - Configured via `RUST_LOG` environment variable
  - Example: `RUST_LOG=debug ./clean_run.sh` (from CLAUDE.md)
  - Log levels: `log_info!()`, `log_error!()`, `warn!()`, `debug!()`, `perf_debug!()` (macro)
- Python: `logging` module with detailed formatting
  - Config: `backend/app/main.py` lines 18-35
  - Format includes timestamp, level, filename:lineno, function name
  - Example: `2025-01-03 12:34:56 - INFO - [main.py:123 - endpoint_name()] - Message`

**Patterns:**
- TypeScript: Inline descriptive messages during development
- Rust: Info logging for major operations, debug for detailed tracing, error for failures
  - Performance-critical code uses `perf_debug!()` and `perf_trace!()` (eliminated in release)
- Python: Consistent formatting with function context for debugging

**When to Log:**
- TypeScript: At entry/exit of async operations, state changes, errors
- Rust: At function entry with parameters, at error conditions, at state transitions
- Python: At API endpoint entry, database operations, errors with full stack trace

## Comments

**When to Comment:**
- Rust: Complex algorithms, platform-specific code, non-obvious design decisions
  - Module-level doc comments with `///` for public APIs
  - Inline comments starting with `//` for clarification
  - See examples: `frontend/src-tauri/src/audio/recording_manager.rs` lines 58-68 (function doc)

**JSDoc/TSDoc:**
- TypeScript: Full JSDoc comments for complex hooks and public functions
  - Describes parameters, return type, and behavior
  - Example from `frontend/src/hooks/useRecordingStop.ts` lines 23-35:
    ```typescript
    /**
     * Custom hook for managing recording stop lifecycle.
     * Handles the complex stop sequence: transcription wait → buffer flush → SQLite save → navigation.
     *
     * Features:
     * - Transcription completion polling
     * - Transcript buffer flush coordination
     * - SQLite meeting save
     * - Comprehensive analytics tracking
     * - Auto-navigation to meeting details
     * - Toast notifications for success/error
     * - Window exposure for Rust callbacks
     */
    ```
- Rust: Doc comments with `///` for public items, inline comments with `//`
  - Example: `frontend/src-tauri/src/audio/recording_manager.rs` line 58-68
    ```rust
    /// Start recording with specified devices
    ///
    /// # Arguments
    /// * `microphone_device` - Optional microphone device to use
    /// * `system_device` - Optional system audio device to use
    /// * `auto_save` - Whether to save audio checkpoints
    ```

**Style Guidelines:**
- Comments explain "why" not "what" (code shows what)
- Avoid obvious comments: `let x = 5; // Set x to 5` (unnecessary)
- Update comments when code changes
- Use TODO/FIXME for known issues (see CONCERNS.md for tracker)

## Function Design

**Size Guidelines:**
- TypeScript: 50-150 lines for hooks, 100-200 for complex functions
  - Example: `useRecordingStop()` ~350 lines (complex lifecycle hook with multiple phases)
  - Example: `usePermissionCheck()` ~87 lines (simple utility hook)
- Rust: 50-100 lines for most functions, 100+ for complex orchestration
  - Example: `start_recording()` in recording_manager ~140 lines (orchestrates multiple systems)
- Python: 50-100 lines typical, up to 200 for endpoint handlers

**Parameters:**
- TypeScript: Extract related parameters into config objects for functions with >4 params
  - Example: `useRecordingStop(setIsRecording, setIsRecordingDisabled)` (2 explicit, state via context)
  - Use destructuring: `const { hasMicrophone, isChecking } = usePermissionCheck()`
- Rust: Similar approach, use structs for complex parameter sets
  - Example: `start_recording(&mut self, microphone_device, system_device, auto_save)`
- Callbacks: Use single parameter for event data or config object

**Return Values:**
- TypeScript hooks: Return object with state + methods
  - Example from `usePermissionCheck()`:
    ```typescript
    return {
      ...status,
      checkPermissions,
      requestPermissions,
    };
    ```
- Rust functions: Return `Result<T>` for fallible operations, `T` for infallible
- Keep return types consistent within a module

## Module Design

**Exports:**
- TypeScript: Named exports for components, hooks, utils
  - Example: `export function usePermissionCheck()`
  - Default exports for pages/layouts
  - Type exports: `export interface PermissionStatus { ... }`
- Rust: Public items with `pub`, private by default
  - Module exports at end of file or via `mod.rs`
  - Example: `pub async fn start_recording() -> Result<T>`
- Python: Module-level functions and classes are implicitly public

**Barrel Files:**
- Not extensively used in this codebase
- Each module/directory exports its own items
- No index files consolidating exports (e.g., no `components/index.ts`)
- Direct imports preferred: `import { Recording Controls } from '@/components/RecordingControls'`

**File Organization Within Modules:**
- Rust: Type definitions → Struct/impl → Public functions → Helper functions → Tests
  - Example from `backend_config.rs`: Enum def (lines 8-84) → impl (85-130) → functions (137-150) → tests (152+)
- TypeScript: Interfaces → Component/hook def → Helper functions
  - Example from `page.tsx`: Imports → State setup → Effects → JSX
- Python: Imports → Class/function defs → Main logic

## State Management Patterns

**TypeScript/React:**
- Contexts for global state: `RecordingStateContext`, `TranscriptContext`, `ConfigContext`
- Hooks for derived/complex state: `useRecordingStop`, `useModalState`
- Local component state for UI-only concerns (animations, modals)
- Refs for mutable values that don't trigger renders: `useRef(null)`

Example from `RecordingStateContext.tsx`:
```typescript
const [state, setState] = useState<RecordingState>({
  isRecording: false,
  isPaused: false,
  isActive: false,
  // ...
});
```

**Rust:**
- `Arc<RwLock<T>>` for shared mutable state across async tasks
- `Arc<AtomicBool>` for simple boolean flags
- Immutable owned values where possible

Example from `lib.rs`:
```rust
static RECORDING_FLAG: AtomicBool = AtomicBool::new(false);
pub struct RecordingState {
    is_recording: Arc<AtomicBool>,
    audio_sender: Arc<RwLock<Option<mpsc::UnboundedSender<AudioChunk>>>>,
}
```

## Performance Considerations

**Hot Path Logging:**
- Rust: Use `perf_debug!()` and `perf_trace!()` macros that are eliminated in release builds
- TypeScript: Avoid excessive logging in rendering/event handlers
- Python: No equivalent pattern; use standard logging for backend

**Async Coordination:**
- TypeScript: Use `useCallback` for memoized callbacks in hooks
- Rust: Use `tokio::sync` primitives (channels, RwLock) for thread-safe coordination
- Prevent duplicate operations with guard refs: `stopInProgressRef.current`

---

*Convention analysis: 2026-02-01*
