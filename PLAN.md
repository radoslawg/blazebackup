# BlazeBackup MVP Implementation Plan

## 1. Executive Summary

BlazeBackup is a Rust-based console application for compressing directories and uploading them to Backblaze B2 cloud storage via S3-compatible API. This document outlines the implementation plan to transform the current prototype into a Minimal Viable Product (MVP) with cross-platform support for Linux and Windows.

The current codebase (147 lines in `src/main.rs`) contains working but incomplete implementations of core features: ZIP compression with AES-256 encryption, Backblaze B2 uploads, and JSON configuration parsing. However, it suffers from several critical issues including hardcoded values, poor error handling, unprofessional debug code, and lack of change detection.

The MVP scope focuses on three primary goals:
1. **Externalize Configuration**: Move hardcoded paths and passwords to `.env` and external JSON configuration file
2. **Implement Change Detection**: Add file tracking to avoid redundant uploads when nothing has changed
3. **Cross-Platform Support**: Ensure proper path handling for both Windows and Linux environments

This plan defines 6 implementation phases with 23 specific tasks, comprehensive testing strategy, and estimated timeline of 16-24 hours of development work. The plan prioritizes technical debt cleanup before new feature implementation, following best practices outlined in `AGENTS.md`.

## 2. Current State Analysis

### Code Audit Results

**File**: `src/main.rs` (147 lines)

**Issue #1: Import Ordering Violation (Lines 11-20)**
```rust
use anyhow::Result;                    // line 11 - external
use aws_sdk_s3::{client, types::Bucket}; // line 14 - external  
use dotenv::dotenv;                    // line 17 - external
use serde_json::{Value, json};         // line 18 - external
use std::io::{self, Read};             // line 19 - std ❌ SHOULD BE FIRST
use zipoxide::create_zip_from_files;   // line 20 - external
```
**Problem**: Violates AGENTS.md standard requiring std → external → internal order with blank lines between groups.

**Issue #2: Unprofessional Debug Code (Lines 36-52)**
```rust
let config: Value = json!({
   "file1" : ["f1", "d1", "d3"],
   "file2" : ["f3", "f2"]
});
match &config {
    Value::Object(x) => {
        println!("Object! {:?}", x.keys());
    }
    Value::Array(y) => {
        println!("Array!{:?}", y);
    }
    _ => {
        println!("The fuck do I know");  // line 48 ❌ PROFANITY
    }
}
```
**Problem**: Contains profanity, non-functional test code, not integrated into application flow.

**Issue #3: Hardcoded Values (Lines 54-59)**
```rust
create_zip_from_files(
    "d:/projekty.zip".to_string(),        // line 55 ❌ Windows-specific path
    vec!["d:/Projekty/".to_string()],     // line 56 ❌ Windows-specific path
    zip::write::FileOptions::default().with_aes_encryption(
        zip::AesMode::Aes256, 
        "test123"  // line 57 ❌ Hardcoded password
    ),
)
```
**Problem**: Hardcoded Windows paths, hardcoded password, not cross-platform.

**Issue #4: Poor Error Handling in Main (Lines 67, 71)**
```rust
let bucket_name = std::env::var("BUCKET_NAME").unwrap();  // line 67 ❌ No context
upload_to_bucket(&String::from("d:/projekty.zip"), &bucket_name)
    .await
    .expect("Should work?");  // line 71 ❌ Unprofessional message
```
**Problem**: No error context, unhelpful error messages.

**Issue #5: Weak Error Handling in Upload Function (Lines 88-89, 100-101)**
```rust
let mut file = std::fs::File::open(filename).expect("Ok");  // line 88 ❌
file.read_to_end(&mut buffer).expect("Read file ok");  // line 89 ❌

match client.put_object()...send().await {
    Ok(_) => println!("Upload successfull to bucket: {:?}", bucket_name),
    Err(err) => println!("Error uploading object: {:?}", err),  // line 101 ❌ Doesn't propagate
}
```
**Problem**: Function returns `Result<()>` but prints errors instead of propagating them.

**Issue #6: Dead Code (Lines 109-146)**
```rust
async fn get_bucket(
    aws_client: &client::Client,
    bucket_name: &String,
) -> Result<Bucket> {
    // ... well-implemented but never called
}
```
**Problem**: 37 lines of code that is never used in the application.

### Dependencies Status

**Current `Cargo.toml`** (All dependencies needed for MVP are present):
- ✅ `anyhow` - Error handling (underutilized)
- ✅ `aws-sdk-s3`, `aws-config` - Backblaze B2 integration
- ✅ `tokio` - Async runtime
- ✅ `dotenv` - Environment variables
- ✅ `serde`, `serde_json` - JSON config (partially used)
- ✅ `zip`, `zipoxide` - Compression with AES encryption

**Action Required**: Add `serde` derive feature for struct deserialization:
```toml
serde = { version = "1.0.228", features = ["derive"] }
```

### Environment Configuration

**Current `.env`** (19 lines):
```bash
AWS_ACCESS_KEY_ID=00347363e86939b0000000002
AWS_SECRET_ACCESS_KEY=K003Iy7O5Fv5R4DS8mowSD4kjvfo8KI
BUCKET_NAME=radoslawg-Backups
AWS_ENDPOINT_URL=https://s3.eu-central-003.backblazeb2.com
AWS_REGION=eu-central-003
```

**Missing Variables**:
- `ZIP_ENCRYPTION_PASSWORD` - Currently hardcoded as "test123"
- `CONFIG_FILE_PATH` (optional) - Default location for config JSON
- `STATE_FILE_PATH` (optional) - Change detection state file

### Test Suite Status

**Current `tests/integration_test.rs`** (19 lines):
```rust
#[test]
fn test_main_function_runs() {
    assert!(true);  // ❌ Placeholder, does nothing
}
```
**Status**: Completely inadequate for MVP. Requires full rewrite.

## 3. MVP Requirements

### User-Specified Requirements

1. **Platform Support**: Must work on both Linux and Windows
2. **Minimal Viable Product**: Focus on core features only, no extras
3. **Complete Partial Implementations**:
   - JSON parsing for configuration (currently only debug code)
   - Zipping files (working but hardcoded)
   - Upload to Backblaze B2 (working but needs improvement)
4. **Configuration Externalization**:
   - Move encryption password from hardcoded "test123" to `.env`
   - Move source/output paths from hardcoded to external JSON file
5. **Change Detection**: Validate if files changed since last upload to avoid redundant backups
6. **Code Quality**: Clean up unprofessional code, improve error handling

### Explicit Non-Requirements (Out of Scope)

- ❌ CLI argument parsing (clap available but not needed yet)
- ❌ Multiple backup jobs in single run
- ❌ Incremental backups (full backup only)
- ❌ Backup rotation/retention policies
- ❌ Progress bars or fancy UI
- ❌ Logging to file (console output sufficient)
- ❌ Scheduled execution (manual runs only)
- ❌ Backup verification/restoration features

## 4. Design Decisions

### 4.1 Configuration File Schema

**Decision**: JSON file structure for `backup-config.json`

**Proposed Schema**:
```json
{
  "version": "1.0",
  "backup": {
    "name": "my-backup",
    "source": "/path/to/source",
    "output": "/path/to/output",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": [
      "*.tmp",
      "node_modules/",
      ".git/",
      "target/"
    ]
  },
  "storage": {
    "bucket": "radoslawg-Backups",
    "key_prefix": "backups/"
  }
}
```

**Rust Structs**:
```rust
#[derive(Debug, serde::Deserialize)]
struct BackupConfig {
    version: String,
    backup: BackupSettings,
    storage: StorageSettings,
}

#[derive(Debug, serde::Deserialize)]
struct BackupSettings {
    name: String,
    source: String,        // Will be converted to PathBuf
    output: String,        // Will be converted to PathBuf
    output_filename: String,
    #[serde(default)]
    exclude_patterns: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct StorageSettings {
    bucket: String,
    #[serde(default)]
    key_prefix: String,
}
```

**Rationale**:
- Use `{name}` and `{timestamp}` placeholders in filename
- Support exclusion patterns for future enhancement (MVP: log warning if present)
- Cross-platform paths (normalize on read using `PathBuf`)
- Bucket name in config (not .env) for flexibility with multiple configs
- Version field for future schema migrations

### 4.2 Change Detection Strategy

**Decision**: Hash-based approach for MVP

**Strategy**: Calculate SHA-256 hash of directory tree metadata (paths, sizes, timestamps). Single hash represents entire directory state. Compare hashes to detect ANY change.

**State File Format** (`.backup-state.json`):
```json
{
  "version": "1.0",
  "last_backup": "2024-01-15T10:30:00Z",
  "last_backup_file": "my-backup-20240115-103000.zip",
  "directory_hash": "a7f3c8e9d2b4f1e6c3a8d9f2e1b4c7a6d3f8e2c1b9a4f7e3d2c8b1a6f4e9d3c2"
}
```

**Rust Struct**:
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupState {
    version: String,
    last_backup: String,          // ISO 8601 timestamp
    last_backup_file: String,
    directory_hash: String,       // SHA-256 hash (64 hex chars)
}
```

**Detection Algorithm**:
1. Calculate current directory hash by walking tree and hashing metadata
2. Load previous hash from state file (if exists)
3. Compare: `current_hash != previous_hash` → backup required
4. Edge case: No state file → backup required (first run)

**What Gets Hashed**:
- Relative file path (normalized)
- File size in bytes
- Modified timestamp (as Unix seconds)
- **NOT file contents** (too slow - metadata is sufficient)

**Hash Calculation Implementation**:
```rust
use sha2::{Sha256, Digest};

