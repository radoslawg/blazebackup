use crate::buckets::upload_file;
use crate::config::load_config;
use crate::fileutil::*;
use crate::state::{BackupState, State, load_state};
use anyhow::{Context, Result};
use dotenv::dotenv;
use simplelog::TermLogger;
use std::path::PathBuf;
use tempfile::tempdir;
use thiserror::Error;
use tokio::task::JoinSet;

mod buckets;
mod config;
mod fileutil;
mod state;

#[derive(Debug, Error)]
pub enum HashingError {
    #[error("Hash already exists")]
    HashExists,
}

enum BackupMode {
    Full,
    Incremental,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Logger
    TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .context("Cannot initilize Logger")?;

    dotenv().ok();
    log::info!("BackBlaze system starting..");

    let config = load_config().await.context("Cannot load config")?;
    let mut state = match load_state().await {
        Ok(state) => state,
        Err(_) => State { backups: vec![] },
    };

    let zip_password = match std::env::var("ZIP_PASSWORD") {
        Ok(pass) => String::from(pass.trim()),
        Err(_) => String::from(""),
    };

    let _tempdir_guard: Option<tempfile::TempDir>;
    let compression_path = match std::env::var("COMPRESSION_DIR") {
        Ok(p) => {
            _tempdir_guard = None;
            PathBuf::from(p)
        }
        Err(_) => {
            let tmp = tempdir().context("Cannot create temporary directory")?;
            let path = tmp.path().to_path_buf();
            _tempdir_guard = Some(tmp);
            path
        }
    };

    let mut tasks = JoinSet::new();
    for b in config.backups.iter() {
        let sources: Vec<String> = b.sources.clone();
        //let backup_hash = calculate_directory_hash(&sources).context("Cannot compute hash")?;

        let changed_files: Option<Vec<String>>;
        let deleted_files: Option<Vec<String>>;
        let mode = match state.backups.iter_mut().find(|s| s.name == b.name) {
            Some(s) => {
                (changed_files, deleted_files) =
                    get_changed_files_exclusion(&sources, &s.file_hashes, |s| b.is_excluded(s))?;
                s.deleted_files = deleted_files.clone().unwrap_or_default();
                log::info!("Incremental Mode for {}", b.name);
                // log::debug!("Changed files: {:?}", changed_files);
                // log::debug!("Deleted files: {:?}", s.deleted_files);
                if changed_files.is_none() {
                    log::info!("{} - No change detected! No processing", b.name);
                    continue;
                }
                BackupMode::Incremental
            }
            None => {
                log::info!("Full mode for {}", b.name);
                let files_hash = calculate_files_hash_exclusion(&sources, |s| b.is_excluded(s))
                    .context("Cannot compute files hashes")?;
                changed_files = Some(files_hash.keys().cloned().collect());
                deleted_files = None;
                state.backups.push(BackupState {
                    name: b.name.clone(),
                    hash: String::from(""),
                    file_hashes: files_hash,
                    deleted_files: vec![],
                });
                BackupMode::Full
            }
        };

        let dest = match mode {
            BackupMode::Incremental => b
                .output_filename(
                    compression_path.as_path(),
                    Some(String::from("Incremental")),
                )
                .context("Cannot construct output path!")?,
            BackupMode::Full => b
                .output_filename(compression_path.as_path(), None)
                .context("Cannot construct output path!")?,
        };

        log::debug!("Destination filename {}", dest.to_str().unwrap_or_default());

        // TODO: Somehow this needs to be more robust and saved only after succesful processing.
        // also, later we enter asynchronous computation so it needs to be even more robust.
        // Maybe store separate state file for each config.backups entry?
        state.save_state().await.context("Cannot save state.")?;

        let password = zip_password.clone();
        let storage = config.storage.clone();

        // Tokio magic
        tasks.spawn(async move {
            let dest_clone = dest.clone();
            let sources = changed_files.context("Changed files shoule not be None here")?;
            let password_clone = password.clone();

            tokio::task::spawn_blocking(move || {
                compress_sources(
                    dest_clone.as_path(),
                    &sources,
                    &password_clone,
                    &deleted_files,
                )
            })
            .await
            .context("Panic in compression task")??;

            upload_file(dest.as_path(), &storage)
                .await
                .context("Error uploading file")?;

            Ok::<(), anyhow::Error>(())
        });
    }

    while let Some(res) = tasks.join_next().await {
        res.context("Task execution failed")??;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write, path::PathBuf, thread::sleep, time::Duration};
    use tempfile::tempdir;

