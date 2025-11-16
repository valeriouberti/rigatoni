---
layout: default
title: S3 Configuration
parent: Guides
nav_order: 1
description: "Complete guide to configuring the S3 destination."
---

# S3 Configuration Guide
{: .no_toc }

Learn how to configure the S3 destination for your use case.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Basic Configuration

The minimal S3 configuration requires only a bucket and region:

```rust
use rigatoni_destinations::s3::S3Config;

let config = S3Config::builder()
    .bucket("my-data-lake")
    .region("us-east-1")
    .build()?;
```

## Configuration Options

### Required Fields

#### `bucket(impl Into<String>)`
{: .text-purple-000 }

The S3 bucket name where data will be written.

**Validation Rules:**
- Must be 3-63 characters long
- Only lowercase letters, numbers, hyphens, and periods
- Cannot start or end with a hyphen

```rust
.bucket("my-data-lake")  // ✓ Valid
.bucket("MyBucket")       // ✗ Invalid (uppercase)
.bucket("ab")             // ✗ Invalid (too short)
```

#### `region(impl Into<String>)`
{: .text-purple-000 }

AWS region where the bucket is located.

```rust
.region("us-east-1")
.region("eu-west-1")
.region("ap-southeast-2")
```

### Optional Fields

#### `prefix(impl Into<String>)`
{: .text-blue-000 }

Key prefix for all objects. Useful for organizing data within a bucket.

```rust
.prefix("mongodb-cdc")
.prefix("production/events")
```

**Validation Rules:**
- Cannot contain `..` (path traversal)
- Cannot start with `/`

**Example Key:**
```
Without prefix: users/2025/01/15/10/1705318800000.jsonl
With prefix:    mongodb-cdc/users/2025/01/15/10/1705318800000.jsonl
```

#### `format(SerializationFormat)`
{: .text-blue-000 }

Serialization format for events. See [Formats and Compression](formats-compression) guide.

**Available formats:**
- `SerializationFormat::Json` - Newline-delimited JSON (default)
- `SerializationFormat::Csv` - Comma-separated values
- `SerializationFormat::Parquet` - Apache Parquet columnar format
- `SerializationFormat::Avro` - Apache Avro binary format

```rust
use rigatoni_destinations::s3::SerializationFormat;

.format(SerializationFormat::Parquet)
```

#### `compression(Compression)`
{: .text-blue-000 }

Compression algorithm for objects. See [Formats and Compression](formats-compression) guide.

**Available compression:**
- `Compression::None` - No compression (default)
- `Compression::Gzip` - Gzip compression (requires `gzip` feature)
- `Compression::Zstd` - Zstandard compression (requires `zstandard` feature)

```rust
use rigatoni_destinations::s3::Compression;

.compression(Compression::Zstd)
```

#### `key_strategy(KeyGenerationStrategy)`
{: .text-blue-000 }

Strategy for generating S3 object keys. See [Partitioning Strategies](partitioning) guide.

**Available strategies:**
- `KeyGenerationStrategy::DateHourPartitioned` - Date and hour (default)
- `KeyGenerationStrategy::HivePartitioned` - Hive-style partitioning
- `KeyGenerationStrategy::DatePartitioned` - Date only
- `KeyGenerationStrategy::CollectionBased` - Collection grouping
- `KeyGenerationStrategy::Flat` - Flat structure

```rust
use rigatoni_destinations::s3::KeyGenerationStrategy;

.key_strategy(KeyGenerationStrategy::HivePartitioned)
```

#### `max_retries(u32)`
{: .text-blue-000 }

Maximum number of retries for S3 operations. Default: `3`.

```rust
.max_retries(5)  // Retry up to 5 times
```

#### `endpoint_url(impl Into<String>)`
{: .text-blue-000 }

Custom S3 endpoint URL for S3-compatible storage (LocalStack, MinIO, etc.).

```rust
.endpoint_url("http://localhost:4566")  // LocalStack
.endpoint_url("http://minio:9000")      // MinIO
```

#### `force_path_style(bool)`
{: .text-blue-000 }

Use path-style addressing instead of virtual-hosted style. Required for LocalStack and MinIO.

```rust
.force_path_style(true)
```

**URL Styles:**
- Virtual-hosted: `https://bucket.s3.region.amazonaws.com/key`
- Path-style: `https://s3.region.amazonaws.com/bucket/key`

---

## Complete Example

```rust
use rigatoni_destinations::s3::{
    S3Config, Compression, SerializationFormat, KeyGenerationStrategy
};

let config = S3Config::builder()
    // Required
    .bucket("analytics-data")
    .region("us-west-2")

    // Optional
    .prefix("mongodb-events")
    .format(SerializationFormat::Parquet)
    .compression(Compression::Zstd)
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .max_retries(5)

    .build()?;
```

This configuration creates keys like:
```
analytics-data/mongodb-events/collection=users/year=2025/month=01/day=15/hour=10/1705318800000.parquet.zst
```

---

## AWS Credentials

The S3 destination uses the AWS SDK, which loads credentials from:

