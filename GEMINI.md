# Project Overview

This is console application "BlazeBackup". It's a console application written in Rust. It should take input parameter as a folder/directory path. Provided directory should be compressed to one file and uploaded to AWS S3 bucket. Information about AWS S3 bucket (key, bucket name etc.) should be stored in .env file. Finally application will be used with Backblaze B2 service.

Backblaze B2 API is compatible with S3 amazon services. Use Amazon S3 crate to access Backblaze. Do not use "backblaze-b2-client" for that.

Application should be created step by step in a tutorial manor. All the steps should be documented in markdown in Tutorial.md

# Building and Running

## Build

Build project using cargo.

> cargo build

## Run

Run project either by calling binary directly or using cargo:

> cargo run

## Test

To test the project use carg:

> cargo test

# Installation

# Development Conventions

The project follows standard Rust conventions. Use comments as if this project would be tutorial on how to program in rust. Do not assume any prior knowledge about rust from the reader.

### Additional Coding Preferences

- Keep project dependencies minimal.
- Use comments explaining purpose and functionality of code as if it would be tutorial/educational material.
- Use gitmoji https://gitmoji.dev/ convention for git commit messages.