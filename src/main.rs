use crate::config::load_config;
use anyhow::Result;
use dotenv::dotenv;
use sevenz_rust2::{self, encoder_options};
use std::path::PathBuf;
use tokio;
use tokio::task::JoinSet;

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = load_config().await?;

    let zip_password = match std::env::var("ZIP_PASSWORD") {
        Ok(pass) => String::from(pass.trim()),
        Err(_) => String::from(""),
    };

    let mut tasks = JoinSet::new();
    for b in config.backups {
        let mut dest = PathBuf::from(&b.output);
        dest.push(b.output_filename());
        let dest = dest.into_os_string().into_string().unwrap();
        let sources = b.sources;
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
