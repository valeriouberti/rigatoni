# Rigatoni S3 Destination

Production-ready AWS S3 destination for the Rigatoni ETL framework. Supports multiple serialization formats, compression options, and flexible partitioning strategies.

## Features

- ✅ **Multiple Formats**: JSON, CSV, Parquet, Avro
- ✅ **Compression**: Gzip, Zstandard
- ✅ **Flexible Partitioning**: Hive-style, date-based, collection-based
- ✅ **Automatic Retry**: Exponential backoff with configurable retries
- ✅ **S3-Compatible**: Works with AWS S3, MinIO, LocalStack
- ✅ **Production Ready**: Comprehensive error handling and logging

## Quick Start

### Basic Usage

```rust
use rigatoni_core::Destination;
use rigatoni_destinations::s3::{S3Config, S3Destination};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = S3Config::builder()
        .bucket("my-data-lake")
        .region("us-east-1")
        .prefix("mongodb/events")
        .build()?;

    let mut destination = S3Destination::new(config).await?;

    // Write events
    // destination.write_batch(events).await?;

    destination.close().await?;
    Ok(())
}
```

### With Compression and Partitioning

```rust
use rigatoni_destinations::s3::{
    S3Config, S3Destination, Compression, KeyGenerationStrategy
};

let config = S3Config::builder()
    .bucket("analytics-data")
    .region("us-west-2")
    .prefix("events")
    .compression(Compression::Zstd)
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .build()?;

let destination = S3Destination::new(config).await?;
```

## Serialization Formats

### JSON (Default)

Newline-delimited JSON (JSONL) - one JSON object per line.

**Best for**:
- Human readability
- S3 Select queries
- Mixed schemas

**Enable**: `--features json` (included in default)

```rust
use rigatoni_destinations::s3::SerializationFormat;

S3Config::builder()
    .format(SerializationFormat::Json)
    // ...
```

### CSV

Comma-separated values with JSON-encoded nested documents.

**Best for**:
- Excel compatibility
- Simple data
- Quick analysis

**Enable**: `--features csv`

```rust
S3Config::builder()
    .format(SerializationFormat::Csv)
    // ...
```

### Parquet

Apache Parquet columnar format.

**Best for**:
- Analytics workloads
- Excellent compression
- Fast queries

**Enable**: `--features parquet`

```rust
S3Config::builder()
    .format(SerializationFormat::Parquet)
    // ...
```

### Avro

Apache Avro binary format with schema evolution support.

**Best for**:
- Schema evolution
- Streaming
- Kafka integration

**Enable**: `--features avro`

```rust
S3Config::builder()
    .format(SerializationFormat::Avro)
    // ...
```

## Compression

### Gzip

Standard compression with wide compatibility.

**Stats**: ~70-80% compression ratio, moderate speed

**Enable**: `--features gzip`

```rust
use rigatoni_destinations::s3::Compression;

S3Config::builder()
    .compression(Compression::Gzip)
    // ...
```

### Zstandard

Modern compression with better ratio and speed than gzip.

**Stats**: ~75-85% compression ratio, faster than gzip

**Enable**: `--features zstandard`

```rust
S3Config::builder()
    .compression(Compression::Zstd)
    // ...
```

## Key Generation Strategies

### Hive Partitioned

Hive-style partitioning for analytics platforms.

**Pattern**: `{prefix}/collection={name}/year={YYYY}/month={MM}/day={DD}/hour={HH}/{timestamp}.{ext}`

**Example**: `events/collection=users/year=2025/month=01/day=15/hour=10/1705318800000.jsonl`

```rust
use rigatoni_destinations::s3::KeyGenerationStrategy;

S3Config::builder()
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    // ...
```

**Best for**: Athena, Presto, Spark (automatic partition discovery)

### Date Hour Partitioned (Default)

Simple time-based partitioning with hour granularity.

**Pattern**: `{prefix}/{collection}/{YYYY}/{MM}/{DD}/{HH}/{timestamp}.{ext}`

**Example**: `events/users/2025/01/15/10/1705318800000.jsonl`

```rust
S3Config::builder()
    .key_strategy(KeyGenerationStrategy::DateHourPartitioned)
    // ...
```

**Best for**: Time-range queries, lifecycle policies, human-readable structure

### Date Partitioned

Daily partitioning without hour granularity.

**Pattern**: `{prefix}/{collection}/{YYYY}/{MM}/{DD}/{timestamp}.{ext}`

