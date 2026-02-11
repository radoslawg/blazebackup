AGENTS.md - BlazeBackup Assistant Guide

Purpose
BlazeBackup is a Rust 2024 console app that compresses directories and uploads
them to Backblaze B2 via the S3-compatible API. The code favors minimal
dependencies and tutorial-style comments.

No Cursor or Copilot rules are present in this repo.

Context7 Usage
- Always use Context7 when you need code generation, setup or configuration steps,
  or library/API documentation. Automatically resolve the library id and fetch
  docs with the Context7 MCP tools without waiting for the user to ask.

Build, Run, Lint
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run (debug)
cargo run

# Run with args
cargo run -- <args>

# Run (release)
cargo run --release -- <args>

# Format
cargo fmt

# Format check
cargo fmt --check

# Lint
cargo clippy

# Lint with warnings as errors
cargo clippy -- -D warnings

# Format + lint
cargo fmt --check && cargo clippy -- -D warnings
```

Tests
```bash
# All tests
cargo test

# Single test by name (unit or integration)
cargo test <test_name>

# Single test with output
cargo test <test_name> -- --nocapture

# Run tests sequentially
cargo test -- --test-threads=1

# Only one integration test target
cargo test --test integration_test
```

Code Style and Conventions

Naming
- Functions, variables, modules: snake_case
- Types, traits: PascalCase
- Constants: SCREAMING_SNAKE_CASE

Imports
- Group in this order with blank lines between groups:
  1) std
  2) external crates (alphabetized)
  3) local modules

Example:
```rust
use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Result};
use aws_sdk_s3::Client;

use crate::utils::compress;
```

Formatting
- Use rustfmt defaults (`cargo fmt`).
- Keep line lengths reasonable for readability.

Types and Error Handling
- Use `anyhow::Result` for fallible functions.
- Add context with `anyhow::Context` on I/O or external calls.
- Prefer `bail!` for early exits with messages.

Example:
```rust
use anyhow::{bail, Context, Result};

fn read_config(path: &str) -> Result<String> {
    let data = std::fs::read_to_string(path)
        .context("Failed to read config file")?;
    if data.is_empty() {
        bail!("Config file is empty");
    }
    Ok(data)
}
```

Async and Blocking Work
- Use Tokio for async entry points and networking.
- For CPU-heavy or blocking I/O work, prefer `tokio::task::spawn_blocking`.

Comments and Documentation
- Write tutorial-style comments that explain why and how.
- Keep comments close to the code they explain.
- Document major features step-by-step in `Tutorial.md`.

Project-Specific Rules (from GEMINI.md)
- Minimize dependencies; use std or existing crates first.
- Keep solutions straightforward; avoid clever abstractions.
- Prefer educational code and clear structure.

Environment Variables
- `.env` is optional; `dotenv` loads it if present.
- Required values are defined in `example.env`.

Required Variables
- `BUCKET_NAME`: Backblaze B2 bucket name
- `AWS_ACCESS_KEY_ID`: B2 application key ID
- `AWS_SECRET_ACCESS_KEY`: B2 application key secret
- `AWS_REGION`: B2 region, e.g. `us-west-004`
- `AWS_ENDPOINT_URL`: B2 S3 endpoint

Repo Layout
```
blazebackup/
├── src/main.rs
├── tests/integration_test.rs
├── Cargo.toml
├── Cargo.lock
├── example.env
├── Tutorial.md
├── GEMINI.md
└── AGENTS.md
```

Dependencies (high level)
- anyhow: error handling
- aws-config/aws-sdk-s3: S3 client
- tokio: async runtime
- dotenv: `.env` loader
- serde/serde_json: config parsing
- zip/zipoxide/sevenz_rust2: archive creation

Git Commit Conventions
- Use gitmoji prefixes for commits.
- Examples:
  - ✨ Add zip compression for backup directories
  - 🐛 Fix S3 upload timeout for large files
  - 📝 Document environment variable configuration
  - ♻️ Extract S3 client into separate module
