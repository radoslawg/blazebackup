use anyhow::{Context, Result};
use std::path::Path;

use tokio::{fs::File, io::AsyncReadExt};

#[derive(Debug, serde::Deserialize)]
pub struct BackupConfig {
    pub backups: Vec<BackupSettings>,
    storage: StorageSettings,
}

#[derive(Debug, serde::Deserialize)]
pub struct BackupSettings {
    name: String,
    pub sources: Vec<String>,
    pub output: String,
    output_filename: String,
}

#[derive(Debug, serde::Deserialize)]
struct StorageSettings {
    bucket: String,
    #[serde(default)]
    key_prefix: String,
}

impl BackupSettings {
    pub fn output_filename(&self) -> String {
        self.output_filename.clone()
    }
}

pub async fn load_config(path: &Path) -> Result<BackupConfig> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("Failed to open config file: {}", path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    serde_json::from_str::<BackupConfig>(&content).with_context(|| "Failed to parse config.json")
}

/// Load configuration from a file in the same directory as the executable
pub async fn load_config_from_file() -> Result<BackupConfig> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;

    // Construct path to config.json in the same directory
    let config_path = exe_path
        .parent()
        .context("Failed to get executable directory")?
        .join("config.json");

    load_config(&config_path).await
}
