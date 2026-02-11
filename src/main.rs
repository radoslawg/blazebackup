use anyhow::{Context, Result};
use aws_sdk_s3::{client, types::Bucket};
use dotenv::dotenv;
use sevenz_rust2::{self, encoder_options};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio;
use tokio::task::JoinSet;

// Import necessary crates

#[derive(Debug, serde::Deserialize)]
struct BackupConfig {
    backups: Vec<BackupSettings>,
    storage: StorageSettings,
}

#[derive(Debug, serde::Deserialize)]
struct BackupSettings {
    name: String,
    sources: Vec<String>,
    output: String,
    output_filename: String,
}

#[derive(Debug, serde::Deserialize)]
struct StorageSettings {
    bucket: String,
    #[serde(default)]
    key_prefix: String,
}

/// Load configuration from a JSON file at the specified path
fn load_config(path: &Path) -> Result<BackupConfig> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open config file: {}", path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    serde_json::from_str::<BackupConfig>(&content).with_context(|| "Failed to parse config.json")
}

/// Load configuration from a file in the same directory as the executable
fn load_config_from_file() -> Result<BackupConfig> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;

    // Construct path to config.json in the same directory
    let config_path = exe_path
        .parent()
        .context("Failed to get executable directory")?
        .join("config.json");

    load_config(&config_path)
}

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

/// The `#[tokio::main]` attribute is used to make the `main` function an asynchronous entry point.
/// `tokio` is a popular asynchronous runtime for Rust, enabling non-blocking I/O operations,
/// which are essential for network requests like interacting with S3.
/// `async fn main()` declares an asynchronous main function.
/// `-> Result<()>` now uses `anyhow::Result` for more flexible error handling.
/// It indicates that the function will either return successfully (`Ok(())`) or
/// return an `anyhow::Error`.
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from the `.env` file.
    // `.ok()` converts the Result into an Option, and if there's an error (e.g., no .env file),
    // it will be ignored, and the program will continue. This is common for optional config files.
    dotenv().ok();

    let test_json = r#"
        {
  "backups": [
  {
    "name": "my-backup",
    "sources": ["c:/bin"],
    "output": "d:/temp",
    "output_filename": "dupa.7z"
  },
  {
    "name": "configs",
    "sources": ["c:/Users/Administrator/.config/"],
    "output": "d:/temp",
    "output_filename": "dupa2.7z"
  },
  {
    "name": "obsidian",
    "sources": ["d:/Obsidian/"],
    "output": "d:/temp",
    "output_filename": "obsidian.7z"
  }
  ],
  "storage": {
    "bucket": "radoslawg-Backups",
    "key_prefix": "backups/"
  }
}
        "#;
    let config = serde_json::from_str::<BackupConfig>(test_json)?;

    let zip_password = match std::env::var("ZIP_PASSWORD") {
        Ok(pass) => String::from(pass.trim()),
        Err(_) => String::from(""),
    };

    let mut tasks = JoinSet::new();
    for b in config.backups {
        let mut dest = PathBuf::from(b.output);
        dest.push(b.output_filename);
        let dest = dest.into_os_string().into_string().unwrap();
        let sources = b.sources;
        let password = zip_password.clone();
        tasks.spawn_blocking(move || compress_sources(dest, sources, password));
    }
    while let Some(res) = tasks.join_next().await {
        res??;
    }

    panic!("Stop, hammer time");
    // for b in config.backups {
    // let mut dest = PathBuf::from(b.output);
    // dest.push(b.output_filename);
    //     compress_sources(
    //         &dest.into_os_string().into_string().unwrap(),
    //         &b.sources,
    //         &zip_password,
    //     )
    //     .await;
    // }

    // sevenz_rust2::compress_to_path_encrypted(
    //     config.backup.source,
    //     "D:/configs.7z",
    //     zip_password.as_str().into(),
    // )
    // .expect("compress ok");

    // Retrieve the `BUCKET_NAME` environment variable.
    // `std::env::var` reads the value of the specified environment variable.
    // `.unwrap()` is used here for simplicity. In a production application,
    // you would want to handle the `Result` returned by `var` more gracefully,
    // perhaps using `anyhow::Context` to add more informative error messages
    // if the variable is not set.
    let bucket_name = std::env::var("BUCKET_NAME").unwrap();

    upload_to_bucket(&String::from("d:/projekty.zip"), &bucket_name).await;

    Ok(()) // Indicate successful execution of the main function.
}

async fn upload_to_bucket(filename: &String, bucket_name: &String) {
    // Load the AWS configuration.
    // `aws_config::from_env()` builds a configuration based on environment variables
    // (e.g., AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    // `.load().await` asynchronously loads this configuration.
    let config = aws_config::from_env().load().await;
    // Create a new S3 client using the loaded configuration.
    // This client will be used to perform S3 operations.
    let client = aws_sdk_s3::Client::new(&config);

    let mut buffer = Vec::new();
    {
        let mut file = std::fs::File::open(filename).expect("Ok");
        file.read_to_end(&mut buffer).expect("Read file ok");
    }

    match client
        .put_object()
        .bucket(bucket_name)
        .key(filename)
        .body(buffer.into())
        .send()
        .await
    {
        Ok(_) => println!("Upload successfull to bucket: {:?}", bucket_name),
        Err(err) => println!("Error uploading object: {:?}", err),
    };
}

/// Asynchronous function to check for the existence of an S3 bucket.
/// It takes an S3 client and the bucket name (both by reference) as input.
/// It returns an `anyhow::Result` which will either be the `Bucket` object if found,
/// or an `anyhow::Error` if the operation fails or the bucket does not exist.
async fn get_bucket(
    aws_client: &client::Client, // The AWS S3 client to use for the API call.
    bucket_name: &String,        // The name of the bucket to search for.
) -> Result<Bucket> {
    // List all buckets accessible by the configured AWS credentials.
    // `.send().await?` executes the request and propagates any errors.
    // `.buckets.unwrap()` unwraps the Option containing the list of buckets.
    // In a real application, you would handle the None case gracefully.
    let buckets = aws_client.list_buckets().send().await?.buckets.unwrap();
    // Iterate through the list of buckets to find the one with the matching name.
    // `.iter()` creates an iterator over the buckets.
    // `.find()` searches for an element that satisfies the given predicate (closure).
    // `|b| b.name.as_deref().unwrap_or_default() == bucket_name` is the predicate:
    // It compares the bucket's name (extracted safely with `as_deref().unwrap_or_default()`)
    // with the `bucket_name` provided to the function.
    let bucket = buckets
        .iter()
        .find(|b| b.name.as_deref().unwrap_or_default() == bucket_name);

    // Use a `match` statement to handle the two possible outcomes of `find`:
    // `Some(b)`: The bucket was found.
    // `None`: The bucket was not found.
    match bucket {
        Some(b) => {
            // If found, print a confirmation message and return the cloned bucket object.
            println!("Bucket '{}' exists.", bucket_name);
            return Ok(b.clone());
        }
        None => {
            // If not found, use `anyhow::bail!` to return an error. This is a more robust
            // way to handle errors than `panic!`, allowing the caller to decide how to
            // handle the missing bucket. In a real application, you might also consider
            // creating the bucket if it doesn't exist, or providing a more specific
            // error handling strategy.
            anyhow::bail!("Bucket '{}' does not exist.", bucket_name);
        }
    }
}
