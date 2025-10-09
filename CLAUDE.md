# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`xx` is a Rust utility library providing enhanced versions of common operations with better error handling. The library uses a feature-based architecture where optional functionality is gated behind Cargo features.

## Development Commands

This project uses [mise](https://mise.jdx.dev) for task management. All tasks are defined in `mise.toml`.

### Building
```bash
mise run build              # Build with all features
mise run build-release      # Build release with all features
```

### Testing
```bash
mise run test               # Run all tests with all features enabled
mise run t                  # Alias for test
```

Tests run with:
- `RUST_LOG=xx=trace` for detailed logging
- `RUST_TEST_THREADS=1` for sequential execution (required for tests that modify shared state)
- `--all --all-features --nocapture` flags

### Linting
```bash
mise run lint               # Run hk check (includes format check and clippy)
mise run l                  # Alias for lint
mise run format             # Format code with cargo fmt
```

### CI/Release
```bash
mise run ci                 # Run lint + test (what CI runs)
mise run release            # Cut a new release using cargo-release
mise run clean              # Clean build artifacts
```

### Pre-commit Hooks
The project uses `lefthook` for git hooks. Pre-commit automatically runs `mise run lint`.

### Single Test Execution
To run a specific test:
```bash
cargo test --all-features test_name -- --nocapture
# Or with logging:
RUST_LOG=xx=trace cargo test --all-features test_name -- --nocapture
```

## Architecture

### Error Handling Pattern
All errors use the `XXError` enum (defined in `src/error.rs`) which wraps errors with contextual information:
- `FileError(std::io::Error, PathBuf)` - includes the file path
- `GitError(std::io::Error, PathBuf)` - includes the repo path
- `ProcessError(std::io::Error, String)` - includes the command
- Feature-specific errors (ArchiveIOError, HTTPError, etc.)

The library exports `XXResult<T>` as a type alias for `Result<T, XXError>`.

Two convenience macros are provided:
- `error!()` - creates an XXError with formatting
- `bail!()` - returns early with an XXError

### Feature Gates
Core modules (always available):
- `file` - Enhanced file operations
- `process` - Process execution
- `git` - Git repository operations
- `env` - Environment variable parsing
- `context` - Context management
- `error` - Error types

Optional modules (behind features):
- `archive` (feature: `archive`) - Archive extraction with sub-features:
  - `archive_untar_gzip`, `archive_untar_bzip2`, `archive_untar_xz`
  - `archive_unzip`, `archive_ungz`
- `hash` (feature: `hash`) - SHA256 hashing
- `http` (feature: `http`) - HTTP client with tokio runtime
- `fslock` (feature: `fslock`) - File system locking
- `glob` functionality in `file` module (feature: `glob`)

### Module Patterns

Each module follows consistent patterns:

1. **Enhanced std wrappers**: Functions wrap std operations with better error messages
2. **Auto-parent creation**: Write operations create parent directories automatically
3. **Path context**: All errors include relevant paths in error messages
4. **Re-exports**: Modules re-export relevant std items (e.g., `pub use std::fs::*` in file module)

### Testing Structure

- Tests are embedded in each module using `#[cfg(test)] mod tests`
- Test data is in `test/data/` directory (archives, test files, etc.)
- Uses `test_log::test` for test logging
- Uses `tempfile::tempdir()` via helper in `src/test.rs` for temporary directories
- Uses `pretty_assertions` for better assertion output
- Uses `insta` for snapshot testing

### Important Implementation Details

1. **File operations** (`src/file.rs`):
   - `write()` and `append()` automatically create parent directories
   - `mkdirp()` is idempotent (returns Ok if directory exists)
   - `find_up()` searches parent directories for files
   - Unix-specific: `chmod()`, `make_executable()` (no-op on Windows)

2. **Process execution** (`src/process.rs`):
   - Uses `duct` library for process handling
   - Shell commands use `sh` on Unix, `cmd` on Windows

3. **Git operations** (`src/git.rs`):
   - Uses `process` module to shell out to git commands
   - `Git` struct wraps a repository path
   - `CloneOptions` uses builder pattern

## MSRV (Minimum Supported Rust Version)

The project targets Rust 1.85+, specified in `Cargo.toml` as `rust-version = "1.85"`.

## CI/CD

GitHub Actions workflows:
- `test.yml` - Runs `mise run ci` (lint + test)
- `msrv.yml` - Validates MSRV compliance

## Code Style

- Use descriptive function documentation with Args/Returns/Errors/Example sections
- Include inline examples in doc comments
- Use `debug!()` and `trace!()` logging for diagnostics
- Prefer `PathBuf` over `Path` for storage, `AsRef<Path>` for parameters
