# rigatoni-destinations

Destination implementations for Rigatoni ETL framework - write data to S3, BigQuery, Kafka, and more.

[![Crates.io](https://img.shields.io/crates/v/rigatoni-destinations.svg)](https://crates.io/crates/rigatoni-destinations)
[![Documentation](https://docs.rs/rigatoni-destinations/badge.svg)](https://docs.rs/rigatoni-destinations)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE)

## Overview

Production-ready destination implementations for streaming data from MongoDB to various targets.

## Supported Destinations

### AWS S3 (Available)

- **Multiple Formats**: JSON, CSV, Parquet, Avro
- **Compression**: Gzip, Zstandard
- **Partitioning**: Hive-style, date-based, collection-based
- **Features**: Retry logic, S3-compatible storage (LocalStack, MinIO)

### Coming Soon

- **Google BigQuery** - Data warehouse integration
- **Apache Kafka** - Event streaming

## Installation

```toml
[dependencies]
rigatoni-destinations = { version = "0.1", features = ["s3", "json"] }
```

### Available Features

**Destinations:**
- `s3` - AWS S3 (enabled by default)
- `bigquery` - Google BigQuery (coming soon)
- `kafka` - Apache Kafka (coming soon)

**Formats:**
- `json` - JSON/JSONL (enabled by default)
- `csv` - CSV format
- `parquet` - Apache Parquet
- `avro` - Apache Avro

**Compression:**
- `gzip` - Gzip compression
- `zstandard` - Zstandard compression

**Convenience:**
- `all-destinations` - All destination implementations
- `all-formats` - All serialization formats
- `all` - Everything

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

See the [examples](examples/) directory:

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

Licensed under either of:

- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