**Best for**: Daily aggregations, lower partition count

### Collection Based

Simple grouping by collection.

**Pattern**: `{prefix}/{collection}/{timestamp}.{ext}`

**Best for**: Collection-specific access, minimal partitioning

### Flat

Flat structure with timestamp.

**Pattern**: `{prefix}/{collection}_{timestamp}.{ext}`

**Best for**: Simple backups, testing

## Configuration Options

```rust
S3Config::builder()
    .bucket("my-bucket")              // Required
    .region("us-east-1")              // Required
    .prefix("path/to/data")           // Optional prefix
    .format(SerializationFormat::Json) // Default: JSON
    .compression(Compression::Zstd)    // Default: None
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .max_retries(5)                   // Default: 3
    .endpoint_url("http://localhost:4566") // For LocalStack/MinIO
    .force_path_style(true)           // For LocalStack/MinIO
    .build()?
```

## Examples

Run the examples to see the S3 destination in action:

### Basic Example

```bash
export S3_BUCKET="your-bucket-name"
cargo run --example s3_basic --features s3,json
```

### With Compression

```bash
# Gzip compression
cargo run --example s3_with_compression --features s3,json,gzip

# Zstd compression (better)
cargo run --example s3_with_compression --features s3,json,zstandard
```

### Advanced Features

```bash
# All features
cargo run --example s3_advanced --all-features

# Specific formats
cargo run --example s3_advanced --features s3,csv,gzip
cargo run --example s3_advanced --features s3,parquet,zstandard
cargo run --example s3_advanced --features s3,avro
```

## Testing

### Unit Tests

```bash
cargo test --package rigatoni-destinations --features s3,json,gzip,zstandard --lib
```

### Integration Tests with LocalStack

Start LocalStack:

```bash
cd rigatoni-destinations
docker-compose up -d
```

Wait for LocalStack to be healthy:

```bash
docker-compose logs -f
```

Run integration tests:

```bash
# All integration tests
cargo test --package rigatoni-destinations --test s3_integration_test --all-features -- --ignored

# Specific tests
cargo test --test s3_integration_test test_s3_basic_write --features s3,json -- --ignored
cargo test --test s3_integration_test test_s3_with_gzip_compression --features s3,json,gzip -- --ignored
```

Stop LocalStack:

```bash
docker-compose down
```

## Analytics Integration

### AWS Athena

```sql
-- Create external table with Hive partitioning
CREATE EXTERNAL TABLE mongodb_events (
  operation STRING,
  database STRING,
  collection STRING,
  cluster_time TIMESTAMP,
  full_document STRING
)
PARTITIONED BY (
  collection_name STRING,
  year INT,
  month INT,
  day INT,
  hour INT
)
STORED AS PARQUET
LOCATION 's3://your-bucket/analytics/mongodb-cdc/';

-- Discover partitions
MSCK REPAIR TABLE mongodb_events;

-- Query with partition pruning
SELECT * FROM mongodb_events
WHERE collection_name = 'users'
  AND year = 2025
  AND month = 1
  AND day = 15;
```

### Apache Spark

```python
# Read with partition discovery
df = spark.read.parquet("s3://your-bucket/analytics/mongodb-cdc/")

# Query with partition pruning
users_df = df.filter(
    (df.collection_name == "users") &
    (df.year == 2025) &
    (df.month == 1)
)
```

## Performance Tips

1. **Batching**: Buffer events and write in larger batches (1000-10000 events)
2. **Compression**: Use Zstd for best compression ratio and speed
3. **Partitioning**: Use Hive partitioning for analytics workloads
4. **Format**:
   - JSON for flexibility and debugging
   - Parquet for analytics and compression
   - CSV for Excel/simple analysis
   - Avro for schema evolution
5. **Retries**: Increase `max_retries` for production (5-10)

## Troubleshooting

### Access Denied

Ensure your AWS credentials have S3 write permissions:

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
      "Resource": "arn:aws:s3:::your-bucket/*"
    }
  ]
}
```

### LocalStack Connection Refused

```bash
# Check LocalStack is running
docker-compose ps

# View LocalStack logs
docker-compose logs -f localstack

# Restart LocalStack
docker-compose restart localstack
```

### Path Style Addressing

For LocalStack and MinIO, you must enable path-style addressing:

```rust
S3Config::builder()
    .force_path_style(true)
    // ...
```

## License

Licensed under MIT OR Apache-2.0
