use std::path::Path;

use anyhow::{Context, Result};
use aws_sdk_s3::{
    primitives::ByteStream,
    types::{Bucket, CompletedMultipartUpload, CompletedPart},
};
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
    log::debug!("Uploading: {}", destination);
    let file_size = tokio::fs::File::open(filename)
        .await?
        .metadata()
        .await?
        .len();

    if file_size > MIN_CHUNK_SIZE as u64 {
        upload_to_bucket_multipart(filepath, &configuration.bucket, &destination).await?;
    } else {
        upload_to_bucket(filepath, &configuration.bucket, &destination).await?;
    }
    Ok(())
}

const MIN_CHUNK_SIZE: usize = 20 * 1024 * 1024; // 5 MB is minimum chunksize required by Backblaze                                            

// https://github.com/awsdocs/aws-doc-sdk-examples/blob/main/rustv1/examples/s3/src/bin/s3-multipart-upload.rs#L48
async fn upload_to_bucket_multipart(
    filename: &Path,
    bucket_name: &String,
    destination: &String,
) -> Result<()> {
    log::debug!("Uploading with multipart: {}", destination);
    let config = aws_config::from_env().load().await;
    let client = aws_sdk_s3::Client::new(&config);

    let multipart_upload = client
        .create_multipart_upload()
        .key(destination)
        .bucket(bucket_name)
        .send()
        .await?;

    let upload_id = multipart_upload.upload_id().context("No upload ID")?;

    let mut file = tokio::fs::File::open(filename).await?;
    let file_size = file.metadata().await?.len();
    let mut completed_parts = Vec::new();
    let mut part_number = 1;
    let mut uploaded_bytes = 0;

    while uploaded_bytes < file_size {
        let mut buffer = Vec::new();
        let mut total_bytes_read = 0;
        while total_bytes_read < MIN_CHUNK_SIZE {
            let mut temp_buffer = vec![0u8; MIN_CHUNK_SIZE];
            let bytes_read = file.read(&mut temp_buffer).await?;
            if bytes_read == 0 {
                break;
            }

            if bytes_read < MIN_CHUNK_SIZE {
                temp_buffer.truncate(bytes_read);
            }
            total_bytes_read += bytes_read;
            buffer.extend_from_slice(&temp_buffer);
        }
        if total_bytes_read == 0 {
            break;
        }

        let stream = ByteStream::from(buffer);

        log::debug!(
            "Uploading part {} ({} bytes) for {}",
            part_number,
            total_bytes_read,
            destination
        );

        let upload_part = client
            .upload_part()
            .key(destination)
            .bucket(bucket_name)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await?;

        completed_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part.e_tag().unwrap_or_default())
                .part_number(part_number)
                .build(),
        );

        uploaded_bytes += total_bytes_read as u64;
        part_number += 1;
    }
    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    client
        .complete_multipart_upload()
        .key(destination)
        .bucket(bucket_name)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;

    Ok(())
}

async fn upload_to_bucket(
    filename: &Path,
    bucket_name: &String,
    destination: &String,
) -> Result<()> {
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

    client
        .put_object()
        .bucket(bucket_name)
        .key(destination)
        .body(buffer.into())
        .send()
        .await?;
    Ok(())
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
            log::debug!("Bucket '{}' exists.", bucket_name);
            Ok(b.clone())
        }
        None => {
            anyhow::bail!("Bucket '{}' does not exist.", bucket_name);
        }
    }
}
