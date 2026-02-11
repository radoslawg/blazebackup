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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_output_filename_normal() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec!["src".to_string()],
            output: "/tmp".to_string(),
            output_filename: "backup.zip".to_string(),
        };
        assert_eq!(settings.output_filename(), "backup.zip");
    }

    #[test]
    fn test_output_filename_empty() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],
            output: "/tmp".to_string(),
            output_filename: "".to_string(),
        };
        assert_eq!(settings.output_filename(), "");
    }

    #[test]
    fn test_output_filename_special_characters() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],
            output: "/tmp".to_string(),
            output_filename: "backup_2024-01-15@14:30:00.zip".to_string(),
        };
        assert_eq!(settings.output_filename(), "backup_2024-01-15@14:30:00.zip");
    }

    #[tokio::test]
    async fn test_load_config_valid() {
        let json = r#"{
            "backups": [
                {
                    "name": "test_backup",
                    "sources": ["/home/user/docs"],
                    "output": "/tmp",
                    "output_filename": "docs.zip"
                }
            ],
            "storage": {
                "bucket": "my-bucket",
                "key_prefix": "backups/"
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = load_config(temp_file.path()).await.unwrap();
        assert_eq!(config.backups.len(), 1);
        assert_eq!(config.backups[0].name, "test_backup");
        assert_eq!(config.backups[0].output_filename(), "docs.zip");
    }

    #[tokio::test]
    async fn test_load_config_missing_file() {
        let nonexistent = Path::new("/nonexistent/path/config.json");
        let result = load_config(nonexistent).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to open config file"));
    }

    #[tokio::test]
    async fn test_load_config_malformed_json() {
        let json = r#"{ invalid json }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config.json"));
    }

    #[tokio::test]
    async fn test_load_config_missing_required_field() {
        let json = r#"{
            "backups": [
                {
                    "name": "test_backup",
                    "sources": ["/home/user/docs"]
                }
            ],
            "storage": {
                "bucket": "my-bucket"
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config.json"));
    }

    #[tokio::test]
    async fn test_load_config_empty_json() {
        let json = r#"{}"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config.json"));
    }

    #[tokio::test]
    async fn test_load_config_multiple_backups() {
        let json = r#"{
            "backups": [
                {
                    "name": "backup1",
                    "sources": ["/path/1"],
                    "output": "/out1",
                    "output_filename": "file1.zip"
                },
                {
                    "name": "backup2",
                    "sources": ["/path/2a", "/path/2b"],
                    "output": "/out2",
                    "output_filename": "file2.zip"
                }
            ],
            "storage": {
                "bucket": "bucket-name"
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = load_config(temp_file.path()).await.unwrap();
        assert_eq!(config.backups.len(), 2);
        assert_eq!(config.backups[0].name, "backup1");
        assert_eq!(config.backups[1].name, "backup2");
        assert_eq!(config.backups[1].sources.len(), 2);
    }
}
