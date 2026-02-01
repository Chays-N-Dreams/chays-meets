# Testing Patterns

**Analysis Date:** 2026-02-01

## Test Framework

**Rust:**
- Runner: Built-in Rust test framework (no external runner)
- Config: Implicit via `Cargo.toml` dev-dependencies
- Dev Dependencies: `tempfile`, `criterion` (benchmarking), `tracing-subscriber`
- Run Commands:
  ```bash
  cargo test                    # Run all unit tests
  cargo test --release          # Run tests optimized
  cargo test -- --nocapture     # Show println! output
  cargo test audio::            # Run tests in audio module
  cargo bench                    # Run benchmarks with criterion
  ```

**TypeScript/JavaScript:**
- Runner: Not detected (no Jest, Vitest, or similar configured)
- Status: No test framework configured for frontend
- Note: Unit tests not found despite `package.json` having development infrastructure

**Python:**
- Runner: Not detected (no pytest or unittest config found)
- Status: No automated testing framework configured for backend
- Note: API testing via Swagger UI available at `http://localhost:5167/docs`

## Test File Organization

**Location:**
- Rust: **Inline with source code** - Tests in `#[cfg(test)] mod tests { ... }` blocks
  - No separate test directory structure
  - Tests co-located with implementation
- TypeScript: No test files detected in codebase
- Python: No test files detected in codebase

**Naming:**
- Rust: Test functions prefixed with `test_` inside `#[cfg(test)]` modules
  - Example: `test_backend_to_string`, `test_backend_from_string`
- File pattern: Tests appear at end of implementation files

**Structure by Module:**
```
frontend/src-tauri/src/
├── audio/
│   ├── capture/
│   │   ├── backend_config.rs        # 26 tests for AudioCaptureBackend
│   │   └── [tests inline in file]
│   ├── device_detection.rs          # 7+ tests for device kind detection
│   ├── device_monitor.rs            # 1+ test for monitoring
│   ├── diagnostics.rs               # 1+ test for diagnostics
│   ├── hardware_detector.rs         # 3+ tests for hardware detection
│   └── buffer_pool.rs               # 3+ tests for buffer management
```

## Test Structure

**Rust Inline Test Pattern:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Setup
        let input = value;

        // Execute
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = function_that_might_fail();
        assert!(result.is_err());
    }
}
```

**Example from `backend_config.rs` (lines 152-226):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_to_string() {
        assert_eq!(AudioCaptureBackend::ScreenCaptureKit.to_string(), "screencapturekit");
        #[cfg(target_os = "macos")]
        assert_eq!(AudioCaptureBackend::CoreAudio.to_string(), "coreaudio");
    }

    #[test]
    fn test_backend_from_string() {
        assert_eq!(
            AudioCaptureBackend::from_string("screencapturekit"),
            Some(AudioCaptureBackend::ScreenCaptureKit)
        );
    }

    #[test]
    fn test_backend_config() {
        let config = BackendConfig::new();
        #[cfg(target_os = "macos")]
        assert_eq!(config.get(), AudioCaptureBackend::CoreAudio);

        // Test setting
        config.set(AudioCaptureBackend::CoreAudio);
        assert_eq!(config.get(), AudioCaptureBackend::CoreAudio);

        // Test reset
        config.reset();
        #[cfg(target_os = "macos")]
        assert_eq!(config.get(), AudioCaptureBackend::CoreAudio);
    }
}
```

**Patterns:**
- Setup phase: Create test data/fixtures inline
- Execution: Call function under test directly
- Assertion: Use `assert_eq!`, `assert!`, `assert!(result.is_ok())`
- Platform-specific tests: Wrap in `#[cfg(target_os = "macos")]` or similar

## Mocking

**Status:** Limited mocking infrastructure detected

**Rust Approaches Observed:**
- **No explicit mocking framework** (mockall, Mockito not in dev-dependencies)
- **Concrete type testing**: Tests use actual implementations
  - Example: `BackendConfig::new()` creates real config in tests
  - Tests verify behavior of actual types, not mocks

**What to Mock:**
- External system dependencies if needed (file I/O, network) - not currently done
- Platform-specific code could benefit from mocking but doesn't

**What NOT to Mock:**
- Configuration types (used directly in tests)
- Enums and value types (cheap to construct)
- Error handling paths

## Fixtures and Factories

**Test Data:**
- Inline construction: Test data created in test functions directly
  - Example from `backend_config.rs`: `AudioCaptureBackend::ScreenCaptureKit` constructed inline

No dedicated fixtures or factory patterns detected. Test data is:
- Simple: Constructed directly in test functions
- Immutable: Enums and simple values
- Platform-aware: Uses `#[cfg(...)]` for platform-specific fixtures

**Example Pattern:**
```rust
#[test]
fn test_feature() {
    // Fixture constructed inline
    let test_backend = AudioCaptureBackend::ScreenCaptureKit;
    let config = BackendConfig::new();

    // Use fixtures
    config.set(test_backend);
    assert_eq!(config.get(), test_backend);
}
```

