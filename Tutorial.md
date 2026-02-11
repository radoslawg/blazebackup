# BlazeBackup Tutorial

Welcome to the BlazeBackup tutorial! This guide will walk you through building a Rust console application that compresses a directory and uploads it to an S3-compatible object storage service like Backblaze B2.

## Step 1: Project Setup and Initial Code

The project is already set up with a basic Rust structure. Our main application logic resides in `src/main.rs`.

### `Cargo.toml`

First, let's look at the `Cargo.toml` file, which manages our project's dependencies and metadata. We'll need to add a few dependencies for S3 interaction, environment variable loading, and asynchronous programming.

Here's what your `Cargo.toml` should look like:

```toml
[package]
name = "blazebackup"
version = "0.1.0"
edition = "2024" # Updated edition

[dependencies]
# Used for asynchronous runtime, essential for network operations.
tokio = { version = "1.49.0", features = ["full"] } # Updated version
# AWS SDK for Rust, specifically for S3 services.
aws-sdk-s3 = "1.119.0" # Updated version
# AWS configuration utilities.
aws-config = { version = "1.8.12", features = ["behavior-version-latest"] } # Updated version and features
# For loading environment variables from a .env file.
dotenv = { version = "0.15.0", features = ["clap", "cli"] } # Added features
```

Make sure these dependencies are present in your `Cargo.toml`. If you're starting from a fresh project, you might need to add them. You can check your `Cargo.toml` by running `cat Cargo.toml`.

### `src/main.rs` - Initial Structure and S3 Bucket Check

Our `src/main.rs` file contains the core logic. It's structured to:
1. Load environment variables.
2. Initialize an AWS S3 client.
3. Check for the existence of a specified S3 bucket.

Let's break down the current `src/main.rs` code:

```rust
//! # BlazeBackup
//!
//! This is the main entry point for the BlazeBackup application.
//! BlazeBackup is a console application written in Rust that
//! compresses a specified directory and uploads it to an AWS S3-compatible
//! object storage service (like Backblaze B2).
//!
//! This file sets up the basic AWS S3 client and retrieves a specified bucket.

// Import necessary crates and modules.
// `aws_sdk_s3` provides the AWS SDK for S3, allowing us to interact with S3-compatible services.
use aws_sdk_s3::{client, types::Bucket};
// `dotenv` is used to load environment variables from a `.env` file.
// This is crucial for securely managing credentials and configuration without hardcoding them.
use dotenv::dotenv;

/// The `#[tokio::main]` attribute is used to make the `main` function an asynchronous entry point.
/// `tokio` is a popular asynchronous runtime for Rust, enabling non-blocking I/O operations,
/// which are essential for network requests like interacting with S3.
/// `async fn main()` declares an asynchronous main function.
/// `-> Result<(), Box<dyn std::error::Error>>` specifies the return type.
/// It indicates that the function will either return successfully (Ok(())) or
/// return an error (`Box<dyn std::error::Error>`), which is a general trait object
/// for any error type.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from the `.env` file.
    // `.ok()` converts the Result into an Option, and if there's an error (e.g., no .env file),
    // it will be ignored, and the program will continue. This is common for optional config files.
    dotenv().ok();

    // Retrieve the `BUCKET_NAME` environment variable.
    // `std::env::var` reads the value of the specified environment variable.
    // `.unwrap()` is used here for simplicity in this example. In a production application,
    // you would want to handle the `Result` returned by `var` more gracefully (e.g., with `expect`
    // or pattern matching) to provide better error messages if the variable is not set.
    let bucket_name = std::env::var("BUCKET_NAME").unwrap();

    // Load the AWS configuration.
    // `aws_config::from_env()` builds a configuration based on environment variables
    // (e.g., AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    // `.load().await` asynchronously loads this configuration.
    let config = aws_config::from_env().load().await;
    // Create a new S3 client using the loaded configuration.
    // This client will be used to perform S3 operations.
    let client = aws_sdk_s3::Client::new(&config);

    // Call the `get_bucket` asynchronous function to check if the specified bucket exists.
    // `await` pauses the execution of `main` until `get_bucket` completes.
    // The `?` operator is a shorthand for error propagation: if `get_bucket` returns an `Err`,
    // it will immediately return from `main` with that error.
    let _bucket = get_bucket(client, bucket_name.clone()).await?;
    // The `_` before `bucket` indicates that the variable is intentionally unused.
    // This suppresses compiler warnings.

    // This line was commented out in the original code, but if uncommented,
    // it would print the name of the found bucket.
    //println!("{}", bucket.unwrap().name.as_deref().unwrap_or_default());

    // Print a simple "Hello, world!" message to the console.
    println!("Hello, world!");
    Ok(()) // Indicate successful execution of the main function.
}

