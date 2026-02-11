use anyhow::Result;
use aws_sdk_s3::{client, types::Bucket};
use tokio::{fs::File, io::AsyncReadExt};

pub async fn upload_to_bucket(filename: &String, bucket_name: &String) {
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
        let mut file = File::open(filename).await.expect("Ok");
        file.read_to_end(&mut buffer).await.expect("Read file ok");
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
pub async fn get_bucket(
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
