---
layout: default
title: Getting Started
nav_order: 2
description: "Learn how to install Rigatoni and build your first data pipeline."
permalink: /getting-started
---

# Getting Started
{: .no_toc }

Learn how to install Rigatoni and build your first data pipeline in minutes.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.85 or later** - [Install Rust](https://www.rust-lang.org/tools/install)
- **MongoDB** - For the source (local or remote instance)
- **AWS credentials** - For S3 destination (or LocalStack for testing)

### Verify Rust Installation

```bash
rustc --version
# Should output: rustc 1.85.0 (or later)

cargo --version
# Should output: cargo 1.85.0 (or later)
```

---

## Installation

### Create a New Project

```bash
cargo new my-etl-pipeline
cd my-etl-pipeline
```

### Add Dependencies

Edit your `Cargo.toml`:

```toml
[package]
name = "my-etl-pipeline"
version = "0.1.0"
edition = "2021"

[dependencies]
rigatoni-core = "0.1"
rigatoni-destinations = { version = "0.1", features = ["s3", "json"] }

# Additional dependencies for the example
tokio = { version = "1.40", features = ["full"] }
tracing-subscriber = "0.3"
```

### Feature Flags

Rigatoni uses feature flags to reduce compile time and binary size:

**Destination Features:**
- `s3` - AWS S3 destination
- `bigquery` - Google BigQuery destination
- `kafka` - Apache Kafka destination

**Format Features:**
- `json` - JSON/JSONL support (default)
- `csv` - CSV support
- `parquet` - Apache Parquet support
- `avro` - Apache Avro support

**Compression Features:**
- `gzip` - Gzip compression
- `zstandard` - Zstandard compression

**Example** - S3 with Parquet and Zstd:

```toml
rigatoni-destinations = { version = "0.1", features = ["s3", "parquet", "zstandard"] }
```

---

## Your First Pipeline

Let's build a simple pipeline that streams MongoDB changes to S3.

### Step 1: Set Up MongoDB

Start MongoDB locally (if you don't have it running):

```bash
# Using Docker
docker run -d -p 27017:27017 --name mongodb mongo:latest

# Or install MongoDB locally
# https://www.mongodb.com/docs/manual/installation/
```

Insert some test data:

```bash
mongosh

use mydb
db.users.insertMany([
  { name: "Alice", email: "alice@example.com", age: 30 },
  { name: "Bob", email: "bob@example.com", age: 25 }
])
```

### Step 2: Configure AWS Credentials

For production:

```bash
# Set AWS credentials
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1
```

For testing with LocalStack:

```bash
# Install LocalStack
pip install localstack

# Start LocalStack
localstack start -d

# Set LocalStack credentials (dummy values)
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_REGION=us-east-1
```

### Step 3: Write the Pipeline Code

Create `src/main.rs`:

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_destinations::s3::{S3Config, S3Destination};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting MongoDB to S3 pipeline...\n");

    // Configure S3 destination
    let s3_config = S3Config::builder()
        .bucket("my-data-lake")
        .region("us-east-1")
        .prefix("mongodb-cdc")
        .build()?;

    let destination = S3Destination::new(s3_config).await?;

    // Configure pipeline
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("mydb")
        .collections(vec!["users".to_string()])
        .batch_size(100)
        .build()?;

    println!("Configuration:");
    println!("  MongoDB: mongodb://localhost:27017/mydb");
    println!("  Collections: users");
    println!("  S3 Bucket: my-data-lake");
    println!("  Prefix: mongodb-cdc\n");

    // Create and run pipeline
    println!("Starting pipeline...\n");
    let mut pipeline = Pipeline::new(config, destination).await?;

    // Run pipeline (this blocks until shutdown)
    pipeline.run().await?;

    Ok(())
}
```

### Step 4: Run the Pipeline

```bash
cargo run
```

You should see output like:

```
Starting MongoDB to S3 pipeline...

Configuration:
  MongoDB: mongodb://localhost:27017/mydb
  Collections: users
  S3 Bucket: my-data-lake
  Prefix: mongodb-cdc

Starting pipeline...

INFO rigatoni_core::pipeline: Pipeline started
INFO rigatoni_core::pipeline: Worker 0 started for collection: users
```

### Step 5: Test the Pipeline

In another terminal, insert more data:

```bash
mongosh

use mydb
db.users.insertOne({ name: "Charlie", email: "charlie@example.com", age: 35 })
```

You should see the pipeline process the change:

```
INFO rigatoni_core::pipeline: Batching 1 events for collection: users
INFO rigatoni_destinations::s3: Writing batch to S3: mongodb-cdc/users/2025/01/15/10/1705318800000.jsonl
```

### Step 6: Verify S3 Upload

Check your S3 bucket:

```bash
# AWS CLI
aws s3 ls s3://my-data-lake/mongodb-cdc/users/ --recursive

# LocalStack
awslocal s3 ls s3://my-data-lake/mongodb-cdc/users/ --recursive
```

---

## Configuration Deep Dive

### Pipeline Configuration

```rust
PipelineConfig::builder()
    // MongoDB connection
    .mongodb_uri("mongodb://localhost:27017")
    .database("mydb")
    .collections(vec!["users".to_string(), "orders".to_string()])

    // Batching
    .batch_size(1000)          // Max events per batch
    .batch_timeout_ms(5000)    // Max wait time for batch (ms)

    // Workers
    .num_workers(4)            // Concurrent workers per collection

    // Retry configuration
    .max_retries(3)            // Max retry attempts
    .retry_delay_ms(1000)      // Initial retry delay
    .max_retry_delay_ms(60000) // Max retry delay

    // Buffering
    .channel_buffer_size(1000) // Internal channel buffer

    .build()?
```

### S3 Destination Configuration

```rust
use rigatoni_destinations::s3::{
    S3Config, Compression, SerializationFormat, KeyGenerationStrategy
};

S3Config::builder()
    // Required
    .bucket("my-bucket")
    .region("us-east-1")

    // Optional
    .prefix("data/mongodb")
    .format(SerializationFormat::Parquet)
    .compression(Compression::Zstd)
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .max_retries(5)

    // For LocalStack/MinIO
    .endpoint_url("http://localhost:4566")
    .force_path_style(true)

    .build()?
```

---

## Advanced Examples

### With Parquet and Compression

```rust
let s3_config = S3Config::builder()
    .bucket("analytics-data")
    .region("us-west-2")
    .prefix("events")
    .format(SerializationFormat::Parquet)
    .compression(Compression::Zstd)
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .build()?;
```

This creates keys like:
```
events/collection=users/year=2025/month=01/day=15/hour=10/1705318800000.parquet.zst
```

### With Multiple Collections

```rust
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017")
    .database("mydb")
    .collections(vec![
        "users".to_string(),
        "orders".to_string(),
        "products".to_string(),
    ])
    .num_workers(2)  // 2 workers per collection = 6 total workers
    .build()?;
```

### With Custom Retry Logic

```rust
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017")
    .database("mydb")
    .collections(vec!["critical_data".to_string()])
    .max_retries(10)           // Retry up to 10 times
    .retry_delay_ms(500)       // Start with 500ms delay
    .max_retry_delay_ms(30000) // Cap at 30 seconds
    .build()?;
```

---

## Error Handling

### Common Errors

#### MongoDB Connection Error

```
Error: Failed to connect to MongoDB
```

**Solution**: Verify MongoDB is running and the URI is correct:

```bash
mongosh mongodb://localhost:27017
```

#### S3 Access Denied

```
Error: S3 operation failed: Access Denied
```

**Solution**: Verify AWS credentials and S3 bucket permissions:

```bash
aws s3 ls s3://my-bucket/
```

#### Invalid Configuration

```
Error: bucket is required
```

**Solution**: Ensure all required configuration fields are set:

```rust
let config = S3Config::builder()
    .bucket("my-bucket")  // Required
    .region("us-east-1")  // Required
    .build()?;
```

### Error Recovery

The pipeline automatically retries on transient errors with exponential backoff:

```rust
// Automatic retry with backoff
// Attempt 1: immediate
// Attempt 2: 1000ms delay
// Attempt 3: 2000ms delay (exponential)
// Attempt 4: 4000ms delay
// ...up to max_retry_delay_ms
```

---

## Best Practices

### 1. Batching

Use larger batch sizes for higher throughput:

```rust
.batch_size(5000)      // Good for high-volume streams
.batch_timeout_ms(30000) // 30 seconds max wait
```

### 2. Compression

Use Zstandard for better performance:

```rust
.compression(Compression::Zstd)  // Better than Gzip
```

### 3. Partitioning

Use Hive partitioning for analytics:

```rust
.key_strategy(KeyGenerationStrategy::HivePartitioned)
```

### 4. Monitoring

Enable comprehensive logging:

```rust
tracing_subscriber::fmt()
    .with_env_filter("rigatoni=debug,warn")
    .init();
```

### 5. Graceful Shutdown

Handle CTRL+C for graceful shutdown:

```rust
use tokio::signal;

// In main()
tokio::select! {
    result = pipeline.run() => {
        result?;
    }
    _ = signal::ctrl_c() => {
        println!("\nShutting down gracefully...");
        pipeline.shutdown().await?;
    }
}
```

---

## Next Steps

Now that you have a working pipeline, explore more features:

- **[Architecture](architecture)** - Understand how Rigatoni works
- **[User Guides](guides/)** - Task-specific guides
- **[API Reference](https://docs.rs/rigatoni)** - Complete API documentation

---

## Troubleshooting

### Pipeline Not Processing Changes

1. **Verify MongoDB is in replica set mode** - Change streams require replica sets:

```bash
# Start MongoDB as a replica set
mongod --replSet rs0

# Initialize replica set
mongosh
rs.initiate()
```

2. **Check collection exists and has data**:

```bash
mongosh
use mydb
db.users.find()
```

3. **Enable debug logging**:

```rust
tracing_subscriber::fmt()
    .with_env_filter("rigatoni=debug")
    .init();
```

### High Memory Usage

Reduce batch size and buffer size:

```rust
.batch_size(100)
.channel_buffer_size(100)
```

### Slow S3 Uploads

Enable compression to reduce data size:

```rust
.compression(Compression::Zstd)
```

---

Need help? [Open an issue on GitHub](https://github.com/valeriouberti/rigatoni/issues)
