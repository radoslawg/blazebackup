use crate::buckets::upload_file;
use crate::config::load_config;
use crate::state::{BackupState, State, load_state};
use anyhow::{Context, Result};
use dotenv::dotenv;
use sevenz_rust2::encoder_options;
use simplehash::fnv::Fnv1aHasher64;
use std::hash::Hasher;
use std::path::Path;
use std::time::UNIX_EPOCH;
use thiserror::Error;
use tokio::task::JoinSet;
use walkdir::WalkDir;

mod buckets;
mod config;
mod state;

pub fn compress_sources(destination: &Path, sources: Vec<String>, password: String) -> Result<()> {
    println!("Compressing: {:?}", destination);
    let mut writer = sevenz_rust2::ArchiveWriter::create(destination).expect("create writer ok");
    writer.set_content_methods(vec![
        encoder_options::AesEncoderOptions::new(password.as_str().into()).into(),
        encoder_options::Lzma2Options::from_level_mt(9, 32, 16 * 1024 * 1024).into(),
    ]);
    for source in sources {
        writer.push_source_path(source, |_| true).expect("pack ok");
    }
    writer.finish()?;
    Ok(())
}

pub fn calculate_directory_hash(paths: &[String]) -> Result<String> {
    let mut hasher = Fnv1aHasher64::new();

    let mut sorted_paths = Vec::from(paths);
    sorted_paths.sort();

    for p in sorted_paths {
        let walkdir = WalkDir::new(&p);
        for dir in walkdir.sort_by(|a, b| a.path().cmp(b.path())) {
            let udir = dir.context("Failed to access directory entry during hash calculation")?;
            if udir.file_type().is_file() {
                let metadata = udir.metadata()?;
                let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                let file_size = metadata.len();

                hasher.write(&modified.to_ne_bytes());
                hasher.write(&file_size.to_ne_bytes());
                hasher.write(udir.path().as_os_str().as_encoded_bytes());
            }
        }
    }
    Ok(format!("{:x}", hasher.finish_raw()))
}

#[derive(Debug, Error)]
pub enum HashingError {
    #[error("Hash already exists")]
    HashExists,
}

fn update_hash(state: &mut State, backup_name: &String, hash: String) -> Result<(), HashingError> {
    match state.backups.iter_mut().find(|s| &s.name == backup_name) {
        Some(s) => {
            if s.hash == hash {
                return Err(HashingError::HashExists);
            } else {
                s.hash = hash;
                Ok(())
            }
        }
        None => {
            state.backups.push(BackupState {
                name: backup_name.clone(),
                hash: hash,
            });
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = load_config().await.context("Cannot load config")?;
    let mut state = match load_state().await {
        Ok(state) => state,
        Err(_) => State { backups: vec![] },
    };
    println!("{:?}", state);

    let zip_password = match std::env::var("ZIP_PASSWORD") {
        Ok(pass) => String::from(pass.trim()),
        Err(_) => String::from(""),
    };
    let compression_path = std::env::var("COMPRESSION_DIR").unwrap_or(String::from("d:/temp"));

    let mut tasks = JoinSet::new();
    for b in config.backups.iter() {
        let sources = b.sources.clone();
        let backup_hash = calculate_directory_hash(&sources).context("Cannot compute hash")?;

        println!("{}, {}", b.name, backup_hash);
        match update_hash(&mut state, &b.name, backup_hash) {
            Ok(_) => {}
            Err(e) => match e {
                HashingError::HashExists => {
                    println!("Hash exists!");
                    continue;
                }
            },
        }

        state.save_state().await.context("Cannot save state.")?;
        let dest = b
            .output_filename(&compression_path)
            .context("Cannot construct output path!")?;
        let password = zip_password.clone();
        let storage = config.storage.clone();

        tasks.spawn(async move {
            let dest_clone = dest.clone();
            let sources_clone = sources.clone();
            let password_clone = password.clone();

            tokio::task::spawn_blocking(move || {
                compress_sources(dest_clone.as_path(), sources_clone, password_clone)
            })
            .await
            .context("Panic in compression task")?
            .context("Error during compression")?;

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
        let path2 = dir.path().join("file1.txt");

        create_test_file(&PathBuf::from(&path1), "test content", 50)?;

        let hash1 = calculate_directory_hash(&[path1.to_str().unwrap().to_string()])?;
        sleep(Duration::from_millis(50)); // Wait a bit before creating the second hash
        let hash2 = calculate_directory_hash(&[path2.to_str().unwrap().to_string()])?;

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
        let hash1 = calculate_directory_hash(&[path.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(1000)); // We have to wait in worst case one second to make sure
        // modification time is increased.

        let mut file = fs::File::create(&path)?;
        file.write_all(b"initial content")?; // Write the same content, but this updates mtime
        file.flush()?;

        let hash2 = calculate_directory_hash(&[path.to_str().unwrap().to_string()])?;

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
        let hash1 = calculate_directory_hash(&[path.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(50)); // Wait to ensure mtime is different

        // Modify the file, which changes content and size
        let mut file = fs::File::create(&path)?;
        file.write_all(b"much larger content")?;
        file.flush()?;

        let hash2 = calculate_directory_hash(&[path.to_str().unwrap().to_string()])?;

        assert_ne!(hash1, hash2, "Hash should change when file size changes.");
        Ok(())
    }

    #[test]
    fn test_hash_changes_on_path_name() -> Result<()> {
        let dir = tempdir()?;
        let path1 = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");

        create_test_file(&PathBuf::from(&path1), "content", 10)?;
        let hash1 = calculate_directory_hash(&[path1.to_str().unwrap().to_string()])?;

        sleep(Duration::from_millis(50));

        // Rename the file - this tests the path being written to the hasher
        fs::rename(&path1, &path2)?;
        let hash2 = calculate_directory_hash(&[path2.to_str().unwrap().to_string()])?;

        assert_ne!(hash1, hash2, "Hash should change when file path changes.");
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
        let hash1 = calculate_directory_hash(&paths_sorted_input)?;

        // Run hash with paths in reverse order in the input vector
        let paths_reverse_input = vec![
            path_b.to_str().unwrap().to_string(),
            path_a.to_str().unwrap().to_string(),
        ];
        let hash2 = calculate_directory_hash(&paths_reverse_input)?;

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
        create_test_file(&hash_target_dir.join("file_A.txt"), "A", 10)?;
        let hash1 = calculate_directory_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        // File outside the hashed directory structure (in the root temp dir)
        let unrelated_file = root_dir.path().join("unrelated.txt");
        create_test_file(&PathBuf::from(&unrelated_file), "unrelated", 10)?;

        // Hash again without touching the target directory content
        let hash2 = calculate_directory_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        assert_eq!(
            hash1, hash2,
            "Hash should be identical when only an external, untracked file is added to the parent temporary directory."
        );

        // Now modify the file *inside* the hashed directory
        create_test_file(&hash_target_dir.join("file_A.txt"), "A changed", 10)?;
        let hash3 = calculate_directory_hash(&[hash_target_dir.to_str().unwrap().to_string()])?;

        assert_ne!(
            hash1, hash3,
            "Hash must change when content inside the hashed directory changes."
        );
        Ok(())
    }
}