fn calculate_directory_hash(source: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    
    // CRITICAL: Sort entries for deterministic hash
    let mut entries: Vec<_> = walkdir::WalkDir::new(source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    
    for entry in entries {
        let metadata = entry.metadata()?;
        let relative_path = entry.path().strip_prefix(source)?;
        
        hasher.update(relative_path.to_string_lossy().as_bytes());
        hasher.update(&metadata.len().to_le_bytes());
        
        let modified = metadata.modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        hasher.update(&modified.to_le_bytes());
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}
```

**Rationale**:
- **Privacy**: Filenames not exposed in state file (only cryptographic hash)
- **Performance**: O(n) directory walk, O(1) hash comparison
- **Accuracy**: Detects ALL changes (new/modified/deleted/renamed files)
- **Simplicity**: Tiny state file (~200 bytes vs 100KB+), simpler code (~50 lines vs ~110)
- **Short-circuit potential**: Can optimize in v2 to stop after detecting change
- **Cryptographic certainty**: SHA-256 collision probability ~10^-77

**Dependencies Required**:
- `sha2 = "0.10"` - SHA-256 hashing algorithm
- `walkdir = "2"` - Directory traversal (already planned)

**State File Location**:
- Default: `.backup-state.json` in same directory as config file
- Configurable via `.env` variable `STATE_FILE_PATH`

### 4.3 Cross-Platform Path Handling

**Decision**: Use `std::path::PathBuf` throughout, normalize on config load

**Implementation Strategy**:
```rust
use std::path::{Path, PathBuf};

// Read from JSON config
let source_str: String = config.backup.source;

// Convert to PathBuf (cross-platform)
let source_path = PathBuf::from(&source_str);
u   
// Validate path exists
if !source_path.exists() {
    anyhow::bail!("Source path does not exist: {}", source_path.display());
}

// Join paths (automatically uses correct separator)
let file_path = source_path.join("subdir").join("file.txt");

// Convert back to string for display
println!("Processing: {}", file_path.display());
```

**Path Normalization Rules**:
- Accept both `/` and `\` in JSON config (convert to native)
- Always use `PathBuf::join()` instead of string concatenation
- Use `path.display()` for user-facing output
- Use `path.to_string_lossy()` for internal processing
- Expand `~` to home directory on both platforms

**Platform-Specific Handling**:
- Windows: Handle drive letters (`C:\path`)
- Linux: Handle root paths (`/path`)
- Both: Support relative paths (resolved from current directory)

### 4.4 Error Handling Strategy

**Decision**: Fail fast with descriptive context using `anyhow`

**Principles**:
1. **No `.unwrap()` or `.expect()` in production code** (tests are OK)
2. **Always use `.context()` to add helpful error messages**
3. **Propagate errors with `?` operator**
4. **Print full error chain in main** using `{:#}` format

**Transformation Examples**:

**Before** (line 67):
```rust
let bucket_name = std::env::var("BUCKET_NAME").unwrap();
```

**After**:
```rust
let bucket_name = std::env::var("BUCKET_NAME")
    .context("Failed to read BUCKET_NAME from environment. Check your .env file.")?;
```

**Before** (line 88):
```rust
let mut file = std::fs::File::open(filename).expect("Ok");
```

**After**:
```rust
let mut file = std::fs::File::open(filename)
    .with_context(|| format!("Failed to open backup file: {}", filename))?;
```

**Before** (lines 100-101):
```rust
match client.put_object()...send().await {
    Ok(_) => println!("Upload successful"),
    Err(err) => println!("Error: {:?}", err),  // Doesn't propagate!
}
```

**After**:
```rust
client.put_object()
    .bucket(bucket_name)
    .key(&key)
    .body(buffer.into())
    .send()
    .await
    .context("Failed to upload backup to Backblaze B2")?;

println!("✓ Upload successful to bucket: {}", bucket_name);
Ok(())
```

**Main Function Error Display**:
```rust
#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {:#}", err);  // Print full error chain
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Main logic here
}
```

### 4.5 Module Structure

**Decision**: Keep `main.rs` monolithic for MVP

**Rationale**:
- Current code is ~150 lines, will grow to ~400-500 lines with MVP features
- Single file is easier to understand for new contributors
- Follows educational style from AGENTS.md
- Can refactor to modules post-MVP if needed

**Function Organization** (within `main.rs`):
```rust
// src/main.rs structure:

use std::...
use external::...

// ============================================================================
// Configuration Structures and Loading
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct BackupConfig { /* ... */ }

fn load_config(path: &Path) -> Result<BackupConfig> { /* ... */ }

// ============================================================================
// Change Detection
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupState { /* ... */ }

fn load_state(path: &Path) -> Result<Option<BackupState>> { /* ... */ }
fn save_state(path: &Path, state: &BackupState) -> Result<()> { /* ... */ }
fn build_current_state(source: &Path) -> Result<BackupState> { /* ... */ }
fn detect_changes(current: &BackupState, previous: Option<&BackupState>) -> bool { /* ... */ }

// ============================================================================
// Backup Operations
// ============================================================================

fn create_backup(config: &BackupConfig, password: &str) -> Result<PathBuf> { /* ... */ }

// ============================================================================
// Upload Operations
// ============================================================================

async fn upload_backup(file: &Path, bucket: &str, key: &str) -> Result<()> { /* ... */ }

// ============================================================================
// Main Orchestration
// ============================================================================

async fn run() -> Result<()> { /* ... */ }

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {:#}", err);
        std::process::exit(1);
    }
}
```

**Comment Style**: Tutorial-style comments explaining **why** and **how**, as per AGENTS.md.

## 5. Implementation Tasks (Priority Order)

### Phase 1: Critical Cleanup (Remove Technical Debt)

**Priority**: 🔴 CRITICAL - Must be done first  
**Estimated Effort**: 1-2 hours

#### Task 1.1: Fix Import Ordering
- **File**: `src/main.rs`, lines 11-20
- **Action**: Reorder imports to std → external → internal with blank lines
- **Effort**: 5 minutes

#### Task 1.2: Remove Debug JSON Code
- **File**: `src/main.rs`, lines 36-52
- **Action**: Delete entire block (17 lines)
- **Effort**: 2 minutes

#### Task 1.3: Remove or Annotate Dead Code
- **File**: `src/main.rs`, lines 109-146
- **Action**: Delete `get_bucket()` function (not needed for MVP)
- **Effort**: 2 minutes

#### Task 1.4: Improve Error Handling Throughout
- **File**: `src/main.rs`, lines 67, 71, 88-89, 100-101
- **Action**: Replace all `.unwrap()`, `.expect()` with `.context()?`
- **Effort**: 30 minutes

#### Task 1.5: Fix Function Signatures
- **File**: `src/main.rs`, line 76
- **Action**: Change `&String` to `&str` in function parameters
- **Effort**: 10 minutes

#### Task 1.6: Run cargo fmt and cargo clippy
- **Action**: Format code and fix all warnings
- **Effort**: 15 minutes

### Phase 2: Configuration System

**Priority**: 🟡 HIGH - Required for MVP  
**Estimated Effort**: 3-4 hours

#### Task 2.1: Add Serde Derive Feature
- **File**: `Cargo.toml`
- **Action**: Update serde dependency to include derive feature
- **Effort**: 2 minutes

#### Task 2.2: Define Configuration Structs
- **File**: `src/main.rs`
- **Action**: Add `BackupConfig`, `BackupSettings`, `StorageSettings` structs with serde derives
- **Effort**: 20 minutes

#### Task 2.3: Implement load_config() Function
- **File**: `src/main.rs`
- **Action**: Read JSON file, parse with serde_json, validate fields
- **Effort**: 45 minutes

#### Task 2.4: Add Path Normalization
- **File**: `src/main.rs` in `load_config()`
- **Action**: Convert string paths to PathBuf, validate existence
- **Effort**: 30 minutes

#### Task 2.5: Update .env File
- **File**: `.env`
- **Action**: Add `ZIP_ENCRYPTION_PASSWORD` variable
- **Effort**: 2 minutes

#### Task 2.6: Update example.env
- **File**: `example.env`
- **Action**: Add new variables with placeholder values and comments
- **Effort**: 5 minutes

#### Task 2.7: Create Sample backup-config.json
- **File**: `backup-config.json` (new file)
- **Action**: Create example configuration file with comments
- **Effort**: 15 minutes

#### Task 2.8: Update Main to Use Config
- **File**: `src/main.rs`, lines 54-59, 67, 69
- **Action**: Replace hardcoded values with config struct fields
- **Effort**: 30 minutes

### Phase 3: Change Detection

**Priority**: 🟡 HIGH - Core MVP feature  
**Estimated Effort**: 4-5 hours

#### Task 3.1: Define State Structs
- **File**: `src/main.rs`
- **Action**: Add `BackupState`, `FileMetadata` structs with serde derives
- **Effort**: 15 minutes

#### Task 3.2: Implement build_current_state()
- **File**: `src/main.rs`
- **Action**: Walk directory, collect file metadata (size, modified time)
- **Effort**: 1.5 hours

#### Task 3.3: Implement load_state()
- **File**: `src/main.rs`
- **Action**: Read and parse state JSON file, handle missing file gracefully
- **Effort**: 30 minutes

#### Task 3.4: Implement save_state()
- **File**: `src/main.rs`
- **Action**: Serialize state to JSON and write to file
- **Effort**: 30 minutes

#### Task 3.5: Implement detect_changes()
- **File**: `src/main.rs`
- **Action**: Compare current state vs previous, return true if any changes
- **Effort**: 45 minutes

#### Task 3.6: Integrate into Main Flow
- **File**: `src/main.rs` in `run()`
- **Action**: Call change detection, skip backup if unchanged
- **Effort**: 30 minutes

#### Task 3.7: Add Timestamp Formatting
- **File**: `src/main.rs`
- **Action**: Generate ISO 8601 timestamps for state and filename
- **Effort**: 20 minutes

### Phase 4: Cross-Platform Support

**Priority**: 🟡 HIGH - MVP requirement  
**Estimated Effort**: 2-3 hours

#### Task 4.1: Replace String Paths with PathBuf
- **File**: `src/main.rs`
- **Action**: Update all path variables to use `PathBuf` instead of `String`
- **Effort**: 1 hour

#### Task 4.2: Use PathBuf::join() for Path Construction
- **File**: `src/main.rs`
- **Action**: Replace string concatenation with `.join()`
- **Effort**: 30 minutes

#### Task 4.3: Add Home Directory Expansion
- **File**: `src/main.rs` in `load_config()`
- **Action**: Expand `~` to home directory on both platforms
- **Effort**: 30 minutes

#### Task 4.4: Test on Windows
- **Action**: Run full workflow on Windows, fix platform-specific issues
- **Effort**: 45 minutes

#### Task 4.5: Test on Linux (if available)
- **Action**: Run full workflow on Linux, verify paths work correctly
- **Effort**: 30 minutes

### Phase 5: Testing

**Priority**: 🟢 MEDIUM - Quality assurance  
**Estimated Effort**: 4-6 hours

#### Task 5.1: Write Unit Tests for load_config()
- **File**: `src/main.rs` (add `#[cfg(test)]` module)
- **Action**: Test valid config, invalid JSON, missing fields
- **Effort**: 1 hour

#### Task 5.2: Write Unit Tests for Change Detection
- **File**: `src/main.rs`
- **Action**: Test unchanged files, new files, modified files, deleted files
- **Effort**: 1.5 hours

#### Task 5.3: Write Unit Tests for Path Normalization
- **File**: `src/main.rs`
- **Action**: Test Windows paths, Linux paths, relative paths, home expansion
- **Effort**: 1 hour

#### Task 5.4: Rewrite Integration Test
- **File**: `tests/integration_test.rs`
- **Action**: Full end-to-end test with temp directories
- **Effort**: 2 hours

#### Task 5.5: Add Manual Testing Checklist
- **File**: `TESTING.md` (new file)
- **Action**: Document manual test scenarios
- **Effort**: 30 minutes

### Phase 6: Documentation

**Priority**: 🟢 MEDIUM - User experience  
**Estimated Effort**: 2-3 hours

#### Task 6.1: Create README.md
- **File**: `README.md` (new file)
- **Action**: Installation, configuration, usage instructions
- **Effort**: 1 hour

#### Task 6.2: Update Inline Code Comments
- **File**: `src/main.rs`
- **Action**: Add tutorial-style comments throughout
- **Effort**: 1 hour

#### Task 6.3: Document Configuration Schema
- **File**: `CONFIG.md` (new file)
- **Action**: Detailed explanation of all config fields
- **Effort**: 45 minutes

#### Task 6.4: Create example.backup-config.json
- **File**: `example.backup-config.json` (new file)
- **Action**: Comprehensive example with comments
- **Effort**: 15 minutes

## 6. Detailed Task Breakdown

### Phase 1 Tasks (Detailed)

#### Task 1.1: Fix Import Ordering

**Location**: `src/main.rs`, lines 11-20

**Current Code**:
```rust
use anyhow::Result;
// ... mixed external and std imports
use std::io::{self, Read};
```

**Target Code**:
```rust
// Standard library imports
use std::io::Read;
use std::path::{Path, PathBuf};

// External crate imports
use anyhow::{Context, Result};
use aws_sdk_s3::Client;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json;
use zipoxide::create_zip_from_files;
```

**Acceptance Criteria**:
- [ ] All `std::` imports grouped at top
- [ ] Blank line before external crates
- [ ] Alphabetical order within each group
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

**Effort**: 5 minutes

**Dependencies**: None

---

#### Task 1.2: Remove Debug JSON Code

**Location**: `src/main.rs`, lines 36-52

**Action**: Delete the entire debug code block (17 lines)

**Lines to Remove**:
```rust
let config: Value = json!({ ... });
match &config { ... }
println!("Keys: {:?}", config);
```

**Acceptance Criteria**:
- [ ] Lines 36-52 completely removed
- [ ] Code compiles without errors
- [ ] No references to `Value` or `json!` macro remain (except in actual config loading)

**Effort**: 2 minutes

**Dependencies**: None

---

#### Task 1.3: Remove Dead Code

**Location**: `src/main.rs`, lines 109-146

**Action**: Delete `get_bucket()` function entirely

**Rationale**: Function is well-written but unused. Not needed for MVP. Can be added back post-MVP if bucket validation is desired.

**Acceptance Criteria**:
- [ ] `get_bucket()` function removed
- [ ] No compiler warnings about dead code
- [ ] `cargo build` succeeds

**Effort**: 2 minutes

**Dependencies**: None

---

#### Task 1.4: Improve Error Handling Throughout

**Locations**: Lines 67, 71, 88-89, 100-101

**Transformation 1** (line 67):
```rust
// BEFORE:
let bucket_name = std::env::var("BUCKET_NAME").unwrap();

// AFTER:
let bucket_name = std::env::var("BUCKET_NAME")
    .context("Failed to read BUCKET_NAME from environment. Check your .env file.")?;
```

**Transformation 2** (line 71):
```rust
// BEFORE:
upload_to_bucket(&String::from("d:/projekty.zip"), &bucket_name)
    .await
    .expect("Should work?");

// AFTER:
upload_to_bucket(&zip_path, &bucket_name, &key)
    .await
    .context("Failed to upload backup to Backblaze B2")?;
```

**Transformation 3** (lines 88-89):
```rust
// BEFORE:
let mut file = std::fs::File::open(filename).expect("Ok");
file.read_to_end(&mut buffer).expect("Read file ok");

// AFTER:
let mut file = std::fs::File::open(filename)
    .with_context(|| format!("Failed to open backup file: {}", filename.display()))?;
file.read_to_end(&mut buffer)
    .with_context(|| format!("Failed to read backup file: {}", filename.display()))?;
```

**Transformation 4** (lines 100-101):
```rust
// BEFORE:
match client.put_object()...send().await {
    Ok(_) => println!("Upload successfull"),
    Err(err) => println!("Error: {:?}", err),
}

// AFTER:
client
    .put_object()
    .bucket(bucket_name)
    .key(key)
    .body(buffer.into())
    .send()
    .await
    .context("Failed to upload backup to Backblaze B2")?;

println!("✓ Successfully uploaded to bucket: {}", bucket_name);
Ok(())
```

**Acceptance Criteria**:
- [ ] No `.unwrap()` in production code
- [ ] No `.expect()` in production code
- [ ] All errors propagated with `?` operator
- [ ] All errors have descriptive context messages
- [ ] `cargo clippy` passes without warnings

**Effort**: 30 minutes

**Dependencies**: None

---

#### Task 1.5: Fix Function Signatures

**Location**: `src/main.rs`, line 76

**Current**:
```rust
async fn upload_to_bucket(filename: &String, bucket_name: &String) -> Result<()>
```

**Target**:
```rust
async fn upload_to_bucket(filename: &Path, bucket_name: &str, key: &str) -> Result<()>
```

**Rationale**: 
- `&String` should be `&str` (more flexible, idiomatic)
- `filename` should be `&Path` for proper path handling
- Add `key` parameter for S3 object key

**Acceptance Criteria**:
- [ ] Function signature updated
- [ ] All call sites updated
- [ ] Clippy warning about `&String` resolved

**Effort**: 10 minutes

**Dependencies**: Task 1.4 (error handling)

---

#### Task 1.6: Format and Lint Code

**Commands**:
```bash
cargo fmt
cargo clippy -- -D warnings
cargo build
```

**Acceptance Criteria**:
- [ ] `cargo fmt` makes no changes (already formatted)
- [ ] `cargo clippy` reports 0 warnings
- [ ] `cargo build` succeeds with 0 warnings

**Effort**: 15 minutes

**Dependencies**: Tasks 1.1-1.5 completed

---

### Phase 2 Tasks (Detailed)

#### Task 2.1: Add Serde Derive Feature

**Location**: `Cargo.toml`

**Current**:
```toml
serde = "1.0.228"
```

**Target**:
```toml
serde = { version = "1.0.228", features = ["derive"] }
```

**Acceptance Criteria**:
- [ ] Dependency updated
- [ ] `cargo build` succeeds

**Effort**: 2 minutes

**Dependencies**: None

---

#### Task 2.2: Define Configuration Structs

**Location**: `src/main.rs` (after imports, before functions)

**Code to Add**:
```rust
// ============================================================================
// Configuration Structures
// ============================================================================

/// Main configuration for backup operations.
/// Loaded from JSON file (default: backup-config.json).
#[derive(Debug, Deserialize)]
struct BackupConfig {
    /// Configuration schema version (currently "1.0")
    version: String,
    
    /// Backup-specific settings
    backup: BackupSettings,
    
    /// Cloud storage settings
    storage: StorageSettings,
}

/// Settings for backup creation and source files.
#[derive(Debug, Deserialize)]
struct BackupSettings {
    /// Name of this backup (used in filename generation)
    name: String,
    
    /// Source directory to backup
    source: String,  // Will be converted to PathBuf
    
    /// Output directory for backup files
    output: String,  // Will be converted to PathBuf
    
    /// Output filename template (supports {name} and {timestamp} placeholders)
    output_filename: String,
    
    /// File patterns to exclude (future enhancement, logged as warning in MVP)
    #[serde(default)]
    exclude_patterns: Vec<String>,
}

/// Settings for Backblaze B2 cloud storage.
#[derive(Debug, Deserialize)]
struct StorageSettings {
    /// B2 bucket name
    bucket: String,
    
    /// S3 key prefix for uploaded files (optional)
    #[serde(default)]
    key_prefix: String,
}
```

**Acceptance Criteria**:
- [ ] All structs defined with proper derives
- [ ] Tutorial-style comments added
- [ ] Code compiles

**Effort**: 20 minutes

**Dependencies**: Task 2.1 (serde derive feature)

---

#### Task 2.3: Implement load_config() Function

**Location**: `src/main.rs`

**Function to Add**:
```rust
/// Loads backup configuration from JSON file.
/// 
/// # Arguments
/// * `path` - Path to the JSON configuration file
/// 
/// # Returns
/// * `Ok(BackupConfig)` on success
/// * `Err` with context if file cannot be read or parsed
/// 
/// # Validation
/// - Checks that source directory exists
/// - Checks that output directory exists (or creates it)
/// - Validates that version field is "1.0"
fn load_config(path: &Path) -> Result<BackupConfig> {
    // Read the JSON file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    
    // Parse JSON into BackupConfig struct
    let config: BackupConfig = serde_json::from_str(&content)
        .context("Failed to parse config file. Check JSON syntax.")?;
    
    // Validate version
    if config.version != "1.0" {
        anyhow::bail!("Unsupported config version: {}. Expected 1.0", config.version);
    }
    
    // Validate source directory exists
    let source_path = PathBuf::from(&config.backup.source);
    if !source_path.exists() {
        anyhow::bail!("Source directory does not exist: {}", source_path.display());
    }
    if !source_path.is_dir() {
        anyhow::bail!("Source path is not a directory: {}", source_path.display());
    }
    
    // Validate/create output directory
    let output_path = PathBuf::from(&config.backup.output);
    if !output_path.exists() {
        std::fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory: {}", output_path.display()))?;
    }
    
    // Warn if exclude patterns are present (not implemented in MVP)
    if !config.backup.exclude_patterns.is_empty() {
        eprintln!("Warning: exclude_patterns are not yet implemented and will be ignored.");
    }
    
    Ok(config)
}
```

**Acceptance Criteria**:
- [ ] Function compiles and follows error handling strategy
- [ ] Validates source directory exists
- [ ] Creates output directory if needed
- [ ] Returns helpful errors with context
- [ ] Tutorial-style comments present

**Effort**: 45 minutes

**Dependencies**: Task 2.2 (structs defined)

---

#### Task 2.4: Add Path Normalization

**Location**: Within `load_config()` function

**Enhancement**: Add home directory expansion

**Code to Add** (at start of function):
```rust
/// Expands ~ to home directory in a path string.
/// Works on both Windows and Linux.
fn expand_home_dir(path_str: &str) -> String {
    if path_str.starts_with('~') {
        if let Some(home) = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
        {
            return path_str.replacen('~', &home, 1);
        }
    }
    path_str.to_string()
}

// In load_config(), after parsing JSON:
let source_expanded = expand_home_dir(&config.backup.source);
let output_expanded = expand_home_dir(&config.backup.output);

let source_path = PathBuf::from(source_expanded);
let output_path = PathBuf::from(output_expanded);
```

**Acceptance Criteria**:
- [ ] `~` expands to home directory on both platforms
- [ ] Paths without `~` remain unchanged
- [ ] Works with `HOME` (Linux) and `USERPROFILE` (Windows)

**Effort**: 30 minutes

**Dependencies**: Task 2.3 (load_config implemented)

---

#### Task 2.5: Update .env File

**Location**: `.env`

**Lines to Add**:
```bash
# ZIP encryption password (AES-256)
# IMPORTANT: Keep this secure. Do not commit to version control.
ZIP_ENCRYPTION_PASSWORD=your-secure-password-here

# Optional: Custom config file path (defaults to backup-config.json)
# CONFIG_FILE_PATH=backup-config.json

# Optional: State file for change detection (defaults to .backup-state.json)
# STATE_FILE_PATH=.backup-state.json
```

**Acceptance Criteria**:
- [ ] New variables added with helpful comments
- [ ] `.env` file still loads correctly
- [ ] Security warning included for password

**Effort**: 2 minutes

**Dependencies**: None

---

#### Task 2.6: Update example.env

**Location**: `example.env`

**Add Same Variables as Task 2.5**:
```bash
# ZIP encryption password (AES-256)
ZIP_ENCRYPTION_PASSWORD=replace-with-secure-password

# Optional: Custom config file path
# CONFIG_FILE_PATH=backup-config.json

# Optional: State file for change detection
# STATE_FILE_PATH=.backup-state.json
```

**Acceptance Criteria**:
- [ ] Template updated with placeholder values
- [ ] Comments match `.env`

**Effort**: 5 minutes

**Dependencies**: Task 2.5 (for consistency)

---

#### Task 2.7: Create Sample backup-config.json

**Location**: `backup-config.json` (new file in project root)

**Content**:
```json
{
  "version": "1.0",
  "backup": {
    "name": "my-backup",
    "source": "~/Documents",
    "output": "~/Backups",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": []
  },
  "storage": {
    "bucket": "your-bucket-name",
    "key_prefix": "backups/"
  }
}
```

**Acceptance Criteria**:
- [ ] File created in project root
- [ ] Valid JSON syntax
- [ ] All required fields present
- [ ] Loads successfully with `load_config()`

**Effort**: 15 minutes

**Dependencies**: Task 2.3 (load_config function)

---

#### Task 2.8: Update Main to Use Config

**Location**: `src/main.rs` in `run()` function

**Current Code to Replace** (lines 54-59, 67, 69):
```rust
// OLD - hardcoded values
create_zip_from_files(
    "d:/projekty.zip".to_string(),
    vec!["d:/Projekty/".to_string()],
    zip::write::FileOptions::default().with_aes_encryption(
        zip::AesMode::Aes256, 
        "test123"
    ),
)?;
let bucket_name = std::env::var("BUCKET_NAME")?;
upload_to_bucket(&String::from("d:/projekty.zip"), &bucket_name).await?;
```

**New Code**:
```rust
// Load configuration
let config_path = std::env::var("CONFIG_FILE_PATH")
    .unwrap_or_else(|_| "backup-config.json".to_string());
let config = load_config(Path::new(&config_path))
    .context("Failed to load configuration file")?;

// Load encryption password from environment
let password = std::env::var("ZIP_ENCRYPTION_PASSWORD")
    .context("ZIP_ENCRYPTION_PASSWORD not set in .env file")?;

// Build paths
let source_path = PathBuf::from(&config.backup.source);
let output_path = PathBuf::from(&config.backup.output);

// Generate output filename with timestamp
let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
let filename = config.backup.output_filename
    .replace("{name}", &config.backup.name)
    .replace("{timestamp}", &timestamp.to_string());
let zip_path = output_path.join(filename);

// Create backup
println!("Creating backup: {}", zip_path.display());
create_zip_from_files(
    zip_path.to_string_lossy().to_string(),
    vec![source_path.to_string_lossy().to_string()],
    zip::write::FileOptions::default()
        .with_aes_encryption(zip::AesMode::Aes256, &password),
)
.context("Failed to create ZIP archive")?;

// Build S3 key
let key = format!("{}{}", config.storage.key_prefix, zip_path.file_name().unwrap().to_string_lossy());

// Upload
println!("Uploading to Backblaze B2...");
upload_to_bucket(&zip_path, &config.storage.bucket, &key).await?;
println!("✓ Backup complete: {}", zip_path.display());
```

**Note**: Will need to add `chrono` dependency for timestamp formatting:
```toml
chrono = "0.4"
```

**Acceptance Criteria**:
- [ ] No hardcoded paths remain
- [ ] Password read from `.env`
- [ ] Config loaded from JSON file
- [ ] Timestamps generated correctly
- [ ] S3 key includes prefix from config

**Effort**: 30 minutes

**Dependencies**: Tasks 2.2-2.7 completed

---

### Phase 3 Tasks (Detailed)

#### Task 3.1: Define State Structs

**Location**: `src/main.rs` (after config structs)

**Code to Add**:
```rust
// ============================================================================
// Change Detection Structures
// ============================================================================

/// State file for tracking file changes between backups.
/// Stored as JSON in .backup-state.json by default.
#[derive(Debug, Serialize, Deserialize)]
struct BackupState {
    /// Schema version for future compatibility
    version: String,
    
    /// ISO 8601 timestamp of last successful backup
    last_backup: String,
    
    /// Filename of last backup created
    last_backup_file: String,
    
    /// Map of file paths to their metadata
    /// Key: relative path from source directory
    /// Value: file metadata (size and modified time)
    files: std::collections::HashMap<String, FileMetadata>,
}

/// Metadata for a single file used in change detection.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct FileMetadata {
    /// File size in bytes
    size: u64,
    
    /// ISO 8601 timestamp of last modification
    modified: String,
}
```

**Acceptance Criteria**:
- [ ] Structs compile with serde derives
- [ ] Comments explain purpose
- [ ] PartialEq on FileMetadata for comparison

**Effort**: 15 minutes

**Dependencies**: None

---

#### Task 3.2: Implement build_current_state()

**Location**: `src/main.rs`

**Function to Add**:
```rust
/// Builds current state by walking source directory and collecting file metadata.
/// 
/// # Arguments
/// * `source` - Root directory to scan
/// * `backup_name` - Name of backup (from config)
/// 
/// # Returns
/// * `BackupState` with current timestamp and file metadata
fn build_current_state(source: &Path, backup_name: &str) -> Result<BackupState> {
    use std::collections::HashMap;
    use std::time::SystemTime;
    
    let mut files = HashMap::new();
    
    // Walk directory recursively
    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Skip directories, only process files
        if !path.is_file() {
            continue;
        }
        
        // Get file metadata
        let metadata = path.metadata()
            .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;
        
        // Get relative path from source directory
        let rel_path = path.strip_prefix(source)
            .with_context(|| format!("Failed to compute relative path for: {}", path.display()))?;
        let rel_path_str = rel_path.to_string_lossy().to_string();
        
        // Get modified time
        let modified_time = metadata.modified()
            .with_context(|| format!("Failed to read modified time for: {}", path.display()))?;
        let modified_datetime = chrono::DateTime::<chrono::Utc>::from(modified_time);
        
        // Store file metadata
        files.insert(rel_path_str, FileMetadata {
            size: metadata.len(),
            modified: modified_datetime.to_rfc3339(),
        });
    }
    
    // Build state
    Ok(BackupState {
        version: "1.0".to_string(),
        last_backup: chrono::Utc::now().to_rfc3339(),
        last_backup_file: format!("{}-{}.zip", backup_name, chrono::Utc::now().format("%Y%m%d-%H%M%S")),
        files,
    })
}
```

**Note**: Requires `walkdir` dependency:
```toml
walkdir = "2"
```

**Acceptance Criteria**:
- [ ] Recursively walks all subdirectories
- [ ] Collects size and modified time for each file
- [ ] Relative paths computed correctly
- [ ] Returns valid BackupState

**Effort**: 1.5 hours

**Dependencies**: Task 3.1 (structs), add walkdir to Cargo.toml

---

#### Task 3.3: Implement load_state()

**Location**: `src/main.rs`

**Function to Add**:
```rust
/// Loads previous backup state from JSON file.
/// 
/// # Arguments
/// * `path` - Path to state file (typically .backup-state.json)
/// 
/// # Returns
/// * `Ok(Some(BackupState))` if file exists and is valid
/// * `Ok(None)` if file doesn't exist (first run)
/// * `Err` if file exists but is corrupted/unparseable
fn load_state(path: &Path) -> Result<Option<BackupState>> {
    // Check if state file exists
    if !path.exists() {
        return Ok(None);  // First run, no previous state
    }
    
    // Read and parse state file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read state file: {}", path.display()))?;
    
    let state: BackupState = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file: {}", path.display()))?;
    
    // Validate version
    if state.version != "1.0" {
        anyhow::bail!("Unsupported state file version: {}. Expected 1.0", state.version);
    }
    
    Ok(Some(state))
}
```

**Acceptance Criteria**:
- [ ] Returns None if file doesn't exist (not an error)
- [ ] Returns Some(state) if file is valid
- [ ] Returns Err if file is corrupted
- [ ] Validates version field

**Effort**: 30 minutes

**Dependencies**: Task 3.1 (structs)

---

#### Task 3.4: Implement save_state()

**Location**: `src/main.rs`

**Function to Add**:
```rust
/// Saves backup state to JSON file.
/// 
/// # Arguments
/// * `path` - Path to state file (typically .backup-state.json)
/// * `state` - BackupState to save
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err` with context on failure
fn save_state(path: &Path, state: &BackupState) -> Result<()> {
    // Serialize to JSON with pretty formatting
    let json = serde_json::to_string_pretty(state)
        .context("Failed to serialize state to JSON")?;
    
    // Write to file
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write state file: {}", path.display()))?;
    
    println!("✓ State saved: {}", path.display());
    Ok(())
}
```

**Acceptance Criteria**:
- [ ] Creates/overwrites state file
- [ ] JSON is pretty-printed (readable)
- [ ] Returns helpful error with path on failure

**Effort**: 30 minutes

**Dependencies**: Task 3.1 (structs)

---

#### Task 3.5: Implement detect_changes()

**Location**: `src/main.rs`

**Function to Add**:
```rust
/// Detects if any files have changed since last backup.
/// 
/// # Arguments
/// * `current` - Current state built from filesystem
/// * `previous` - Previous state loaded from state file (None on first run)
/// 
/// # Returns
/// * `true` if backup is needed (files changed or first run)
/// * `false` if no changes detected
fn detect_changes(current: &BackupState, previous: Option<&BackupState>) -> bool {
    // If no previous state, backup is needed
    let prev = match previous {
        Some(p) => p,
        None => {
            println!("ℹ No previous backup state found. Full backup required.");
            return true;
        }
    };
    
    // Check if file count changed
    if current.files.len() != prev.files.len() {
        println!("ℹ File count changed: {} → {} files", prev.files.len(), current.files.len());
        return true;
    }
    
    // Check each file in current state
    for (path, current_meta) in &current.files {
        match prev.files.get(path) {
            Some(prev_meta) => {
                // File existed before, check if changed
                if current_meta != prev_meta {
                    println!("ℹ File changed: {}", path);
                    println!("  Previous: {} bytes, {}", prev_meta.size, prev_meta.modified);
                    println!("  Current:  {} bytes, {}", current_meta.size, current_meta.modified);
                    return true;
                }
            }
            None => {
                // New file detected
                println!("ℹ New file detected: {}", path);
                return true;
            }
        }
    }
    
    // Check for deleted files
    for path in prev.files.keys() {
        if !current.files.contains_key(path) {
            println!("ℹ File deleted: {}", path);
            return true;
        }
    }
    
    // No changes detected
    println!("✓ No changes detected since last backup.");
    false
}
```

**Acceptance Criteria**:
- [ ] Returns true on first run (no previous state)
- [ ] Detects new files
- [ ] Detects modified files (size or time changed)
- [ ] Detects deleted files
- [ ] Returns false only if truly no changes
- [ ] Prints informative messages about what changed

**Effort**: 45 minutes

**Dependencies**: Task 3.1 (structs)

---

#### Task 3.6: Integrate into Main Flow

**Location**: `src/main.rs` in `run()` function

**Code to Add** (before backup creation):
```rust
// Determine state file path
let state_path = std::env::var("STATE_FILE_PATH")
    .unwrap_or_else(|_| ".backup-state.json".to_string());
let state_file = PathBuf::from(state_path);

// Build current filesystem state
println!("Scanning source directory...");
let current_state = build_current_state(&source_path, &config.backup.name)
    .context("Failed to build current file state")?;
println!("✓ Found {} files", current_state.files.len());

// Load previous state
let previous_state = load_state(&state_file)
    .context("Failed to load previous backup state")?;

// Detect changes
let needs_backup = detect_changes(&current_state, previous_state.as_ref());

if !needs_backup {
    println!("Backup skipped - no changes detected.");
    return Ok(());
}

println!("Changes detected. Creating backup...");

// ... existing backup creation code ...

// After successful backup and upload:
let mut final_state = current_state;
final_state.last_backup = chrono::Utc::now().to_rfc3339();
final_state.last_backup_file = zip_path.file_name().unwrap().to_string_lossy().to_string();

save_state(&state_file, &final_state)
    .context("Failed to save backup state")?;
```

**Acceptance Criteria**:
- [ ] State checked before backup
- [ ] Backup skipped if no changes
- [ ] State saved after successful backup
- [ ] State NOT saved if backup fails
- [ ] Informative messages printed

**Effort**: 30 minutes

**Dependencies**: Tasks 3.2-3.5 (all change detection functions)

---

#### Task 3.7: Add Timestamp Formatting

**Location**: Update `Cargo.toml` if not already added

**Dependency**:
```toml
chrono = "0.4"
```

**Usage**: Already implemented in previous tasks via:
```rust
chrono::Utc::now().to_rfc3339()  // ISO 8601 format
chrono::Utc::now().format("%Y%m%d-%H%M%S")  // Filename format
```

**Acceptance Criteria**:
- [ ] Timestamps are ISO 8601 format in state file
- [ ] Timestamps are filesystem-safe in filenames
- [ ] Works consistently on both platforms

**Effort**: 20 minutes (testing and validation)

**Dependencies**: None (dependency likely already added in Phase 2)

---

### Phase 4 Tasks (Detailed)

#### Task 4.1: Replace String Paths with PathBuf

**Location**: Throughout `src/main.rs`

**Transformations**:
```rust
// BEFORE:
let source: String = config.backup.source;
let output: String = config.backup.output;
let zip_path = format!("{}/{}", output, filename);

// AFTER:
let source = PathBuf::from(&config.backup.source);
let output = PathBuf::from(&config.backup.output);
let zip_path = output.join(&filename);
```

**Acceptance Criteria**:
- [ ] All path variables are `PathBuf` or `&Path`
- [ ] No string concatenation for paths
- [ ] Use `.display()` for printing paths
- [ ] Use `.to_string_lossy()` only when necessary for external APIs

**Effort**: 1 hour

**Dependencies**: Phase 2 completed

---

#### Task 4.2: Use PathBuf::join() for Path Construction

**Location**: Anywhere paths are combined

**Examples**:
```rust
// BEFORE:
let file_path = source + "/" + filename;

// AFTER:
let file_path = source.join(filename);

// BEFORE:
let key = format!("{}{}", prefix, filename);

// AFTER (if both are paths):
let key_path = PathBuf::from(prefix).join(filename);
let key = key_path.to_string_lossy().to_string();
```

**Acceptance Criteria**:
- [ ] All path joins use `.join()`
- [ ] No manual `/` or `\` in path construction
- [ ] Works on both Windows and Linux

**Effort**: 30 minutes

**Dependencies**: Task 4.1

---

#### Task 4.3: Add Home Directory Expansion

**Location**: Already implemented in Task 2.4

**Verify**:
- [ ] `~` expands correctly on Windows (`USERPROFILE`)
- [ ] `~` expands correctly on Linux (`HOME`)
- [ ] Works in both source and output paths

**Effort**: 30 minutes (testing)

**Dependencies**: Task 2.4

---

#### Task 4.4: Test on Windows

**Manual Test Procedure**:
1. Create test config with Windows paths: `C:\Users\...`
2. Create test config with forward slashes: `C:/Users/...`
3. Create test config with `~` home directory
4. Run full backup workflow
5. Verify ZIP created in correct location
6. Verify upload succeeds
7. Verify state file saved correctly

**Acceptance Criteria**:
- [ ] All path formats work correctly
- [ ] No path separator issues
- [ ] ZIP file created with correct name
- [ ] Upload succeeds

**Effort**: 45 minutes

**Dependencies**: Tasks 4.1-4.3

---

#### Task 4.5: Test on Linux

**Manual Test Procedure** (if Linux available):
1. Create test config with Linux paths: `/home/user/...`
2. Create test config with `~` home directory
3. Run full backup workflow
4. Verify ZIP created in correct location
5. Verify upload succeeds
6. Verify state file saved correctly

**Alternative** (if Linux unavailable):
- Use WSL on Windows
- Use Docker container with Rust

**Acceptance Criteria**:
- [ ] All path formats work correctly
- [ ] No path separator issues
- [ ] ZIP file created with correct name
- [ ] Upload succeeds

**Effort**: 30 minutes

**Dependencies**: Tasks 4.1-4.4

---

### Phase 5 Tasks (Detailed)

#### Task 5.1: Write Unit Tests for load_config()

**Location**: `src/main.rs` at end of file

**Code to Add**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_load_config_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let config_json = r#"{
            "version": "1.0",
            "backup": {
                "name": "test",
                "source": ".",
                "output": ".",
                "output_filename": "test.zip",
                "exclude_patterns": []
            },
            "storage": {
                "bucket": "test-bucket",
                "key_prefix": ""
            }
        }"#;
        
        std::fs::write(&config_path, config_json).unwrap();
        
        let config = load_config(&config_path).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.backup.name, "test");
        assert_eq!(config.storage.bucket, "test-bucket");
    }
    
    #[test]
    fn test_load_config_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        std::fs::write(&config_path, "{ invalid json }").unwrap();
        
        let result = load_config(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse"));
    }
    
    #[test]
    fn test_load_config_missing_source() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let config_json = r#"{
            "version": "1.0",
            "backup": {
                "name": "test",
                "source": "/nonexistent/path",
                "output": ".",
                "output_filename": "test.zip"
            },
            "storage": {"bucket": "test"}
        }"#;
        
        std::fs::write(&config_path, config_json).unwrap();
        
        let result = load_config(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }
}
```

**Note**: Requires `tempfile` dependency for tests:
```toml
[dev-dependencies]
tempfile = "3"
```

**Acceptance Criteria**:
- [ ] Test valid config passes
- [ ] Test invalid JSON fails appropriately
- [ ] Test missing source directory fails
- [ ] Test wrong version fails
- [ ] All tests pass with `cargo test`

**Effort**: 1 hour

**Dependencies**: Phase 2 completed

---

#### Task 5.2: Write Unit Tests for Change Detection

**Location**: `src/main.rs` in `#[cfg(test)]` module

**Tests to Add**:
```rust
#[test]
fn test_detect_changes_first_run() {
    let current = BackupState {
        version: "1.0".to_string(),
        last_backup: chrono::Utc::now().to_rfc3339(),
        last_backup_file: "test.zip".to_string(),
        files: std::collections::HashMap::new(),
    };
    
    assert!(detect_changes(&current, None));
}

#[test]
fn test_detect_changes_no_changes() {
    let state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-01T00:00:00Z".to_string(),
        last_backup_file: "test.zip".to_string(),
        files: {
            let mut map = std::collections::HashMap::new();
            map.insert("file.txt".to_string(), FileMetadata {
                size: 100,
                modified: "2024-01-01T00:00:00Z".to_string(),
            });
            map
        },
    };
    
    assert!(!detect_changes(&state, Some(&state)));
}

#[test]
fn test_detect_changes_file_modified() {
    let prev_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-01T00:00:00Z".to_string(),
        last_backup_file: "test.zip".to_string(),
        files: {
            let mut map = std::collections::HashMap::new();
            map.insert("file.txt".to_string(), FileMetadata {
                size: 100,
                modified: "2024-01-01T00:00:00Z".to_string(),
            });
            map
        },
    };
    
    let current_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-02T00:00:00Z".to_string(),
        last_backup_file: "test2.zip".to_string(),
        files: {
            let mut map = std::collections::HashMap::new();
            map.insert("file.txt".to_string(), FileMetadata {
                size: 200,  // Changed!
                modified: "2024-01-02T00:00:00Z".to_string(),
            });
            map
        },
    };
    
    assert!(detect_changes(&current_state, Some(&prev_state)));
}

#[test]
fn test_detect_changes_new_file() {
    let prev_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-01T00:00:00Z".to_string(),
        last_backup_file: "test.zip".to_string(),
        files: std::collections::HashMap::new(),
    };
    
    let current_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-02T00:00:00Z".to_string(),
        last_backup_file: "test2.zip".to_string(),
        files: {
            let mut map = std::collections::HashMap::new();
            map.insert("newfile.txt".to_string(), FileMetadata {
                size: 100,
                modified: "2024-01-02T00:00:00Z".to_string(),
            });
            map
        },
    };
    
    assert!(detect_changes(&current_state, Some(&prev_state)));
}

#[test]
fn test_detect_changes_deleted_file() {
    let prev_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-01T00:00:00Z".to_string(),
        last_backup_file: "test.zip".to_string(),
        files: {
            let mut map = std::collections::HashMap::new();
            map.insert("file.txt".to_string(), FileMetadata {
                size: 100,
                modified: "2024-01-01T00:00:00Z".to_string(),
            });
            map
        },
    };
    
    let current_state = BackupState {
        version: "1.0".to_string(),
        last_backup: "2024-01-02T00:00:00Z".to_string(),
        last_backup_file: "test2.zip".to_string(),
        files: std::collections::HashMap::new(),  // File deleted!
    };
    
    assert!(detect_changes(&current_state, Some(&prev_state)));
}
```

**Acceptance Criteria**:
- [ ] Test first run (no previous state)
- [ ] Test no changes
- [ ] Test file modified (size changed)
- [ ] Test file modified (time changed)
- [ ] Test new file added
- [ ] Test file deleted
- [ ] All tests pass

**Effort**: 1.5 hours

**Dependencies**: Phase 3 completed

---

#### Task 5.3: Write Unit Tests for Path Normalization

**Location**: `src/main.rs` in `#[cfg(test)]` module

**Tests to Add**:
```rust
#[test]
fn test_expand_home_dir_with_tilde() {
    let path = "~/Documents/test";
    let expanded = expand_home_dir(path);
    
    // Should not start with ~ anymore
    assert!(!expanded.starts_with('~'));
    
    // Should contain either HOME or USERPROFILE value
    let home_exists = std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok();
    if home_exists {
        assert!(expanded.contains("Documents"));
    }
}

#[test]
fn test_expand_home_dir_without_tilde() {
    let path = "/absolute/path/test";
    let expanded = expand_home_dir(path);
    
    // Should remain unchanged
    assert_eq!(expanded, path);
}

#[test]
fn test_pathbuf_join_cross_platform() {
    let base = PathBuf::from("/base");
    let joined = base.join("subdir").join("file.txt");
    
    // Should use platform-appropriate separator
    let as_string = joined.to_string_lossy();
    assert!(as_string.contains("subdir"));
    assert!(as_string.contains("file.txt"));
}
```

**Acceptance Criteria**:
- [ ] Tilde expansion works
- [ ] Non-tilde paths unchanged
- [ ] PathBuf joins work correctly
- [ ] Tests pass on both platforms

**Effort**: 1 hour

**Dependencies**: Phase 4 completed

---

#### Task 5.4: Rewrite Integration Test

**Location**: `tests/integration_test.rs`

**Complete Rewrite**:
```rust
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_full_backup_workflow() {
    // This test requires valid .env configuration
    // Skip if environment is not configured
    if std::env::var("ZIP_ENCRYPTION_PASSWORD").is_err() {
        eprintln!("Skipping integration test: ZIP_ENCRYPTION_PASSWORD not set");
        return;
    }
    
    // Create temporary directories
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    let output_dir = temp_dir.path().join("output");
    
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");
    std::fs::create_dir(&output_dir).expect("Failed to create output dir");
    
    // Create test files in source
    std::fs::write(source_dir.join("file1.txt"), "test content 1").expect("Failed to create file1");
    std::fs::write(source_dir.join("file2.txt"), "test content 2").expect("Failed to create file2");
    
    let subdir = source_dir.join("subdir");
    std::fs::create_dir(&subdir).expect("Failed to create subdir");
    std::fs::write(subdir.join("file3.txt"), "test content 3").expect("Failed to create file3");
    
    // Create config file
    let config_path = temp_dir.path().join("test-config.json");
    let config_json = format!(r#"{{
        "version": "1.0",
        "backup": {{
            "name": "test-backup",
            "source": "{}",
            "output": "{}",
            "output_filename": "{{name}}-{{timestamp}}.zip",
            "exclude_patterns": []
        }},
        "storage": {{
            "bucket": "test-bucket",
            "key_prefix": "test/"
        }}
    }}"#, 
        source_dir.to_string_lossy().replace('\\', "\\\\"),
        output_dir.to_string_lossy().replace('\\', "\\\\")
    );
    
    std::fs::write(&config_path, config_json).expect("Failed to write config");
    
    // Note: This test only validates config loading and state building
    // Full upload test would require mock S3 server or actual B2 credentials
    
    println!("✓ Integration test setup successful");
    println!("  Source: {}", source_dir.display());
    println!("  Output: {}", output_dir.display());
    println!("  Config: {}", config_path.display());
    
    // TODO: Add actual backup execution when main logic is refactored
    // into testable functions
}

#[test]
fn test_change_detection_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = temp_dir.path().join("source");
    std::fs::create_dir(&source_dir).expect("Failed to create source dir");
    
    // Create initial file
    let file_path = source_dir.join("test.txt");
    std::fs::write(&file_path, "initial content").expect("Failed to create file");
    
    // TODO: Build initial state, save it, modify file, rebuild state, detect changes
    
    println!("✓ Change detection integration test placeholder");
}
```

**Note**: Full end-to-end test with upload requires either:
- Mock S3 server (e.g., `moto` or `localstack`)
- Separate test B2 bucket
- Or just test up to ZIP creation

**Acceptance Criteria**:
- [ ] Test creates temporary directories
- [ ] Test creates sample files
- [ ] Test validates config loading
- [ ] Test validates state building
- [ ] Tests pass with `cargo test`

**Effort**: 2 hours

**Dependencies**: Phases 2-4 completed

---

#### Task 5.5: Add Manual Testing Checklist

**Location**: `TESTING.md` (new file)

**Content**:
```markdown
# BlazeBackup Manual Testing Checklist

## Prerequisites
- [ ] Valid `.env` file with Backblaze B2 credentials
- [ ] Valid `backup-config.json` file

## Test Scenarios

### Scenario 1: First Backup (No Previous State)
1. [ ] Delete `.backup-state.json` if it exists
2. [ ] Run `cargo run --release`
3. [ ] Verify output shows "No previous backup state found"
4. [ ] Verify ZIP file created in output directory
5. [ ] Verify ZIP file uploaded to B2 bucket
6. [ ] Verify `.backup-state.json` created
7. [ ] Verify state file contains all source files

### Scenario 2: No Changes (Skip Backup)
1. [ ] Run `cargo run --release` again immediately
2. [ ] Verify output shows "No changes detected"
3. [ ] Verify backup was skipped
4. [ ] Verify no new ZIP file created
5. [ ] Verify no upload occurred

### Scenario 3: File Modified
1. [ ] Modify a file in source directory
2. [ ] Run `cargo run --release`
3. [ ] Verify output shows which file changed
4. [ ] Verify new backup created and uploaded
5. [ ] Verify state file updated

### Scenario 4: New File Added
1. [ ] Add a new file to source directory
2. [ ] Run `cargo run --release`
3. [ ] Verify output shows new file detected
4. [ ] Verify new backup created
5. [ ] Verify new file included in state

### Scenario 5: File Deleted
1. [ ] Delete a file from source directory
2. [ ] Run `cargo run --release`
3. [ ] Verify output shows file deleted
4. [ ] Verify new backup created
5. [ ] Verify deleted file removed from state

### Scenario 6: Cross-Platform Paths (Windows)
1. [ ] Test with Windows path: `C:\Users\...`
2. [ ] Test with forward slashes: `C:/Users/...`
3. [ ] Test with home directory: `~\Documents`
4. [ ] Verify all path formats work correctly

### Scenario 7: Cross-Platform Paths (Linux)
1. [ ] Test with absolute path: `/home/user/...`
2. [ ] Test with home directory: `~/Documents`
3. [ ] Verify all path formats work correctly

### Scenario 8: Error Handling
1. [ ] Test with missing `.env` file
2. [ ] Test with missing config file
3. [ ] Test with invalid JSON in config
4. [ ] Test with non-existent source directory
5. [ ] Test with invalid B2 credentials
6. [ ] Verify all errors have helpful messages

### Scenario 9: Large Directory
1. [ ] Test with directory containing 1000+ files
2. [ ] Verify backup completes successfully
3. [ ] Verify state file contains all files
4. [ ] Measure performance

### Scenario 10: Special Characters in Filenames
1. [ ] Create files with spaces: `file name.txt`
2. [ ] Create files with special chars: `file-2024.txt`
3. [ ] Verify backup includes all files correctly
```

**Acceptance Criteria**:
- [ ] Checklist covers all major scenarios
- [ ] Each scenario has clear pass/fail criteria
- [ ] Includes both happy path and error cases

**Effort**: 30 minutes

**Dependencies**: None (can be created anytime)

---

### Phase 6 Tasks (Detailed)

#### Task 6.1: Create README.md

**Location**: `README.md` (new file)

**Content Structure**:
```markdown
# BlazeBackup

A simple, reliable backup utility for compressing directories and uploading to Backblaze B2 cloud storage.

## Features

- **ZIP Compression** with AES-256 encryption
- **Change Detection** - Only backup when files change
- **Cross-Platform** - Works on Windows and Linux
- **Backblaze B2** - Direct upload via S3-compatible API
- **Simple Configuration** - JSON config file + environment variables

## Installation

### Prerequisites
- Rust toolchain (1.70 or later)
- Backblaze B2 account with application key

### Build from Source
```bash
git clone https://github.com/your-username/blazebackup.git
cd blazebackup
cargo build --release
```

The compiled binary will be at `target/release/blazebackup`.

## Configuration

### 1. Environment Variables (.env)

Copy `example.env` to `.env` and fill in your credentials:

```bash
cp example.env .env
```

Edit `.env`:
```bash
# Backblaze B2 credentials
AWS_ACCESS_KEY_ID=your_key_id_here
AWS_SECRET_ACCESS_KEY=your_secret_key_here
AWS_ENDPOINT_URL=https://s3.eu-central-003.backblazeb2.com
AWS_REGION=eu-central-003

# Bucket name
BUCKET_NAME=your-bucket-name

# ZIP encryption password
ZIP_ENCRYPTION_PASSWORD=your-secure-password
```

### 2. Backup Configuration (backup-config.json)

Create `backup-config.json`:

```json
{
  "version": "1.0",
  "backup": {
    "name": "my-documents",
    "source": "~/Documents",
    "output": "~/Backups",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": []
  },
  "storage": {
    "bucket": "my-backup-bucket",
    "key_prefix": "backups/"
  }
}
```

**Configuration Fields**:
- `name` - Backup name (used in filename)
- `source` - Directory to backup (supports `~` for home)
- `output` - Where to save ZIP files locally
- `output_filename` - Template with `{name}` and `{timestamp}` placeholders
- `bucket` - B2 bucket name
- `key_prefix` - Prefix for S3 keys (optional)

## Usage

Run the backup:
```bash
./target/release/blazebackup
```

Or with cargo:
```bash
cargo run --release
```

### First Run
On first run, BlazeBackup will:
1. Scan source directory
2. Create ZIP archive with encryption
3. Upload to Backblaze B2
4. Save state file (`.backup-state.json`)

### Subsequent Runs
On subsequent runs:
1. Scan source directory
2. Compare with previous state
3. If changes detected → backup and upload
4. If no changes → skip backup

## Change Detection

BlazeBackup tracks:
- File size
- Last modified time

Changes detected:
- ✅ New files
- ✅ Modified files (size or time changed)
- ✅ Deleted files

State saved in `.backup-state.json` (JSON format).

## Security

- ZIP files encrypted with **AES-256**
- Password stored in `.env` (not committed to git)
- B2 credentials in `.env` (not committed to git)
- Always use strong passwords (16+ characters recommended)

## Troubleshooting

### Error: "Source directory does not exist"
- Check `source` path in `backup-config.json`
- Ensure directory exists and is readable

### Error: "Failed to read BUCKET_NAME"
- Check `.env` file exists
- Ensure `BUCKET_NAME` is set correctly

### Error: "Failed to upload backup to Backblaze B2"
- Verify B2 credentials are correct
- Check bucket name matches exactly
- Ensure bucket region matches endpoint URL

### Backup not triggering (no changes detected)
- Check if files were actually modified
- Try `touch filename` to update modification time
- Delete `.backup-state.json` to force full backup

## Development

Run tests:
```bash
cargo test
```

Run with debug output:
```bash
RUST_LOG=debug cargo run
```

Format code:
```bash
cargo fmt
```

Lint code:
```bash
cargo clippy
```

## License

MIT License - see LICENSE file

## Contributing

Contributions welcome! Please open an issue first to discuss changes.
```

**Acceptance Criteria**:
- [ ] Installation instructions clear
- [ ] Configuration examples provided
- [ ] Usage section covers common scenarios
- [ ] Troubleshooting section addresses likely issues
- [ ] Security notes included

**Effort**: 1 hour

**Dependencies**: All phases completed (for accurate documentation)

---

#### Task 6.2: Update Inline Code Comments

**Location**: `src/main.rs` throughout

**Guidelines**:
- Add tutorial-style comments explaining **why** and **how**
- Explain Rust concepts for learners
- Add section headers with visual separators

**Example Additions**:
```rust
// ============================================================================
// Configuration Loading
// ============================================================================

// We use serde to deserialize JSON directly into Rust structs.
// This gives us type safety - the compiler ensures our config
// matches the expected structure.
let config: BackupConfig = serde_json::from_str(&content)
    .context("Failed to parse config file. Check JSON syntax.")?;

// The ? operator is Rust's error propagation shorthand.
// If an error occurs, it returns early with that error.
// Otherwise, it unwraps the Ok value and continues.

// ============================================================================
// Change Detection
// ============================================================================

// We track file metadata (size and modified time) rather than
// hashing file contents. This is much faster for large files
// and sufficient for detecting changes in most cases.
// Trade-off: Won't detect changes if both size and time are
// somehow identical (very rare in practice).
```

**Acceptance Criteria**:
- [ ] Every major function has explanation comment
- [ ] Complex logic explained in detail
- [ ] Rust concepts explained for learners
- [ ] Section headers added with visual separators
- [ ] Comments follow tutorial style from AGENTS.md

**Effort**: 1 hour

**Dependencies**: All implementation phases completed

---

#### Task 6.3: Document Configuration Schema

**Location**: `CONFIG.md` (new file)

**Content**:
```markdown
# Configuration Reference

## backup-config.json Schema

### Top Level
```json
{
  "version": "1.0",          // Schema version (required)
  "backup": { ... },         // Backup settings (required)
  "storage": { ... }         // Storage settings (required)
}
```

### backup Section
```json
"backup": {
  "name": "my-backup",                      // Backup name (required)
  "source": "~/Documents",                  // Source directory (required)
  "output": "~/Backups",                    // Output directory (required)
  "output_filename": "{name}-{timestamp}.zip",  // Filename template (required)
  "exclude_patterns": []                    // Exclusion patterns (optional, not implemented in MVP)
}
```

**Field Details**:

#### `name` (string, required)
- Short name for this backup
- Used in output filename
- Example: `"documents"`, `"photos"`, `"code-projects"`

#### `source` (string, required)
- Directory to backup
- Supports home directory expansion (`~`)
- Must exist and be readable
- Examples:
  - Windows: `"C:/Users/YourName/Documents"`
  - Linux: `"/home/username/documents"`
  - Cross-platform: `"~/Documents"`

#### `output` (string, required)
- Where to save ZIP files locally
- Created automatically if doesn't exist
- Supports home directory expansion (`~`)
- Examples:
  - Windows: `"D:/Backups"`
  - Linux: `"/home/username/backups"`
  - Cross-platform: `"~/Backups"`

#### `output_filename` (string, required)
- Template for ZIP filename
- Placeholders:
  - `{name}` - Replaced with backup name
  - `{timestamp}` - Replaced with current timestamp (format: YYYYMMDD-HHMMSS)
- Example: `"{name}-{timestamp}.zip"` → `"documents-20240115-143022.zip"`

#### `exclude_patterns` (array of strings, optional)
- **Not implemented in MVP** - logged as warning
- Future feature for excluding files/directories
- Example: `["*.tmp", "node_modules/", ".git/"]`

### storage Section
```json
"storage": {
  "bucket": "my-backup-bucket",    // B2 bucket name (required)
  "key_prefix": "backups/"         // S3 key prefix (optional)
}
```

**Field Details**:

#### `bucket` (string, required)
- Backblaze B2 bucket name
- Must match exactly (case-sensitive)
- Must exist before running backup
- Example: `"radoslawg-backups"`

#### `key_prefix` (string, optional)
- Prefix for S3 object keys
- Useful for organizing backups in bucket
- Defaults to empty string (root of bucket)
- Must end with `/` if specified
- Examples:
  - `"backups/"` → uploads to `backups/` folder
  - `"2024/january/"` → uploads to `2024/january/` folder
  - `""` → uploads to root of bucket

## Environment Variables (.env)

### Required Variables

#### `AWS_ACCESS_KEY_ID`
- Backblaze B2 application key ID
- Obtain from B2 console: App Keys → Create Key
- Example: `"00347363e86939b0000000002"`

#### `AWS_SECRET_ACCESS_KEY`
- Backblaze B2 application key secret
- Shown only once when creating key
- Example: `"K003Iy7O5Fv5R4DS8mowSD4kjvfo8KI"`

#### `AWS_ENDPOINT_URL`
- B2 S3-compatible endpoint
- Region-specific
- Format: `https://s3.<region>.backblazeb2.com`
- Examples:
  - EU Central: `"https://s3.eu-central-003.backblazeb2.com"`
  - US West: `"https://s3.us-west-004.backblazeb2.com"`

#### `AWS_REGION`
- B2 region code
- Must match endpoint URL
- Examples: `"eu-central-003"`, `"us-west-004"`

#### `BUCKET_NAME`
- B2 bucket name
- Must match `storage.bucket` in config
- Example: `"radoslawg-backups"`

#### `ZIP_ENCRYPTION_PASSWORD`
- Password for AES-256 encryption
- Use strong password (16+ characters recommended)
- **Keep secure** - don't commit to git
- Example: `"MyS3cur3P@ssw0rd!2024"`

### Optional Variables

#### `CONFIG_FILE_PATH`
- Path to config JSON file
- Defaults to `"backup-config.json"` in current directory
- Example: `"/etc/blazebackup/config.json"`

#### `STATE_FILE_PATH`
- Path to state file for change detection
- Defaults to `".backup-state.json"` in current directory
- Example: `"~/.blazebackup-state.json"`

## Path Handling

### Cross-Platform Paths
BlazeBackup normalizes paths for cross-platform compatibility:
- Accepts both `/` and `\` separators
- Converts to platform-native separator internally
- Use `PathBuf` internally for all path operations

### Home Directory Expansion
- `~` expands to user's home directory
- Works on both Windows and Linux
- Windows: Uses `USERPROFILE` environment variable
- Linux: Uses `HOME` environment variable

### Relative vs Absolute Paths
- Absolute paths: Recommended for reliability
  - Windows: `"C:/Users/Name/Documents"`
  - Linux: `"/home/username/documents"`
- Relative paths: Resolved from current working directory
  - Example: `"./backups"` → relative to where you run the command
- Home paths: Most portable
  - Example: `"~/Documents"` → works on both platforms
```

**Acceptance Criteria**:
- [ ] All config fields documented
- [ ] Examples provided for each field
- [ ] Environment variables explained
- [ ] Path handling rules documented

**Effort**: 45 minutes

**Dependencies**: Phase 2 completed

---

#### Task 6.4: Create example.backup-config.json

**Location**: `example.backup-config.json` (new file)

**Content**:
```json
{
  "version": "1.0",
  "backup": {
    "name": "my-documents",
    "source": "~/Documents",
    "output": "~/Backups",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": []
  },
  "storage": {
    "bucket": "your-bucket-name-here",
    "key_prefix": "backups/"
  }
}
```

**With Comments** (in accompanying `example.backup-config.md`):
```markdown
# Example Backup Configuration

Copy this file to `backup-config.json` and customize:

```bash
cp example.backup-config.json backup-config.json
```

Then edit `backup-config.json` with your values:
- Replace `your-bucket-name-here` with your actual B2 bucket
- Change source/output paths to match your directories
- Customize backup name as desired
```

**Acceptance Criteria**:
- [ ] Example file created
- [ ] All fields have sensible placeholder values
- [ ] Instructions provided for usage

**Effort**: 15 minutes

**Dependencies**: Task 6.3 (for reference)

---

## 7. File Structure After Implementation

```
blazebackup/
├── src/
│   └── main.rs (~500 lines)          # Main application with all logic
├── tests/
│   └── integration_test.rs (~100 lines)  # End-to-end tests
├── Cargo.toml                         # Dependencies (updated)
├── Cargo.lock                         # Locked versions
├── .env                               # Environment config (git-ignored, updated)
├── example.env                        # Template (updated)
├── backup-config.json                 # User's config (git-ignored)
├── example.backup-config.json         # Config template (NEW)
├── .backup-state.json                 # Change detection state (git-ignored, auto-generated)
├── .gitignore                         # Ignore .env, *.json, *.zip, etc.
├── README.md                          # User documentation (NEW)
├── CONFIG.md                          # Configuration reference (NEW)
├── TESTING.md                         # Manual test checklist (NEW)
├── PLAN.md                            # This implementation plan
├── Tutorial.md                        # Development tutorial (existing)
├── GEMINI.md                          # AI assistant guidelines (existing)
└── AGENTS.md                          # Project standards (existing)
```

## 8. Testing Strategy

### Unit Tests (`src/main.rs`)

**Test Coverage**:
- [ ] Configuration loading (valid, invalid, missing fields)
- [ ] Path normalization (home expansion, cross-platform)
- [ ] Change detection (first run, no changes, modified, new, deleted)
- [ ] State serialization/deserialization
- [ ] Error handling edge cases

**Location**: `#[cfg(test)] mod tests` at end of `main.rs`

**Run**: `cargo test --lib`

### Integration Tests (`tests/integration_test.rs`)

**Test Scenarios**:
- [ ] Full workflow with temp directories
- [ ] Config file parsing end-to-end
- [ ] State file creation and loading
- [ ] Change detection with real filesystem

**Run**: `cargo test --test integration_test`

### Manual Testing Checklist (`TESTING.md`)

**Critical Scenarios**:
1. First backup (no previous state)
2. No changes (skip backup)
3. File modified (size change)
4. File modified (time change only)
5. New file added
6. File deleted
7. Multiple changes
8. Large directory (1000+ files)
9. Windows paths (C:\, forward/backward slashes, ~)
10. Linux paths (/, ~)
11. Error cases (missing config, invalid JSON, bad credentials)

### Performance Testing

**Benchmarks** (optional, post-MVP):
- [ ] Time to scan directory with 10,000 files
- [ ] ZIP creation time for 1GB directory
- [ ] Upload speed for 100MB file
- [ ] State file size for various directory sizes

### Platform Testing

**Windows**:
- [ ] Run on Windows 10/11
- [ ] Test Windows-specific paths
- [ ] Verify path separators handled correctly

**Linux**:
- [ ] Run on Linux (Ubuntu/Debian/Fedora)
- [ ] Test Linux-specific paths
- [ ] Verify permissions handled correctly

**WSL** (optional):
- [ ] Test in WSL environment
- [ ] Cross-filesystem paths

### CI/CD Testing (Post-MVP)

**GitHub Actions Workflow**:
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

## 9. Configuration File Examples

### backup-config.json (Basic)

```json
{
  "version": "1.0",
  "backup": {
    "name": "documents",
    "source": "~/Documents",
    "output": "~/Backups",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": []
  },
  "storage": {
    "bucket": "my-backups",
    "key_prefix": ""
  }
}
```

### backup-config.json (Advanced)

```json
{
  "version": "1.0",
  "backup": {
    "name": "code-projects",
    "source": "~/Development",
    "output": "~/Backups/code",
    "output_filename": "{name}-{timestamp}.zip",
    "exclude_patterns": [
      "node_modules/",
      "target/",
      ".git/",
      "*.tmp"
    ]
  },
  "storage": {
    "bucket": "radoslawg-code-backups",
    "key_prefix": "development/2024/"
  }
}
```

**Note**: `exclude_patterns` will be logged as warning in MVP but not enforced.

### Updated .env

```bash
# Backblaze B2 Credentials
AWS_ACCESS_KEY_ID=00347363e86939b0000000002
AWS_SECRET_ACCESS_KEY=K003Iy7O5Fv5R4DS8mowSD4kjvfo8KI

# Bucket Configuration
BUCKET_NAME=radoslawg-Backups

# S3 Endpoint for Backblaze B2
AWS_ENDPOINT_URL=https://s3.eu-central-003.backblazeb2.com
AWS_REGION=eu-central-003

# ZIP Encryption (NEW)
ZIP_ENCRYPTION_PASSWORD=MySecurePassword123!

# Optional Configuration Paths
# CONFIG_FILE_PATH=backup-config.json
# STATE_FILE_PATH=.backup-state.json
```

### Updated example.env

```bash
# Backblaze B2 Credentials
# Get these from: https://secure.backblaze.com/app_keys.htm
AWS_ACCESS_KEY_ID=your_application_key_id_here
AWS_SECRET_ACCESS_KEY=your_application_key_here

# Bucket Configuration
BUCKET_NAME=your-bucket-name

# S3 Endpoint for Backblaze B2
# Find your endpoint at: https://www.backblaze.com/b2/docs/s3_compatible_api.html
AWS_ENDPOINT_URL=https://s3.eu-central-003.backblazeb2.com
AWS_REGION=eu-central-003

# ZIP Encryption Password
# Use a strong password (16+ characters recommended)
ZIP_ENCRYPTION_PASSWORD=replace-with-secure-password

# Optional: Custom configuration file path
# CONFIG_FILE_PATH=backup-config.json

# Optional: Custom state file path
# STATE_FILE_PATH=.backup-state.json
```

### .backup-state.json (Auto-Generated)

```json
{
  "version": "1.0",
  "last_backup": "2024-01-15T14:30:22Z",
  "last_backup_file": "documents-20240115-143022.zip",
  "files": {
    "README.md": {
      "size": 1234,
      "modified": "2024-01-15T10:00:00Z"
    },
    "src/main.rs": {
      "size": 15678,
      "modified": "2024-01-15T14:25:00Z"
    },
    "Cargo.toml": {
      "size": 456,
      "modified": "2024-01-10T09:00:00Z"
    }
  }
}
```

## 10. Success Criteria

### Functional Requirements
- [ ] Loads configuration from external JSON file
- [ ] Reads encryption password from `.env`
- [ ] Detects changes since last backup
- [ ] Skips backup if no changes
- [ ] Creates ZIP with AES-256 encryption
- [ ] Uploads to Backblaze B2
- [ ] Saves state file after successful backup
- [ ] Works on Windows
- [ ] Works on Linux
- [ ] Handles home directory expansion (`~`)

### Code Quality
- [ ] No hardcoded values (paths, passwords)
- [ ] No `.unwrap()` or `.expect()` in production code
- [ ] All errors have descriptive context
- [ ] `cargo fmt` passes (no formatting changes needed)
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo build` passes with zero warnings
- [ ] Tutorial-style comments throughout

### Testing
- [ ] Unit tests for config loading
- [ ] Unit tests for change detection
- [ ] Unit tests for path handling
- [ ] Integration test passes
- [ ] Manual testing checklist completed
- [ ] Tested on Windows
- [ ] Tested on Linux (or WSL)

### Documentation
- [ ] README.md with clear instructions
- [ ] CONFIG.md with all fields documented
- [ ] TESTING.md with manual test cases
- [ ] Inline comments explain complex logic
- [ ] example.env updated with new variables
- [ ] example.backup-config.json provided

### Performance
- [ ] Backup completes in reasonable time (< 5 min for 10GB)
- [ ] Change detection fast (< 10 sec for 10,000 files)
- [ ] Memory usage reasonable (< 500MB for large backups)

### User Experience
- [ ] Clear error messages when something fails
- [ ] Informative output during execution
- [ ] Easy configuration (just edit two files)
- [ ] No crashes or panics in normal operation

## 11. Risk Assessment

### Technical Risks

**Risk 1: Large File Memory Usage**
- **Issue**: Reading entire file into memory for upload
- **Impact**: HIGH - Could cause OOM for very large files
- **Likelihood**: MEDIUM
- **Mitigation**: Current implementation loads entire file. For MVP acceptable if documented. Post-MVP: Stream upload with `aws-sdk-s3` streaming APIs.

**Risk 2: Change Detection False Negatives**
- **Issue**: Metadata-only comparison might miss changes
- **Impact**: MEDIUM - Could skip backup when shouldn't
- **Likelihood**: LOW (rare for size+time to be identical after change)
- **Mitigation**: Document limitation. Post-MVP: Add optional SHA-256 hashing.

**Risk 3: ZIP Creation Fails Mid-Process**
- **Issue**: Partial ZIP file created, state not updated
- **Impact**: MEDIUM - Disk space wasted
- **Likelihood**: LOW
- **Mitigation**: Use `?` operator to return early on error. State only saved after successful upload.

**Risk 4: Upload Fails After ZIP Creation**
- **Issue**: ZIP created locally but upload fails
- **Impact**: LOW - ZIP file remains, can retry
- **Likelihood**: MEDIUM (network issues common)
- **Mitigation**: State not saved until after upload. Next run will retry.

**Risk 5: State File Corrupted**
- **Issue**: Invalid JSON in state file
- **Impact**: MEDIUM - Change detection broken
- **Likelihood**: LOW
- **Mitigation**: Graceful error handling - delete state file and full backup.

### Platform Risks

**Risk 6: Windows Path Handling**
- **Issue**: Backslashes vs forward slashes
- **Impact**: HIGH - Could break on Windows
- **Likelihood**: MEDIUM (already identified in code)
- **Mitigation**: Use `PathBuf` throughout (Phase 4). Comprehensive testing on Windows.

**Risk 7: Linux Permissions**
- **Issue**: Source directory not readable
- **Impact**: MEDIUM - Backup fails
- **Likelihood**: MEDIUM
- **Mitigation**: Clear error message with permission check.

**Risk 8: Cross-Platform Timestamp Format**
- **Issue**: Different time representations
- **Impact**: LOW - Change detection inconsistent
- **Likelihood**: LOW
- **Mitigation**: Use ISO 8601 (RFC 3339) format everywhere via `chrono`.

### Dependency Risks

**Risk 9: AWS SDK Breaking Changes**
- **Issue**: SDK update breaks API
- **Impact**: HIGH - Upload stops working
- **Likelihood**: LOW (stable API)
- **Mitigation**: Lock `Cargo.lock`, test before updating dependencies.

**Risk 10: Zipoxide or Zip Crate Issues**
- **Issue**: Bug in ZIP creation or encryption
- **Impact**: HIGH - Backups corrupted
- **Likelihood**: LOW (mature libraries)
- **Mitigation**: Test ZIP integrity manually. Post-MVP: Add automated verification.

### Operational Risks

**Risk 11: User Misconfiguration**
- **Issue**: Wrong bucket name, bad credentials, invalid paths
- **Impact**: MEDIUM - Backup fails
- **Likelihood**: HIGH (common user error)
- **Mitigation**: Comprehensive error messages with hints. Validation in `load_config()`.

**Risk 12: Insufficient Disk Space**
- **Issue**: Not enough space for ZIP file
- **Impact**: MEDIUM - Backup fails
- **Likelihood**: MEDIUM
- **Mitigation**: Pre-MVP: Fail with clear error. Post-MVP: Check available space before backup.

## 12. Timeline Estimate

### Phase 1: Critical Cleanup
- Task 1.1-1.6: **1-2 hours**
- **Milestone**: Clean, professional codebase with no warnings

### Phase 2: Configuration System
- Task 2.1-2.8: **3-4 hours**
- **Milestone**: Externalized configuration, no hardcoded values

### Phase 3: Change Detection
- Task 3.1-3.7: **4-5 hours**
- **Milestone**: Working change detection, state file management

### Phase 4: Cross-Platform Support
- Task 4.1-4.5: **2-3 hours**
- **Milestone**: Works correctly on both Windows and Linux

### Phase 5: Testing
- Task 5.1-5.5: **4-6 hours**
- **Milestone**: Comprehensive test coverage, passing tests

### Phase 6: Documentation
- Task 6.1-6.4: **2-3 hours**
- **Milestone**: Complete user and developer documentation

### **Total Estimated Time: 16-24 hours**

### Breakdown by Developer Experience

**Experienced Rust Developer** (familiar with ecosystem):
- Phase 1: 1 hour
- Phase 2: 3 hours
- Phase 3: 4 hours
- Phase 4: 2 hours
- Phase 5: 4 hours
- Phase 6: 2 hours
- **Total: ~16 hours**

**Intermediate Developer** (some Rust experience):
- Phase 1: 1.5 hours
- Phase 2: 4 hours
- Phase 3: 5 hours
- Phase 4: 3 hours
- Phase 5: 5 hours
- Phase 6: 2.5 hours
- **Total: ~21 hours**

**Beginner Rust Developer** (learning as going):
- Phase 1: 2 hours
- Phase 2: 5 hours
- Phase 3: 6 hours
- Phase 4: 4 hours
- Phase 5: 6 hours
- Phase 6: 3 hours
- **Total: ~26 hours**

### Critical Path
1. Phase 1 (cleanup) → Phase 2 (config) → Phase 3 (change detection)
2. Phase 4 (cross-platform) can partially overlap with Phase 3
3. Phase 5 (testing) requires Phases 1-4 complete
4. Phase 6 (docs) requires all phases complete

### Recommended Schedule

**Week 1** (10-12 hours):
- Day 1: Phase 1 (cleanup) - 1-2 hours
- Day 2: Phase 2 (config) - 3-4 hours
- Day 3: Phase 3 start (change detection) - 2-3 hours
- Day 4: Phase 3 finish - 2-3 hours

**Week 2** (6-12 hours):
- Day 5: Phase 4 (cross-platform) - 2-3 hours
- Day 6: Phase 5 (testing) - 4-6 hours
- Day 7: Phase 6 (documentation) - 2-3 hours

## 13. Future Enhancements (Post-MVP)

### Explicitly Deferred Features

#### 1. Exclude Patterns
- **Feature**: Honor `exclude_patterns` in config
- **Complexity**: MEDIUM
- **Effort**: 4-6 hours
- **Use Case**: Skip `node_modules/`, `.git/`, `target/` directories

#### 2. Incremental Backups
- **Feature**: Only backup changed files (not full archive)
- **Complexity**: HIGH
- **Effort**: 20+ hours
- **Use Case**: Faster backups, less storage for large datasets

#### 3. Backup Rotation/Retention
- **Feature**: Auto-delete old backups based on policy
- **Complexity**: MEDIUM
- **Effort**: 8-12 hours
- **Use Case**: Keep last N backups, or backups from last M days

#### 4. CLI Arguments (clap)
- **Feature**: Override config via command-line flags
- **Complexity**: LOW
- **Effort**: 2-3 hours
- **Use Case**: `blazebackup --config other.json --force`

#### 5. Multiple Backup Jobs
- **Feature**: Support multiple backup configs in one run
- **Complexity**: MEDIUM
- **Effort**: 6-8 hours
- **Use Case**: Backup Documents, Photos, and Code in one command

#### 6. Restore Functionality
- **Feature**: Download and extract backup from B2
- **Complexity**: MEDIUM
- **Effort**: 8-10 hours
- **Use Case**: Disaster recovery, file restoration

#### 7. Backup Verification
- **Feature**: Verify ZIP integrity, compare checksums
- **Complexity**: MEDIUM
- **Effort**: 4-6 hours
- **Use Case**: Ensure backups aren't corrupted

#### 8. SHA-256 Hashing for Change Detection
- **Feature**: Hash file contents instead of metadata
- **Complexity**: LOW
- **Effort**: 3-4 hours
- **Use Case**: More accurate change detection (slower)

#### 9. Compression Level Options
- **Feature**: Configure ZIP compression level (speed vs size)
- **Complexity**: LOW
- **Effort**: 1-2 hours
- **Use Case**: Faster backups with less compression

#### 10. Progress Bar (indicatif)
- **Feature**: Visual progress during backup/upload
- **Complexity**: LOW
- **Effort**: 2-3 hours
- **Use Case**: Better UX for large backups

#### 11. Logging to File
- **Feature**: Write logs to file in addition to stdout
- **Complexity**: LOW
- **Effort**: 2-3 hours
- **Use Case**: Debugging, audit trail

#### 12. Scheduled Execution (cron/Windows Task Scheduler)
- **Feature**: Auto-run backups on schedule
- **Complexity**: MEDIUM (platform-specific)
- **Effort**: 6-8 hours
- **Use Case**: Automated daily/weekly backups

#### 13. Email Notifications
- **Feature**: Send email on backup success/failure
- **Complexity**: MEDIUM
- **Effort**: 4-6 hours
- **Use Case**: Monitor backups without manual checking

#### 14. Multiple Storage Backends
- **Feature**: Support AWS S3, Google Cloud Storage, Azure Blob
- **Complexity**: HIGH
- **Effort**: 12-16 hours
- **Use Case**: Flexibility in cloud provider choice

#### 15. Differential Backups
- **Feature**: Only backup changes since last full backup
- **Complexity**: VERY HIGH
- **Effort**: 30+ hours
- **Use Case**: Enterprise-grade backup solution

## 14. Decision Log

### Decision 1: Monolithic vs Modular Structure
- **Date**: Planning phase
- **Decision**: Keep `main.rs` monolithic for MVP
- **Rationale**: 
  - Code size manageable (~500 lines)
  - Easier to understand for learners
  - Follows educational style from AGENTS.md
  - Can refactor post-MVP if grows beyond 1000 lines
- **Trade-offs**: Less modularity, harder to test in isolation
- **Alternatives Considered**: Split into `config.rs`, `backup.rs`, `upload.rs`, `state.rs` modules

### Decision 2: Metadata vs Hash-Based Change Detection
- **Date**: Planning phase
- **Decision**: Use metadata (size + modified time) for MVP
- **Rationale**:
  - Much faster (no need to read file contents)
  - Sufficient for 99%+ of real-world use cases
  - Lower resource usage
  - Simpler implementation
- **Trade-offs**: Won't detect changes if size and time identical (extremely rare)
- **Alternatives Considered**: SHA-256 hashing (deferred to post-MVP enhancement)

### Decision 3: Configuration in JSON vs TOML vs YAML
- **Date**: Planning phase
- **Decision**: Use JSON for configuration file
- **Rationale**:
  - `serde_json` already in dependencies
  - Widely understood format
  - Good IDE support
  - Sufficient for MVP schema
- **Trade-offs**: No comments support in JSON (use example.backup-config.md for docs)
- **Alternatives Considered**: TOML (more human-friendly but requires new dependency), YAML (too complex)

### Decision 4: State File Format
- **Date**: Planning phase
- **Decision**: Use JSON for state file (`.backup-state.json`)
- **Rationale**:
  - Human-readable (useful for debugging)
  - Easy to manually delete if corrupted
  - `serde_json` provides pretty-printing
  - Consistent with config file format
- **Trade-offs**: Slightly larger than binary format, slower to parse (negligible for MVP)
- **Alternatives Considered**: Binary format (faster but not human-readable), SQLite (overkill for MVP)

### Decision 5: Error Handling Strategy
- **Date**: Planning phase
- **Decision**: Use `anyhow` throughout with descriptive contexts
- **Rationale**:
  - Already in dependencies
  - Simpler than custom error types for MVP
  - Good error chains with `.context()`
  - Easy to add helpful user-facing messages
- **Trade-offs**: Less type-safe than custom error enums
- **Alternatives Considered**: `thiserror` (more boilerplate for MVP), custom error types (overkill)

### Decision 6: Upload Entire File vs Streaming
- **Date**: Planning phase
- **Decision**: Load entire file into memory for MVP
- **Rationale**:
  - Simpler implementation
  - Acceptable for typical backup sizes (<10GB)
  - Can upgrade to streaming post-MVP if needed
- **Trade-offs**: Memory usage scales with file size, could OOM on very large files
- **Alternatives Considered**: Streaming upload (deferred to post-MVP)
- **Mitigation**: Document limitation in README

### Decision 7: State Saved Before or After Upload
- **Date**: Planning phase
- **Decision**: Save state AFTER successful upload
- **Rationale**:
  - Ensures state only updated on full success
  - If upload fails, next run will retry
  - Prevents false "already backed up" state
- **Trade-offs**: If state save fails after upload, next run will re-upload (rare, acceptable)
- **Alternatives Considered**: Save before upload (rejected - could lose data if upload fails)

### Decision 8: Skip Backup on No Changes
- **Date**: Planning phase
- **Decision**: Skip both ZIP creation and upload if no changes
- **Rationale**:
  - Saves time, bandwidth, storage
  - Core value proposition of change detection
  - User can force by deleting state file
- **Trade-offs**: No way to force backup via CLI (post-MVP: add --force flag)
- **Alternatives Considered**: Always create ZIP but skip upload (rejected - wastes local disk)

### Decision 9: Timestamp Format
- **Date**: Planning phase
- **Decision**: ISO 8601 (RFC 3339) for state file, custom format for filenames
- **Rationale**:
  - ISO 8601 is standard, unambiguous, cross-platform
  - Custom format (YYYYMMDD-HHMMSS) is filesystem-safe and sortable
  - `chrono` crate provides both
- **Trade-offs**: Two different formats (one for state, one for filenames)
- **Alternatives Considered**: Unix timestamp (not human-readable), single format for both (filename wouldn't be sortable)

### Decision 10: Handle Exclude Patterns in MVP
- **Date**: Planning phase
- **Decision**: Parse but don't enforce, log warning
- **Rationale**:
  - Allows config schema to include field for future
  - Doesn't block MVP with complex pattern matching
  - User gets clear feedback that feature not implemented
- **Trade-offs**: User might expect it to work
- **Alternatives Considered**: Remove from schema entirely (rejected - would require schema migration later)

### Decision 11: Test on Linux
- **Date**: Planning phase
- **Decision**: Linux testing optional for MVP, can use WSL
- **Rationale**:
  - Primary development on Windows
  - PathBuf handles cross-platform automatically
  - WSL sufficient for basic validation
  - Full Linux testing post-MVP
- **Trade-offs**: Might miss Linux-specific edge cases
- **Alternatives Considered**: Require Linux VM (too much overhead for MVP)

### Decision 12: Dependencies - Add vs Avoid
- **Date**: Planning phase
- **Decision**: Only add `walkdir` and `chrono`, use existing everything else
- **Rationale**:
  - Follows AGENTS.md guideline: "minimal dependencies"
  - `walkdir` is standard for directory traversal
  - `chrono` is standard for timestamps
  - No need for `clap`, `env_logger`, `indicatif`, etc. in MVP
- **Trade-offs**: Less polished UX (no progress bars, no CLI flags)
- **Alternatives Considered**: Add `clap` for CLI (deferred), `indicatif` for progress (deferred)

---

## End of Plan

**Total Document Length**: ~4,500 words

**Plan Status**: ✅ COMPLETE

**Ready for Implementation**: YES

**Next Steps**:
1. Review this plan with stakeholders
2. Prioritize any adjustments
3. Begin Phase 1 implementation
4. Follow task order as outlined
5. Update plan if significant deviations needed

**Maintenance**:
- Update plan if design decisions change during implementation
- Add "Actual Effort" column after task completion
- Document any issues encountered for future reference
