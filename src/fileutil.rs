use anyhow::Context;
use anyhow::Result;
use simplehash::Fnv1aHasher64;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{collections::HashMap, hash::Hasher};
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn compress_sources(destination: &Path, sources: &[String], password: &str) -> Result<()> {
    log::info!("Compressing: {:?}", destination);
    let output = File::create(destination)?;
    let mut archive = ZipWriter::new(output);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::ZSTD)
        .compression_level(Some(9))
        .with_aes_encryption(zip::AesMode::Aes256, password);

    for s in sources {
        let path = Path::new(s);
        archive.start_file(s, options)?;
        let mut f = File::open(path)?;

        std::io::copy(&mut f, &mut archive)?;
    }
    archive.finish()?;
    Ok(())
}

pub fn _calculate_directory_hash(paths: &[String]) -> Result<String> {
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

pub fn _calculate_files_hash(paths: &[String]) -> Result<HashMap<String, String>> {
    calculate_files_hash_exclusion(paths, |_| Ok(false))
}

pub fn calculate_files_hash_exclusion<F>(
    paths: &[String],
    is_excluded: F,
) -> Result<HashMap<String, String>>
where
    F: Fn(&str) -> Result<bool>,
{
    let mut result = HashMap::new();

    let mut sorted_paths = Vec::from(paths);
    sorted_paths.sort();

    for p in sorted_paths {
        let walkdir = WalkDir::new(&p);
        for entry in walkdir.sort_by(|a, b| a.path().cmp(b.path())) {
            let mut hasher = Fnv1aHasher64::new();
            let uentry =
                entry.context("Failed to access directory entry during hash calculation")?;
            if uentry.file_type().is_file() {
                if is_excluded(uentry.path().to_str().context("Cannot get path string")?)? {
                    continue;
                }
                let metadata = uentry.metadata()?;
                let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                let file_size = metadata.len();

                hasher.write(&modified.to_ne_bytes());
                hasher.write(&file_size.to_ne_bytes());
                let absolute_path = fs::canonicalize(uentry.path())
                    .context("Failed to get absolute path for hash calculation")?;
                hasher.write(absolute_path.to_string_lossy().as_bytes());
                result.insert(
                    String::from(
                        uentry
                            .path()
                            .to_str()
                            .context("Cannot convert path to string")?,
                    ),
                    format!("{:x}", hasher.finish_raw()),
                );
            }
        }
    }
    Ok(result)
}

pub fn _get_changed_files(
    paths: &[String],
    hashes: &HashMap<String, String>,
) -> Result<(Option<Vec<String>>, Option<Vec<String>>)> {
    get_changed_files_exclusion(paths, hashes, |_| Ok(false))
}

pub fn get_changed_files_exclusion<F>(
    paths: &[String],
    hashes: &HashMap<String, String>,
    is_excluded: F,
) -> Result<(Option<Vec<String>>, Option<Vec<String>>)>
where
    F: Fn(&str) -> Result<bool>,
{
    let mut _changed = Vec::new();
    let mut _deleted = Vec::new();
    let mut traced_map = convert_hashmap(hashes);

    for p in paths {
        for entry in WalkDir::new(p) {
            let mut hasher = Fnv1aHasher64::new();
            let uentry =
                entry.context("Failed to access directory entry during hash calculation")?;
            if _changed.contains(&String::from(
                uentry
                    .path()
                    .to_str()
                    .context("Cannot convert path to String")?,
            )) {
                continue;
            }
            if uentry.file_type().is_file() {
                if is_excluded(uentry.path().to_str().context("Cannot get path string")?)? {
                    continue;
                }
                let metadata = uentry.metadata()?;
                let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                let file_size = metadata.len();

                hasher.write(&modified.to_ne_bytes());
                hasher.write(&file_size.to_ne_bytes());
                let absolute_path = fs::canonicalize(uentry.path())
                    .context("Failed to get absolute path for hash calculation")?;
                hasher.write(absolute_path.to_string_lossy().as_bytes());
                let filename = String::from(
                    uentry
                        .path()
                        .to_str()
                        .context("Cannot convert path to String")?,
                );
                match traced_map.get_mut(&filename) {
                    Some(h) => {
                        if h.0 != format!("{:x}", hasher.finish_raw()) {
                            _changed.push(String::from(
                                uentry
                                    .path()
                                    .to_str()
                                    .context("Cannot convert path to String")?,
                            ));
                        }
                        h.1 = true;
                    }
                    None => _changed.push(String::from(
                        uentry
                            .path()
                            .to_str()
                            .context("Cannot convert path to String")?,
                    )),
                }
            }
        }
    }
    for (filename, (_, was_found)) in traced_map {
        if !was_found {
            _deleted.push(filename);
        }
    }
    let changed: Option<Vec<String>>;
    if _changed.is_empty() {
        changed = None;
    } else {
        changed = Some(_changed);
    }
    let deleted: Option<Vec<String>>;
    if _deleted.is_empty() {
        deleted = None;
    } else {
        deleted = Some(_deleted);
    }
    Ok((changed, deleted))
}

fn convert_hashmap(original_map: &HashMap<String, String>) -> HashMap<String, (String, bool)> {
    let mut new_map = HashMap::new();
    for (key, value) in original_map {
        new_map.insert(key.clone(), (value.clone(), false));
    }
    new_map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_calculate_files_hash_basic() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "hello")?;

        let paths = vec![file_path.to_str().unwrap().to_string()];
        let hashes = _calculate_files_hash(&paths)?;

        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key(file_path.to_str().unwrap()));
        Ok(())
    }

    #[test]
    fn test_get_changed_files_no_change() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "hello")?;

        let paths = vec![file_path.to_str().unwrap().to_string()];
        let hashes = _calculate_files_hash(&paths)?;

        let (changed, deleted) = _get_changed_files(&paths, &hashes)?;
        assert!(changed.is_none());
        assert!(deleted.is_none());
        Ok(())
    }

    #[test]
    fn test_get_changed_files_new_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)?;

        let (changed, deleted) =
            _get_changed_files(&[dir.path().to_str().unwrap().to_string()], &HashMap::new())?;
        assert!(changed.is_some());
        assert_eq!(changed.as_ref().unwrap().len(), 1);
        assert!(deleted.is_none());
        Ok(())
    }

    #[test]
    fn test_get_changed_files_deleted() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)?;

        let paths = vec![file_path.to_str().unwrap().to_string()];
        let hashes = _calculate_files_hash(&paths)?;

        fs::remove_file(&file_path)?;

        // Scan the directory that contained the file
        let (changed, deleted) =
            _get_changed_files(&[dir.path().to_str().unwrap().to_string()], &hashes)?;
        assert!(changed.is_none());
        assert!(deleted.is_some());
        assert_eq!(deleted.as_ref().unwrap().len(), 1);
        assert_eq!(deleted.as_ref().unwrap()[0], file_path.to_str().unwrap());
        Ok(())
    }

    #[test]
    fn test_compress_sources_basic() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "hello content")?;

        let archive_path = dir.path().join("test.7z");
        let sources = vec![file_path.to_str().unwrap().to_string()];

        compress_sources(&archive_path, &sources, "password")?;

        assert!(archive_path.exists());
        assert!(fs::metadata(&archive_path)?.len() > 0);
        Ok(())
    }

    // This test exposes a problem: overlapping paths cause duplicates in changed files
    #[test]
    fn test_problem_overlapping_paths_duplicates() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)?;

        let paths = vec![
            dir.path().to_str().unwrap().to_string(),
            file_path.to_str().unwrap().to_string(),
        ];

        // Everything is new
        let (changed, _) = _get_changed_files(&paths, &HashMap::new())?;
        let changed_files = changed.unwrap();

        // This will FAIL if there are duplicates
        assert_eq!(
            changed_files.len(),
            1,
            "Expected 1 changed file, but found duplicates: {:?}",
            changed_files
        );
        Ok(())
    }

    // This test exposes a problem: path representation (./subdir vs subdir) affects hashes
    #[test]
    fn test_problem_relative_path_consistency() -> Result<()> {
        let dir = tempdir()?;
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir)?;
        let file_path = subdir.join("test.txt");
        File::create(&file_path)?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(dir.path())?;

        let hash1 = _calculate_files_hash(&["subdir".to_string()])?;
        let hash2 = _calculate_files_hash(&["./subdir".to_string()])?;

        std::env::set_current_dir(original_dir)?;

        let val1 = hash1.values().next().unwrap();
        let val2 = hash2.values().next().unwrap();

        // This will FAIL because the path itself is part of the hash
        assert_eq!(
            val1, val2,
            "Hashes should be independent of how the root path was specified (./subdir vs subdir)"
        );
        Ok(())
    }
}
