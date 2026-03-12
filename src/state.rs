use std::collections::HashMap;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt, io::AsyncWriteExt};

use anyhow::{Context, Result, bail};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct State {
    pub backups: Vec<BackupState>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BackupState {
    pub name: String,
    pub file_hashes: HashMap<String, String>,
    pub deleted_files: Vec<String>,
    pub last_full_backup: String,
}

pub async fn load_state() -> Result<State> {
    let home_path = std::env::home_dir().context("Failed to find Home dir")?;

    let state_path = home_path
        .join(".config")
        .join("blazebackup")
        .join("state.json");

    if !state_path.is_file() {
        bail!("There is no state file!");
    }

    load_state_from_file(&state_path).await
}

async fn load_state_from_file(path: &Path) -> Result<State> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("Failed to open state file: {}", path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .with_context(|| format!("Failed to read state file: {}", path.display()))?;

    serde_json::from_str::<State>(&content).with_context(|| "Failed to parse state.json")
}

impl State {
    pub async fn save_state(&self) -> Result<()> {
        let home_path = std::env::home_dir().context("Failed to find Home dir")?;

        let state_path = home_path
            .join(".config")
            .join("blazebackup")
            .join("state.json");

        self.save_state_to_file(&state_path).await
    }

    async fn save_state_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)
            .await
            .with_context(|| format!("Failed to create state file: {}", path.display()))?;
        let json_out = serde_json::to_string_pretty(&self).context("Cannot serialize State")?;
        file.write_all(json_out.as_bytes())
            .await
            .with_context(|| format!("Failed to write state file: {}", path.display()))
    }
}