/// Asynchronous function to check for the existence of an S3 bucket.
/// It takes an S3 client and the bucket name as input.
/// It returns a `Result` which will either be the `Bucket` object if found,
/// or an error if the operation fails or the bucket does not exist.
async fn get_bucket(
    aws_client: client::Client, // The AWS S3 client to use for the API call.
    bucket_name: String,        // The name of the bucket to search for.
) -> Result<Bucket, Box<dyn std::error::Error>> {
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
            // If not found, panic! In a real application, you might return an `Err`
            // or create the bucket if it's not critical for it to pre-exist.
            panic!("Bucket '{}' does not exist.", bucket_name)
        },
    }
}

### Environment Variables (`.env`)

For the application to connect to your S3-compatible service (like Backblaze B2), you need to configure environment variables. Create a file named `.env` in the root of your project directory (where `Cargo.toml` is located) with the following content:

```
# .env file for BlazeBackup configuration

# AWS Access Key ID for your S3-compatible service.
# For Backblaze B2, this is your Key ID.
AWS_ACCESS_KEY_ID="your_access_key_id"

# AWS Secret Access Key for your S3-compatible service.
# For Backblaze B2, this is your Application Key.
AWS_SECRET_ACCESS_KEY="your_secret_access_key"

# The S3 region. For Backblaze B2, this is typically "us-east-1" or the region
# corresponding to your S3 endpoint. Check your B2 bucket details for the correct region.
AWS_REGION="us-east-1" # Example, change as per your B2 setup

# The name of the S3 bucket where you want to store backups.
BUCKET_NAME="your-blazebackup-bucket"

# The S3 endpoint URL. This is crucial for Backblaze B2.
# You can find this in your B2 bucket details, it often looks like
# "s3.us-east-001.backblazeb2.com" or similar.
AWS_ENDPOINT_URL="https://s3.us-east-001.backblazeb2.com"
```

