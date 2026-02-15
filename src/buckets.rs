use std::path::Path;

use anyhow::{Context, Result};
use aws_sdk_s3::types::Bucket;
use tokio::{fs::File, io::AsyncReadExt};

use crate::config::StorageSettings;

pub async fn upload_file(filepath: &Path, configuration: &StorageSettings) -> Result<()> {
    get_bucket(&configuration.bucket)
        .await
        .context("Cannot find bucket")?;
    let filename = filepath.file_name().context("Cannot get filename")?;
    let destination = format!(
        "{}/{}",
        configuration.key_prefix,
        String::from(filename.to_str().context("Cannot convert to str")?)
    );
    println!("Uploading: {}", destination);
    upload_to_bucket(&filepath, &configuration.bucket, &destination).await;
    return Ok(());
}

async fn upload_to_bucket(filename: &Path, bucket_name: &String, destination: &String) {
    // Load the AWS configuration.
    // `aws_config::from_env()` builds a configuration based on environment variables
    // (e.g., AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    let config = aws_config::from_env().load().await;
    let client = aws_sdk_s3::Client::new(&config);

    let mut buffer = Vec::new();
    {
        let mut file = File::open(filename).await.expect("Ok");
        file.read_to_end(&mut buffer).await.expect("Read file ok");
    }

    match client
        .put_object()
        .bucket(bucket_name)
        .key(destination)
        .body(buffer.into())
        .send()
        .await
    {
        Ok(_) => println!("Upload successfull to bucket: {:?}", bucket_name),
        Err(err) => eprintln!("Error uploading object: {:?}", err),
    };
}

async fn get_bucket(
    bucket_name: &String, // The name of the bucket to search for.
) -> Result<Bucket> {
    let config = aws_config::from_env().load().await;
    let aws_client = aws_sdk_s3::Client::new(&config);
    let buckets = aws_client.list_buckets().send().await?.buckets.unwrap();
    let bucket = buckets
        .iter()
        .find(|b| b.name.as_deref().unwrap_or_default() == bucket_name);

    match bucket {
        Some(b) => {
            println!("Bucket '{}' exists.", bucket_name);
            return Ok(b.clone());
        }
        None => {
            anyhow::bail!("Bucket '{}' does not exist.", bucket_name);
        }
    }
}
