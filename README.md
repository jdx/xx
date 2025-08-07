# xx

[![Crates.io](https://img.shields.io/crates/v/xx.svg)](https://crates.io/crates/xx)
[![Documentation](https://docs.rs/xx/badge.svg)](https://docs.rs/xx)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A collection of useful Rust macros and small utility functions to make common tasks easier.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xx = "2.1"
```

## Features

The library provides several optional features that can be enabled as needed:

- **`archive`** - Archive extraction support (tar.gz, tar.bz2, tar.xz, zip, gzip)
- **`glob`** - File globbing support
- **`hash`** - SHA256 hashing utilities
- **`http`** - HTTP client functionality
- **`fslock`** - File system locking

Enable features in your `Cargo.toml`:

```toml
[dependencies]
xx = { version = "2.1", features = ["archive", "glob", "hash"] }
```

## Modules

### Core Modules (Always Available)

- **`file`** - Enhanced file operations with better error handling
- **`process`** - Process execution utilities
- **`git`** - Git repository operations
- **`context`** - Context management utilities
- **`error`** - Error types and result helpers

### Optional Modules

- **`archive`** - Archive extraction (requires `archive` feature)
- **`hash`** - SHA256 hashing (requires `hash` feature)
- **`http`** - HTTP client (requires `http` feature)
- **`fslock`** - File locking (requires `fslock` feature)

## Examples

### File Operations

```rust
use xx::file;

// Read file with better error messages
let content = file::read_to_string("config.toml")?;

// Write file, creating parent directories automatically
file::write("output/data.txt", "Hello, world!")?;

// Create directory and parents
file::mkdirp("path/to/deep/directory")?;
```

### Process Execution

```rust
use xx::process;

// Run shell command
let output = process::sh("ls -la")?;

// Build and run commands
let result = process::cmd("git", &["status"]).read()?;
```

### Git Operations

```rust
use xx::git::{Git, CloneOptions};

// Clone a repository
let options = CloneOptions::default().branch("main");
let repo = xx::git::clone("https://github.com/user/repo", "/tmp/repo", &options)?;

// Work with existing repository
let git = Git::new("/path/to/repo".into());
let branch = git.current_branch()?;
let sha = git.current_sha()?;
```

## License

MIT - See [LICENSE](LICENSE) for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [Documentation](https://docs.rs/xx)
- [Crates.io](https://crates.io/crates/xx)
- [Repository](https://github.com/jdx/xx)