**Important:** Replace `"your_access_key_id"`, `"your_secret_access_key"`, `"us-east-1"`, `"your-blazebackup-bucket"`, and `"https://s3.us-east-001.backblazeb2.com"` with your actual credentials and bucket information from Backblaze B2. Never commit your `.env` file to version control (it's already in `.gitignore`).

### Building and Running

To ensure everything is set up correctly, build and run your application:

1.  **Build:**
    ```bash
    cargo build
    ```
2.  **Run:**
    ```bash
    cargo run
    ```

If your `.env` is correctly configured and the bucket exists, you should see output similar to:

```
Bucket 'your-blazebackup-bucket' exists.
Hello, world!
```

This concludes Step 1. In the next step, we will implement the directory compression.
## Step 2: Enhancing Error Handling and First Upload

In this step, we'll improve our application's error handling using the `anyhow` crate and perform our first S3 upload.

### `Cargo.toml` Updates

First, add `anyhow` to your `Cargo.toml` dependencies. This crate simplifies error handling by providing a generic `Result` type that can wrap any error that implements `std::error::Error`.

```toml
[dependencies]
...
anyhow = "1.0.86" # For simplified error handling
```

After adding `anyhow`, run `cargo check` or `cargo build` to download and compile the new dependency.

### `src/main.rs` - Refactoring and Uploading

We've made several enhancements to `src/main.rs`:

1.  **`anyhow::Result` for main and functions:** We've replaced `Box<dyn std::error::Error>` with `anyhow::Result` for clearer and more ergonomic error propagation.

    ```rust
    // Before:
    // async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // After:
    use anyhow::Result;
    // ...
    // async fn main() -> Result<()> {
    ```

2.  **Passing References to `get_bucket`:** To avoid unnecessary data copying and to follow Rust's ownership best practices, the `get_bucket` function now accepts references (`&`) to the S3 client and bucket name.

    ```rust
    // Before:
    // async fn get_bucket(aws_client: client::Client, bucket_name: String) -> Result<Bucket, Box<dyn std::error::Error>> {
    // After:
    async fn get_bucket(
        aws_client: &client::Client, // Changed to reference
        bucket_name: &String,        // Changed to reference
    ) -> Result<Bucket> {
    ```

    The call in `main` was updated accordingly:
    ```rust
    let bucket = get_bucket(&client, &bucket_name).await?;
    ```

3.  **Graceful Error Handling in `get_bucket`:** Instead of `panic!`ing when a bucket isn't found, we now use `anyhow::bail!` to return a proper error, allowing the `main` function to handle it.

    ```rust
    // Before:
    // panic!("Bucket '{}' does not exist.", bucket_name)
    // After:
    anyhow::bail!("Bucket '{}' does not exist.", bucket_name);
    ```

4.  **First S3 Object Upload:** We've added code to `main` to upload a simple text object named `test.txt` with the content "Hello, world!" to your configured S3 bucket. This demonstrates the basic upload functionality.

    ```rust
    // ... inside main function ...
    match client
        .put_object()
        .bucket(bucket.name.unwrap())
        .key("test.txt")
        .body("Hello, world!".as_bytes().to_vec().into())
        .send()
        .await
    {
        Ok(_) => println!("Upload successfull to bucket: {:?}", bucket_name),
        Err(err) => println!("Error uploading object: {:?}", err),
    }
    // ...
    ```

Here's the updated `src/main.rs` code:

```rust
//! # BlazeBackup
//!
//! This is the main entry point for the BlazeBackup application.
//! BlazeBackup is a console application written in Rust that
//! compresses a specified directory and uploads it to an AWS S3-compatible
//! object storage service (like Backblaze B2).
//!
//! This file sets up the basic AWS S3 client, retrieves a specified bucket,
//! and demonstrates uploading a simple object to that bucket.

// Import necessary crates and modules.
// `aws_sdk_s3` provides the AWS SDK for S3, allowing us to interact with S3-compatible services.
use aws_sdk_s3::{client, types::Bucket};
// `dotenv` is used to load environment variables from a `.env` file.
// This is crucial for securely managing credentials and configuration without hardcoding them.
use anyhow::Result; // `anyhow::Result` is a convenient type alias for `Result<T, anyhow::Error>`,
                    // making error handling more ergonomic by allowing different error types
                    // to be converted into a common `anyhow::Error`.
use dotenv::dotenv;

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

    // Retrieve the `BUCKET_NAME` environment variable.
    // `std::env::var` reads the value of the specified environment variable.
    // `.unwrap()` is used here for simplicity. In a production application,
    // you would want to handle the `Result` returned by `var` more gracefully,
    // perhaps using `anyhow::Context` to add more informative error messages
    // if the variable is not set.
    let bucket_name = std::env::var("BUCKET_NAME").unwrap();

    // Load the AWS configuration.
    // `aws_config::from_env()` builds a configuration based on environment variables
    // (e.g., AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    // `.load().await` asynchronously loads this configuration.
    let config = aws_config::from_env().load().await;
    // Create a new S3 client using the loaded configuration.
    // This client will be used to perform S3 operations.
    let client = aws_sdk_s3::Client::new(&config);

    // Call the `get_bucket` asynchronous function to check if the specified bucket exists.
    // We pass references (`&client` and `&bucket_name`) to avoid moving ownership,
    // allowing these variables to be used later if needed.
    // `await` pauses the execution of `main` until `get_bucket` completes.
    // The `?` operator is a shorthand for error propagation: if `get_bucket` returns an `Err`,
    // it will immediately return from `main` with that error.
    let bucket = get_bucket(&client, &bucket_name).await?;

    match client
        .put_object()
        .bucket(bucket.name.unwrap())
        .key("test.txt")
        .body("Hello, world!".as_bytes().to_vec().into())
        .send()
        .await
    {
        Ok(_) => println!("Upload successfull to bucket: {:?}", bucket_name),
        Err(err) => println!("Error uploading object: {:?}", err),
    }

    // A general confirmation that the main function has executed to this point.
    println!("Hello, world!");
    Ok(()) // Indicate successful execution of the main function.
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
```

### Building and Running

After these changes, build and run your application again:

1.  **Add `anyhow` to `Cargo.toml` and rebuild:**
    ```bash
    cargo add anyhow
    ```
    This command will automatically add `anyhow` to your `Cargo.toml` and then perform a `cargo build`.

2.  **Run:**
    ```bash
    cargo run
    ```

If your `.env` is correctly configured and the bucket exists, you should now see:

```
Bucket 'your-blazebackup-bucket' exists.
Upload successfull to bucket: "your-blazebackup-bucket"
Hello, world!
```

You can also verify the upload by checking your Backblaze B2 bucket (or other S3-compatible storage) interface for a file named `test.txt`.

This concludes Step 2. In the next step, we will implement the directory compression.