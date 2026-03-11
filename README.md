# BlazeBackup

![Build Workflow](https://git.grzanka.org/radoslawg/blazebackup/actions/workflows/build.yaml/badge.svg)

BlazeBackup is a Rust-based utility for automated, encrypted backups to S3-compatible storage (e.g., Backblaze B2, AWS S3). It compresses specified source directories into 7z archives with AES-256 encryption and uploads them in a parallelized pipeline.

## Features

- **Zip Compression**: High-ratio compression using the `zip` crate with Zstandard (Zstd).
- **AES-256 Encryption**: Protects your archives with a password.
- **Parallel Pipeline**: Compresses and uploads multiple backup sets concurrently for maximum efficiency.
- **S3-Compatible**: Works with any S3-compatible storage provider.
- **Smart Change Detection**: Skips backups if source directory contents haven't changed since the last successful run, saving bandwidth and storage.
- **Deterministic Hashing**: Calculates directory hashes to uniquely identify states.

## Configuration

The application looks for a configuration file at `~/.config/blazebackup/config.yaml`.
The application maintains state (hashes of previous backups) at `~/.config/blazebackup/state.json`.

### Example `config.yaml`

```yaml
backups:
  - name: work-projects
    sources:
      - C:/Users/Name/Documents/Work
      - D:/Projects
    output_filename: backup_{name}_{timestamp}.7z
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
- `ZIP_PASSWORD`: Password used for 7z AES encryption.
- `COMPRESSION_DIR`: Temporary directory used for compression before upload (defaults to `d:/temp`).

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

## Future
- [x] Implement state tracking to skip unchanged backups.
- [ ] Implement restore functionality.
- [x] Support partial/incremental uploads (only changed files within a directory).
