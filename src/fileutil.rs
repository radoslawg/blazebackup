use anyhow::Context;
use anyhow::Result;
use sevenz_rust2::ArchiveEntry;
use sevenz_rust2::SourceReader;
use simplehash::Fnv1aHasher64;
use std::fs::File;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{collections::HashMap, hash::Hasher};
use walkdir::WalkDir;

use sevenz_rust2::{ArchiveWriter, encoder_options};

pub fn compress_sources(destination: &Path, sources: &[String], password: &str) -> Result<()> {
    log::info!("Compressing: {:?}", destination);

    let mut writer = ArchiveWriter::create(destination)
        .with_context(|| format!("Failed to create archive at {:?}", destination))?;
    writer.set_content_methods(vec![
        encoder_options::AesEncoderOptions::new(password.into()).into(),
        encoder_options::Lzma2Options::from_level_mt(9, 16, 128 * 1024 * 1024).into(),
    ]);

    let mut s = Vec::with_capacity(sources.len());
    let mut r = Vec::with_capacity(sources.len());

    for source in sources {
        let path = Path::new(source);
        let reader = File::open(path).with_context(|| format!("Failed to open file {:?}", path))?;
        let filename = path.to_string_lossy().into_owned();

        s.push(ArchiveEntry::from_path(path, filename));
        r.push(SourceReader::new(reader));
    }

    writer
        .push_archive_entries(s, r)
        .context("Failed to push files to archive")?;
    writer.finish().context("Failed to Finish archive")?;

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

pub fn calculate_files_hash(paths: &[String]) -> Result<HashMap<String, String>> {
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
                let metadata = uentry.metadata()?;
                let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                let file_size = metadata.len();

                hasher.write(&modified.to_ne_bytes());
                hasher.write(&file_size.to_ne_bytes());
                hasher.write(uentry.path().as_os_str().as_encoded_bytes());
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

pub fn get_changed_files(
    paths: &[String],
    hashes: &HashMap<String, String>,
) -> Result<(Option<Vec<String>>, Option<Vec<String>>)> {
    let mut _changed = Vec::new();
    let mut _deleted = Vec::new();
    let mut traced_map = convert_hashmap(hashes);

    for p in paths {
        for entry in WalkDir::new(p) {
            let mut hasher = Fnv1aHasher64::new();
            let uentry =
                entry.context("Failed to access directory entry during hash calculation")?;
            if uentry.file_type().is_file() {
                let metadata = uentry.metadata()?;
                let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                let file_size = metadata.len();

                hasher.write(&modified.to_ne_bytes());
                hasher.write(&file_size.to_ne_bytes());
                hasher.write(uentry.path().as_os_str().as_encoded_bytes());
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
