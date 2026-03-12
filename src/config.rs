use anyhow::{Context, Result, bail};
use chrono::Local;
use glob::MatchOptions;
use std::path::{Path, PathBuf};

use tokio::{fs::File, io::AsyncReadExt};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BackupConfig {
    pub backups: Vec<BackupSettings>,
    pub storage: StorageSettings,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BackupSettings {
    pub name: String,
    pub sources: Vec<String>,
    output_filename: String,
    exclude: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct StorageSettings {
    pub bucket: String,
    #[serde(default)]
    pub key_prefix: String,
}

impl BackupSettings {
    pub fn output_filename(&self, output_dir: &Path, suffix: Option<String>) -> Result<PathBuf> {
        if !output_dir.exists() || self.output_filename.is_empty() {
            bail!("Cannot create Path! output or output_filename is empty!");
        }
        let output_filename = if let Some(s) = suffix {
            let path = Path::new(&self.output_filename);
            let stem = path
                .file_stem()
                .context("Cannot get file stem")?
                .to_string_lossy();
            let extension = path
                .extension()
                .context("Missing extension")?
                .to_string_lossy();
            format!("{stem}_{s}.{extension}")
        } else {
            self.output_filename.clone()
        };

        let temp_filename = output_filename.replace("{name}", &self.name).replace(
            "{timestamp}",
            Local::now().format("%Y%m%d-%H%M%S").to_string().as_str(),
        );

        Ok(output_dir.join(temp_filename))
    }

    pub fn is_excluded(&self, path: &str) -> Result<bool> {
        if self.exclude.is_none() {
            return Ok(false);
        }

        let excludes = self.exclude.as_ref().unwrap();
        for exclude in excludes {
            let result = glob::Pattern::new(exclude)
                .context("Invalid glob pattern")?
                .matches_with(path, MatchOptions::default());
            if result {
                return Ok(result);
            }
        }
        Ok(false)
    }
}

async fn load_config_from_file(path: &Path) -> Result<BackupConfig> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("Failed to open config file: {}", path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    yaml_serde::from_str::<BackupConfig>(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

pub async fn _load_json_config_from_file(path: &Path) -> Result<BackupConfig> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("Failed to open config file: {}", path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    serde_json::from_str::<BackupConfig>(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

/// Load configuration from a file in the same directory as the executable
pub async fn load_config() -> Result<BackupConfig> {
    // Get the path to the current executable
    // let exe_path = std::env::current_exe().context("Failed to get executable path")?;

    let config_path = match std::env::var("BLAZEBACKUP_CONFIG") {
        Ok(file) => PathBuf::from(file),
        Err(_) => {
            let home_path = std::env::home_dir().context("Failed to find Home dir")?;

            // Construct path to config.json in the same directory
            home_path
                .join(".config")
                .join("blazebackup")
                .join("config.yaml")
        }
    };

    load_config_from_file(&config_path).await
}

pub async fn _save_as_yaml(config: &BackupConfig) -> Result<()> {
    let yaml = yaml_serde::to_string(config)?;
    log::debug!("Saving config to YAML:\n{}", yaml);
    bail!("This method is just for conversion!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_excluded_none() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup.zip".to_string(),
            exclude: None,
        };
        assert!(!settings.is_excluded("any/path").unwrap());
    }

    #[test]
    fn test_is_excluded_one_pattern() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup.zip".to_string(),
            exclude: Some(vec![String::from("an*")]),
        };
        assert!(settings.is_excluded("any/path").unwrap());
    }

    #[test]
    fn test_is_excluded_two_patterns() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup.zip".to_string(),
            exclude: Some(vec![String::from("**/*an*"), String::from("**/foo*.*")]),
        };
        assert!(settings.is_excluded("any/path").unwrap());
        assert!(!settings.is_excluded("bar/margharita/foo12").unwrap()); // that's linux thingy
        assert!(settings.is_excluded("bar/margharita/frani").unwrap());
        assert!(!settings.is_excluded("bar/margharita/frost").unwrap());
        assert!(settings.is_excluded("bar/foo12.12").unwrap());
    }

    #[test]
    fn test_is_excluded_everything_from_directory() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup.zip".to_string(),
            exclude: Some(vec![String::from("**/any/**")]),
        };
        assert!(settings.is_excluded("any/path").unwrap());
        assert!(!settings.is_excluded("any").unwrap());
        assert!(settings.is_excluded("bar/any/").unwrap());
        assert!(
            settings
                .is_excluded("/bar/any/bar/margharita/foo12")
                .unwrap()
        );
        assert!(!settings.is_excluded("/bar").unwrap());
    }
    #[test]
    fn test_is_excluded_not_excluded_pattern() {
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup.zip".to_string(),
            exclude: Some(vec![String::from("foo*")]),
        };
        assert!(!settings.is_excluded("any/path").unwrap());
    }

    #[test]
    fn test_output_filename_normal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec!["src".to_string()],

            output_filename: "backup.zip".to_string(),
            exclude: None,
        };
        assert_eq!(
            settings
                .output_filename(temp_dir.path(), None)
                .unwrap()
                .parent()
                .unwrap(),
            temp_dir.path()
        );
        assert_eq!(
            settings
                .output_filename(temp_dir.path(), None)
                .unwrap()
                .file_name()
                .unwrap(),
            "backup.zip"
        );
    }

    #[test]
    fn test_output_filename_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "".to_string(),
            exclude: None,
        };
        assert!(settings.output_filename(temp_dir.path(), None).is_err());
    }

    #[test]
    fn test_output_filename_special_characters() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup_2024-01-15@14:30:00.zip".to_string(),
            exclude: None,
        };
        assert_eq!(
            settings
                .output_filename(temp_dir.path(), None)
                .unwrap()
                .parent()
                .unwrap(),
            temp_dir.path()
        );
        assert_eq!(
            settings
                .output_filename(temp_dir.path(), None)
                .unwrap()
                .file_name()
                .unwrap(),
            "backup_2024-01-15@14:30:00.zip"
        );
    }

    #[test]
    fn test_output_filename_expansion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "testname".to_string(),
            sources: vec![],

            output_filename: "backup_{name}_{timestamp}.7z".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        let filename = result.file_name().unwrap().to_str().unwrap();
        // Verify pattern: backup_testname_YYYYMMDD-HHMMSS.7z
        assert!(filename.starts_with("backup_testname_"));
        assert!(filename.ends_with(".7z"));
        // Extract and validate the timestamp portion
        let prefix_len = "backup_testname_".len();
        let suffix_len = ".7z".len();
        let timestamp_part = &filename[prefix_len..filename.len() - suffix_len];
        // Should be exactly 15 chars: YYYYMMDD-HHMMSS
        assert_eq!(timestamp_part.len(), 15);
        assert_eq!(timestamp_part.chars().nth(8), Some('-')); // Check hyphen position
        // Parse to ensure it's a real date
        let _ = chrono::NaiveDateTime::parse_from_str(timestamp_part, "%Y%m%d-%H%M%S")
            .expect("Timestamp should be parseable");
    }

    #[test]
    fn test_output_filename_name_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "mybackup".to_string(),
            sources: vec![],

            output_filename: "{name}.zip".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        assert_eq!(result.file_name().unwrap(), "mybackup.zip");
    }

    #[test]
    fn test_output_filename_timestamp_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "test".to_string(),
            sources: vec![],

            output_filename: "backup_{timestamp}.zip".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        let filename = result.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("backup_"));
        assert!(filename.ends_with(".zip"));
        // Extract and validate the timestamp portion
        let prefix_len = "backup_".len();
        let suffix_len = ".zip".len();
        let timestamp_part = &filename[prefix_len..filename.len() - suffix_len];
        assert_eq!(timestamp_part.len(), 15);
        assert_eq!(timestamp_part.chars().nth(8), Some('-'));
    }

    #[test]
    fn test_output_filename_multiple_placeholders() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "prod".to_string(),
            sources: vec![],

            output_filename: "{name}_{name}_{timestamp}_{name}.zip".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        let filename = result.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("prod_prod_"));
        assert!(filename.ends_with("_prod.zip"));
    }

    #[test]
    fn test_output_filename_empty_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "".to_string(),
            sources: vec![],

            output_filename: "{name}_{timestamp}.zip".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        let filename = result.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with('_')); // Empty name results in leading underscore
    }

    #[test]
    fn test_output_filename_special_chars_in_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings = BackupSettings {
            name: "my-backup_v1.2".to_string(),
            sources: vec![],

            output_filename: "{name}.zip".to_string(),
            exclude: None,
        };
        let result = settings.output_filename(temp_dir.path(), None).unwrap();
        assert_eq!(result.file_name().unwrap(), "my-backup_v1.2.zip");
    }

    #[tokio::test]
    async fn test_load_config_valid() {
        let yaml = r#"
backups:
  - name: test_backup
    sources:
      - /home/user/docs
    output_filename: docs.zip
storage:
  bucket: my-bucket
  key_prefix: backups/
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = load_config_from_file(temp_file.path()).await.unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        assert_eq!(config.backups.len(), 1);
        assert_eq!(config.backups[0].name, "test_backup");
        assert_eq!(
            config.backups[0]
                .output_filename(temp_dir.path(), None)
                .unwrap()
                .file_name()
                .unwrap(),
            "docs.zip"
        );
    }

    #[tokio::test]
    async fn test_load_config_missing_file() {
        let nonexistent = Path::new("/nonexistent/path/config.json");
        let result = load_config_from_file(nonexistent).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to open config file"));
    }

    #[tokio::test]
    async fn test_load_config_malformed_json() {
        let yaml = r#"invalid: yaml: ["#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config_from_file(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse"));
    }

    #[tokio::test]
    async fn test_load_config_missing_required_field() {
        let yaml = r#"
backups:
  - name: test_backup
    sources: ["/home/user/docs"]
storage:
  bucket: my-bucket
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config_from_file(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse"));
    }

    #[tokio::test]
    async fn test_load_config_empty_json() {
        let yaml = r#""#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config_from_file(temp_file.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse"));
    }

    #[tokio::test]
    async fn test_load_config_multiple_backups() {
        let yaml = r#"
backups:
  - name: backup1
    sources: ["/path/1"]
    output_filename: file1.zip
  - name: backup2
    sources: ["/path/2a", "/path/2b"]
    output_filename: file2.zip
storage:
  bucket: bucket-name
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = load_config_from_file(temp_file.path()).await.unwrap();
        assert_eq!(config.backups.len(), 2);
        assert_eq!(config.backups[0].name, "backup1");
        assert_eq!(config.backups[1].name, "backup2");
        assert_eq!(config.backups[1].sources.len(), 2);
    }
}