    // Helper function to create a file with known content and timestamp
    fn create_test_file(path: &PathBuf, content: &str, wait_ms: u64) -> Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        // Sleep to ensure modification time is distinct, if necessary for testing
        sleep(Duration::from_millis(wait_ms));
        file.flush()?;
        Ok(())
    }

    #[test]
    fn test_hash_is_deterministic_for_identical_files() -> Result<()> {
        let dir = tempdir()?;
        let path1 = dir.path().join("file1.txt");

        create_test_file(&PathBuf::from(&path1), "test content", 50)?;

        let hash1 = _calculate_files_hash(&[path1.to_str().unwrap().to_string()])?;
        sleep(Duration::from_millis(50)); // Wait a bit before creating the second hash
        let hash2 = _calculate_files_hash(&[path1.to_str().unwrap().to_string()])?;

        assert_eq!(
            hash1, hash2,
            "Hashes should be identical for the same file state."
        );
        Ok(())
    }

    #[test]
    fn test_hash_changes_on_modification_time() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("time_test.txt");

        create_test_file(&PathBuf::from(&path), "initial content", 10)?; // Initial creation
        let hash1 = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(1100)); // Wait to ensure modification time is increased.

        let mut file = fs::File::create(&path)?;
        file.write_all(b"initial content")?; // Write the same content, but this updates mtime
        file.flush()?;

        let hash2 = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        assert_ne!(
            hash1, hash2,
            "Hash should change when modification time changes."
        );
        Ok(())
    }

    #[test]
    fn test_hash_changes_on_file_size() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("size_test.txt");

        create_test_file(&PathBuf::from(&path), "small", 10)?;
        let hash1 = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(50)); // Wait to ensure mtime is different

        // Modify the file, which changes content and size
        let mut file = fs::File::create(&path)?;
        file.write_all(b"much larger content")?;
        file.flush()?;

        let hash2 = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        assert_ne!(hash1, hash2, "Hash should change when file size changes.");
        Ok(())
    }

    #[test]
    fn test_hash_changes_on_path_name() -> Result<()> {
        let dir = tempdir()?;
        let path1 = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");

        create_test_file(&PathBuf::from(&path1), "content", 10)?;
        let hash1 = _calculate_files_hash(&[path1.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(50));

        // Rename the file - this tests the path being written to the hasher
        fs::rename(&path1, &path2)?;
        let hash2 = _calculate_files_hash(&[path2.to_str().unwrap().to_string()])?;

        // We compare the values in the maps
        let h1_val = hash1.values().next().unwrap();
        let h2_val = hash2.values().next().unwrap();

        assert_ne!(h1_val, h2_val, "Hash should change when file path changes.");
        Ok(())
    }

    #[test]
    fn test_hash_is_deterministic_for_multiple_sorted_paths() -> Result<()> {
        let dir = tempdir()?;
        let path_a = dir.path().join("a.txt");
        let path_b = dir.path().join("b.txt");

        // Create files in reverse order of processing
        create_test_file(&PathBuf::from(&path_b), "content b", 50)?;
        sleep(Duration::from_millis(50));
        create_test_file(&PathBuf::from(&path_a), "content a", 50)?;

        // Run hash with paths sorted alphabetically in the input vector
        let paths_sorted_input = vec![
            path_a.to_str().unwrap().to_string(),
            path_b.to_str().unwrap().to_string(),
        ];
        let hash1 = _calculate_files_hash(&paths_sorted_input)?;

        // Run hash with paths in reverse order in the input vector
        let paths_reverse_input = vec![
            path_b.to_str().unwrap().to_string(),
            path_a.to_str().unwrap().to_string(),
        ];
        let hash2 = _calculate_files_hash(&paths_reverse_input)?;

        assert_eq!(
            hash1, hash2,
            "Hashes must be identical because the internal logic sorts the paths."
        );
        Ok(())
    }

    #[test]
    fn test_hash_includes_all_files_in_walked_directory() -> Result<()> {
        let root_dir = tempdir()?;
        let hash_target_dir = root_dir.path().join("target");
        fs::create_dir(&hash_target_dir)?;

        // File that will be included in the hash
        let file_a = hash_target_dir.join("file_A.txt");
        create_test_file(&file_a, "A", 10)?;
        let hash1 = _calculate_files_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        // File outside the hashed directory structure (in the root temp dir)
        let unrelated_file = root_dir.path().join("unrelated.txt");
        create_test_file(&PathBuf::from(&unrelated_file), "unrelated", 10)?;

        // Hash again without touching the target directory content
        let hash2 = _calculate_files_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        assert_eq!(
            hash1, hash2,
            "Hash should be identical when only an external, untracked file is added to the parent temporary directory."
        );

        // Now modify the file *inside* the hashed directory
        create_test_file(&file_a, "A changed", 10)?;
        let hash3 = _calculate_files_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        assert_ne!(
            hash1, hash3,
            "Hash must change when content inside the hashed directory changes."
        );
        Ok(())
    }

    #[test]
    fn test_get_changed_files_detects_changes() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.txt");
        create_test_file(&path, "initial", 10)?;

        let initial_hashes = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(1100));

        // Modify file
        let mut file = fs::File::create(&path)?;
        file.write_all(b"modified")?;
        file.flush()?;

        let (changed, deleted) =
            _get_changed_files(&[path.to_str().unwrap().to_string()], &initial_hashes)?;

        assert!(changed.is_some());
        assert_eq!(changed.unwrap().len(), 1);
        assert!(deleted.is_none());

        Ok(())
    }

    #[test]
    fn test_get_changed_files_detects_deletion() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.txt");
        create_test_file(&path, "initial", 10)?;

        let initial_hashes = _calculate_files_hash(&[path.to_str().unwrap().to_string()])?;

        // Delete file
        fs::remove_file(&path)?;

        let (changed, deleted) =
            _get_changed_files(&[dir.path().to_str().unwrap().to_string()], &initial_hashes)?;

        assert!(changed.is_none());
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().len(), 1);

        Ok(())
    }
}
