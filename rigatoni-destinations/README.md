# rigatoni-destinations

Destination implementations for Rigatoni CDC/Data Replication framework - write data to S3 and other targets.

[![Crates.io](https://img.shields.io/crates/v/rigatoni-destinations.svg)](https://crates.io/crates/rigatoni-destinations)
[![Documentation](https://docs.rs/rigatoni-destinations/badge.svg)](https://docs.rs/rigatoni-destinations)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../LICENSE)

## Overview

Production-ready destination implementations for streaming data from MongoDB to various targets.

## Supported Destinations

### AWS S3

- **Multiple Formats**: JSON, CSV, Parquet, Avro
- **Compression**: Gzip, Zstandard
- **Partitioning**: Hive-style, date-based, collection-based
- **Features**: Retry logic, S3-compatible storage (LocalStack, MinIO)

## Installation

```toml
[dependencies]
rigatoni-destinations = { version = "0.1.1", features = ["s3", "json"] }
```

### Available Features

**Destinations:**
- `s3` - AWS S3 (enabled by default)

**Formats:**
- `json` - JSON/JSONL (enabled by default)
- `csv` - CSV format
- `parquet` - Apache Parquet
- `avro` - Apache Avro

**Compression:**
- `gzip` - Gzip compression
- `zstandard` - Zstandard compression

**Convenience:**
- `all-formats` - All serialization formats
- `all` - All features (S3 + all formats + compression)

## Quick Start - S3 Destination

```rust
use rigatoni_destinations::s3::{S3Config, S3Destination};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = S3Config::builder()
        .bucket("my-data-lake")
        .region("us-east-1")
        .prefix("mongodb-cdc")
        .build()?;

    let destination = S3Destination::new(config).await?;

    // Use with Rigatoni pipeline
    // pipeline.set_destination(destination).await?;

    Ok(())
}
```

## S3 Features

### Serialization Formats

```rust
use rigatoni_destinations::s3::SerializationFormat;

// JSON (default)
.format(SerializationFormat::Json)

// Parquet for analytics
.format(SerializationFormat::Parquet)

// CSV for exports
.format(SerializationFormat::Csv)

// Avro for streaming
.format(SerializationFormat::Avro)
```

### Compression

```rust
use rigatoni_destinations::s3::Compression;

// Gzip (widely compatible)
.compression(Compression::Gzip)

// Zstandard (better ratio and speed)
.compression(Compression::Zstd)
```

### Partitioning Strategies

```rust
use rigatoni_destinations::s3::KeyGenerationStrategy;

// Hive partitioning for analytics
.key_strategy(KeyGenerationStrategy::HivePartitioned)
// Creates: collection=users/year=2025/month=01/day=16/hour=10/timestamp.ext

// Date-hour partitioning (default)
.key_strategy(KeyGenerationStrategy::DateHourPartitioned)
// Creates: users/2025/01/16/10/timestamp.ext

// Date partitioning
.key_strategy(KeyGenerationStrategy::DatePartitioned)
// Creates: users/2025/01/16/timestamp.ext
```

## Examples

See the [rigatoni-examples](../rigatoni-examples/) directory:

- `s3_basic` - Basic S3 usage
- `s3_advanced` - Advanced features (formats, compression, partitioning)
- `s3_with_compression` - Compression examples

```bash
cargo run --example s3_basic --features s3,json
cargo run --example s3_advanced --all-features
```

## Testing with LocalStack

```bash
# Start LocalStack
docker-compose up -d

# Run integration tests
cargo test --test s3_integration_test --features s3,json,gzip -- --ignored
```

## Documentation

- [S3 Configuration Guide](https://valeriouberti.github.io/rigatoni/guides/s3-configuration)
- [Getting Started](https://valeriouberti.github.io/rigatoni/getting-started)
- [API Documentation](https://docs.rs/rigatoni-destinations)
- [Main Repository](https://github.com/valeriouberti/rigatoni)

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](../LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).
