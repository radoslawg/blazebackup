use crate::config::load_config;
use anyhow::{Context, Result};
use dotenv::dotenv;
use sevenz_rust2::encoder_options;
use simplehash::fnv::Fnv1aHasher64;
use std::hash::Hasher;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tokio::task::JoinSet;
use walkdir::WalkDir;

mod buckets;
mod config;

pub fn compress_sources(destination: String, sources: Vec<String>, password: String) -> Result<()> {
    println!("Compressing: {}", destination);
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
            let udir = dir.context("Failed to access directory entyr during hash calculation")?;
            if udir.file_type().is_file() {
                let modified = udir
                    .metadata()?
                    .modified()?
                    .duration_since(UNIX_EPOCH)?
                    .as_secs();
                hasher.write(&modified.to_ne_bytes());
                hasher.write(udir.path().as_os_str().as_encoded_bytes());
            }
        }
    }
    Ok(format!("{:x}", hasher.finish_raw()))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = load_config().await?;

    let zip_password = match std::env::var("ZIP_PASSWORD") {
        Ok(pass) => String::from(pass.trim()),
        Err(_) => String::from(""),
    };

    let mut tasks = JoinSet::new();
    for b in config.backups.iter() {
        let sources = b.sources.clone();
        println!(
            "source: {:?}, hash: {:?}",
            sources,
            calculate_directory_hash(&sources)
        );
        let mut dest = PathBuf::from(&b.output);
        dest.push(b.output_filename().unwrap());
        let dest = dest.into_os_string().into_string().unwrap();
        let password = zip_password.clone();
        tasks.spawn_blocking(move || compress_sources(dest, sources, password));
    }
    while let Some(res) = tasks.join_next().await {
        res??;
    }

    //let bucket_name = std::env::var("BUCKET_NAME").unwrap();

    //upload_to_bucket(&String::from("d:/projekty.zip"), &bucket_name).await;

    Ok(()) // Indicate successful execution of the main function.
}