**Location:**
- No separate fixtures directory
- Fixtures within test modules using `use super::*;`
- Some tests in `frontend/src-tauri/src/` use `Cargo.toml` dev-dependencies:
  - `tempfile`: For temporary file fixtures
  - `memory-stats`: For memory profiling during tests

## Coverage

**Requirements:** Not enforced (no coverage configuration found)

**View Coverage:**
```bash
# Option 1: Using tarpaulin (install with: cargo install cargo-tarpaulin)
cargo tarpaulin --out Html

# Option 2: Using llvm-cov (install with: cargo install cargo-llvm-cov)
cargo llvm-cov --html
```

**Observed Coverage Gaps:**
- TypeScript/React: No tests at all (0% coverage)
- Python backend: No tests at all (0% coverage)
- Rust: Tests for configuration and device detection modules only
  - Core audio pipeline: Limited testing
  - Recording manager: No dedicated tests
  - Transcription engine: No tests observed

## Test Types

**Unit Tests:**
- Scope: Single Rust struct/enum with its methods
- Approach: Inline `#[cfg(test)]` modules testing implementation
- Example: `AudioCaptureBackend` enum tests verify string conversion, parsing, defaults
- Frequency: Found in audio modules (device detection, configuration, diagnostics)
- Typical pattern: Test both happy path and error cases

**Integration Tests:**
- Not detected in codebase
- No separate `tests/` directory with integration test harness
- Full recording workflow (device detection → capture → transcription → save) not tested

**E2E Tests:**
- Not detected
- Manual testing via Tauri app UI or Swagger API docs

## Common Patterns

**Assertion Patterns:**

```rust
// Equality
assert_eq!(actual, expected);
assert_eq!(config.get(), AudioCaptureBackend::CoreAudio);

// Boolean conditions
assert!(backends.contains(&AudioCaptureBackend::ScreenCaptureKit));
assert!(result.is_ok());
assert!(result.is_err());

// Option/Result testing
assert_eq!(
    AudioCaptureBackend::from_string("invalid"),
    None
);
```

**Setup/Teardown Pattern:**

```rust
#[test]
fn test_with_config() {
    // Setup
    let config = BackendConfig::new();

    // Test
    config.set(AudioCaptureBackend::ScreenCaptureKit);
    assert_eq!(config.get(), AudioCaptureBackend::ScreenCaptureKit);

    // Teardown: Automatic via drop() when config goes out of scope
    // No explicit cleanup needed for test types
}
```

**Platform-Specific Testing:**

```rust
#[test]
fn test_backend_config() {
    let config = BackendConfig::new();

    // Run on all platforms
    #[cfg(target_os = "macos")]
    {
        // macOS-specific assertions
        assert_eq!(config.get(), AudioCaptureBackend::CoreAudio);
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Non-macOS assertions
        assert_eq!(config.get(), AudioCaptureBackend::ScreenCaptureKit);
    }
}
```

**Error Testing:**

No comprehensive error test patterns detected. Where used:
```rust
#[test]
fn test_error_case() {
    let result = operation_that_should_fail();
    assert!(result.is_err());
}
```

## Running Tests Locally

**Rust Tests:**

```bash
# Navigate to frontend directory with Tauri/Rust code
cd /Users/chaysenrathert/Desktop/Vaults/chays.ai/repositories/chays-meets/frontend/src-tauri

# Run all tests
cargo test

# Run specific module tests
cargo test audio::

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

**Testing Individual Modules:**

From the codebase, these have tests:
```bash
# Device configuration tests
cargo test capture::backend_config

# Device detection tests
cargo test audio::device_detection

# Device monitoring tests
cargo test audio::device_monitor

# Hardware detection tests
cargo test audio::hardware_detector

# Buffer pool tests
cargo test audio::buffer_pool

# Diagnostics tests
cargo test audio::diagnostics
```

## Test Gaps and Known Issues

**Rust:**
- Audio pipeline (`pipeline.rs`): No unit tests detected
- Recording manager (`recording_manager.rs`): No dedicated tests
- Audio stream capture (`capture/microphone.rs`, `capture/system.rs`): Not tested
- Whisper engine integration: No tests
- Parakeet engine integration: No tests
- Database layer: No tests
- Recording state management: No tests

**TypeScript/React:**
- All frontend code untested (hooks, contexts, components)
- Recording logic untested
- Transcript management untested
- Audio device selection untested
- Permission checking untested
- State synchronization untested

**Python/Backend:**
- All API endpoints untested
- Database operations untested
- LLM integration untested
- Transcript processing untested
- Summary generation untested
- Meeting CRUD operations untested

## CI/CD Testing Integration

**GitHub Actions:**
- Workflows found: `.github/workflows/`
- Status: Manual trigger configured (see CLAUDE.md)
- Build tests: Likely run via `cargo test` during build
- Frontend tests: Not running (no test suite)
- Backend tests: Not running (no test suite)

---

*Testing analysis: 2026-02-01*