### 1. Environment Variables

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1
```

### 2. AWS Credentials File

`~/.aws/credentials`:
```ini
[default]
aws_access_key_id = your_access_key
aws_secret_access_key = your_secret_key
```

`~/.aws/config`:
```ini
[default]
region = us-east-1
```

### 3. IAM Instance Profile

For EC2 instances or ECS tasks, use IAM roles.

### 4. Environment Variables (Alternative)

```bash
export AWS_PROFILE=production
export AWS_DEFAULT_REGION=us-west-2
```

### Required IAM Permissions

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:PutObjectAcl"
      ],
      "Resource": "arn:aws:s3:::my-data-lake/*"
    }
  ]
}
```

---

## LocalStack Configuration

For local development and testing with LocalStack:

```rust
let config = S3Config::builder()
    .bucket("test-bucket")
    .region("us-east-1")
    .endpoint_url("http://localhost:4566")
    .force_path_style(true)
    .build()?;
```

**Environment Setup:**

```bash
# Install LocalStack
pip install localstack

# Start LocalStack
localstack start -d

# Set credentials (dummy values)
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_REGION=us-east-1

# Create bucket
awslocal s3 mb s3://test-bucket
```

See [LocalStack Development Guide](localstack) for more details.

---

## Configuration Validation

The builder validates configuration at build time:

### Valid Configuration

```rust
let config = S3Config::builder()
    .bucket("my-bucket")
    .region("us-east-1")
    .build()?;  // ✓ OK
```

### Invalid Configurations

```rust
// Missing required field
S3Config::builder()
    .bucket("my-bucket")
    // Missing region
    .build()?;  // ✗ Error: region is required

// Invalid bucket name
S3Config::builder()
    .bucket("MyBucket")  // Uppercase not allowed
    .region("us-east-1")
    .build()?;  // ✗ Error: invalid bucket name

// Invalid prefix
S3Config::builder()
    .bucket("my-bucket")
    .region("us-east-1")
    .prefix("data/../secrets")  // Path traversal
    .build()?;  // ✗ Error: invalid prefix
```

### Error Handling

```rust
use rigatoni_destinations::s3::S3ConfigError;

match S3Config::builder()
    .bucket("ab")
    .region("us-east-1")
    .build()
{
    Ok(config) => { /* use config */ }
    Err(S3ConfigError::InvalidBucket { name, reason }) => {
        eprintln!("Invalid bucket '{}': {}", name, reason);
    }
    Err(e) => {
        eprintln!("Config error: {}", e);
    }
}
```

---

## Environment-Based Configuration

### Using Environment Variables

```rust
use std::env;

let config = S3Config::builder()
    .bucket(env::var("S3_BUCKET")?)
    .region(env::var("AWS_REGION")?)
    .prefix(env::var("S3_PREFIX").unwrap_or_default())
    .build()?;
```

### Configuration File (TOML)

`config.toml`:
```toml
[s3]
bucket = "my-data-lake"
region = "us-east-1"
prefix = "mongodb-cdc"
format = "parquet"
compression = "zstd"
```

Load with `config` crate:

```rust
use config::{Config, File};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    s3: S3Settings,
}

#[derive(Deserialize)]
struct S3Settings {
    bucket: String,
    region: String,
    prefix: Option<String>,
}

let settings = Config::builder()
    .add_source(File::with_name("config"))
    .build()?
    .try_deserialize::<AppConfig>()?;

let s3_config = S3Config::builder()
    .bucket(&settings.s3.bucket)
    .region(&settings.s3.region)
    .prefix(settings.s3.prefix.unwrap_or_default())
    .build()?;
```

---

## Best Practices

### 1. Use Descriptive Prefixes

Organize data by environment and purpose:

```rust
// Development
.prefix("dev/mongodb-cdc")

// Production
.prefix("prod/mongodb-cdc")

// By source system
.prefix("mongodb/production/cdc")
```

### 2. Enable Compression

Reduce storage costs and bandwidth:

```rust
.compression(Compression::Zstd)  // Best ratio and speed
```

### 3. Use Appropriate Partitioning

Choose based on query patterns:

```rust
// Analytics: Hive partitioning
.key_strategy(KeyGenerationStrategy::HivePartitioned)

// Backups: Date partitioning
.key_strategy(KeyGenerationStrategy::DatePartitioned)
```

### 4. Increase Retries for Production

```rust
.max_retries(10)  // More retries for production
```

### 5. Use IAM Roles Instead of Access Keys

For EC2/ECS deployments, use IAM instance profiles instead of hardcoding credentials.

---

## Troubleshooting

### Access Denied

**Error:**
```
Error: S3 operation failed: Access Denied
```

**Solutions:**
1. Verify AWS credentials are configured
2. Check IAM permissions include `s3:PutObject`
3. Verify bucket name and region are correct

### Bucket Not Found

**Error:**
```
Error: S3 operation failed: NoSuchBucket
```

**Solutions:**
1. Create the bucket: `aws s3 mb s3://my-bucket`
2. Verify bucket name spelling
3. Verify region matches bucket location

### Invalid Bucket Name

**Error:**
```
Error: invalid bucket name: MyBucket (must contain only lowercase letters, numbers, hyphens, and periods)
```

**Solution:**
Use lowercase letters only:
```rust
.bucket("my-bucket")  // ✓ Correct
```

---

## Next Steps

- **[Formats and Compression](formats-compression)** - Choose the right format
- **[Partitioning Strategies](partitioning)** - Optimize for analytics
- **[Production Deployment](production-deployment)** - Deploy to production
