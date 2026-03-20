# BlazeBackup

![Build Workflow](https://git.grzanka.org/radoslawg/blazebackup/actions/workflows/build.yaml/badge.svg)

BlazeBackup is a Rust-based utility for automated, encrypted backups to S3-compatible storage (e.g., Backblaze B2, AWS S3). It compresses specified source directories into archives with AES-256 encryption and uploads them in a parallelized pipeline.

## Features

- **Incremental Backups**: Smart change detection ensures only new or modified files are uploaded, saving bandwidth and storage.
- **Automated Full Backups**: Automatically triggers a full backup after a configurable interval (default: 30 days) to ensure data integrity.
- **Zip Compression (Zstd)**: High-ratio compression using Zstandard (level 9) for smaller archive sizes.
- **AES-256 Encryption**: All archives are protected with AES-256 encryption using a user-defined password.
- **Parallel Pipeline**: Multiple backup sets are processed and uploaded concurrently using an asynchronous tokio-based pipeline.
- **S3-Compatible**: Seamlessly works with any S3-compatible storage provider (AWS S3, Backblaze B2, MinIO, etc.).
- **Deletion Tracking**: Generates a `backblaze_deleted.sh` script within archives to track files deleted since the last backup.
- **Exclude Patterns**: Robust support for glob-style patterns to exclude specific files or directories (e.g., `node_modules`, `target`, `*.tmp`).
- **Flexible Filenaming**: Support for placeholders like `{name}` and `{timestamp}` in output filenames.

## Configuration

The application looks for a configuration file at `~/.config/blazebackup/config.yaml` by default.
You can override this location by setting the `BLAZEBACKUP_CONFIG` environment variable.

The application maintains state (hashes of previous backups, timestamps) at `~/.config/blazebackup/state.json`.

### Example `config.yaml`

```yaml
backups:
  - name: work-projects
    sources:
      - C:/Users/Name/Documents/Work
      - D:/Projects
    exclude:
      - "**/node_modules/**"
      - "**/target/**"
      - "*.tmp"
    output_filename: backup_{name}_{timestamp}.7z
    repeat_full: "30" # Days between full backups. Use "never" for incremental-only.
storage:
  bucket: my-backup-bucket
  key_prefix: daily
```

- `output_filename` supports placeholders:
    - `{name}`: The name defined in the backup settings.
    - `{timestamp}`: Current time in `YYYYMMDD-HHMMSS` format.

## Environment Variables

The following variables should be defined in a `.env` file or your system environment:

- `AWS_ACCESS_KEY_ID`: Your S3 access key.
- `AWS_SECRET_ACCESS_KEY`: Your S3 secret key.
- `AWS_REGION`: The region for your bucket.
- `AWS_ENDPOINT_URL`: The custom endpoint (essential for providers like Backblaze B2).
- `ZIP_PASSWORD`: Password used for AES-256 encryption.
- `COMPRESSION_DIR`: Temporary directory used for compression before upload (defaults to system temp).
- `BLAZEBACKUP_CONFIG`: (Optional) Custom path to the `config.yaml` file.

## Build and Usage

### Prerequisites

- Rust (Edition 2024)

### Build

```bash
# Build for release
cargo build --release
```

### Run

```bash
# Run the application
cargo run
```

### Test

```bash
# Run tests
cargo test
```

