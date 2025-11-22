# Rigatoni Destinations Examples

Detailed documentation for S3 destination examples with compression, partitioning, and analytics patterns.

> **💡 New to Rigatoni examples?** Start with the [Examples Overview](../../examples/README.md) for quick start guides and common setup instructions.

## Examples in This Directory

| Example | Difficulty | Key Features | Description |
|---------|-----------|--------------|-------------|
| [s3_basic.rs](#s3_basicrs) | ⭐ Beginner | JSON, LocalStack | Simple S3 upload with default settings |
| [s3_with_compression.rs](#s3_with_compressionrs) | ⭐⭐ Intermediate | Gzip, Zstd | Compression strategies and cost optimization |
| [s3_advanced.rs](#s3_advancedrs) | ⭐⭐⭐ Advanced | Parquet, partitioning | Analytics data lake with Hive partitioning |

**Prerequisites:** See [LocalStack Setup](../../examples/README.md#localstack-setup-for-s3-examples) for S3 configuration.

---

## s3_basic.rs

**Simple S3 upload** with JSON serialization - perfect for getting started.

### What It Demonstrates

- Basic S3 configuration with minimal settings
- JSON serialization (human-readable format)
- Default key generation strategy
- Writing and flushing batches

### Running

```bash
# With LocalStack (see Examples Overview for setup)
cargo run -p rigatoni-destinations --example s3_basic --features s3,json

# With custom bucket
S3_BUCKET=my-bucket cargo run -p rigatoni-destinations --example s3_basic --features s3,json
```

### Key Concepts

#### S3Config Builder

```rust
let config = S3Config::builder()
    .bucket("rigatoni-test-bucket")
    .region("us-east-1")
    .prefix("rigatoni/examples/basic")
    .build()?;
```

**Options:**
- `format(SerializationFormat::Json)` - Default format
- `compression(Compression::None)` - No compression by default
- `max_retries(3)` - Retry failed uploads
- `endpoint("http://localhost:4566")` - For LocalStack

#### JSON Format

**Characteristics:**
- **Pros:** Human-readable, widely supported, easy debugging
- **Cons:** Larger file size, slower parsing
- **Best for:** Development, debugging, small-medium volumes

#### File Organization

Default structure:
```
s3://bucket/prefix/collection/YYYY/MM/DD/HH/batch-{uuid}.json
```

Example:
```
rigatoni-test-bucket/
└── rigatoni/examples/basic/
    ├── users/2025/01/21/14/batch-abc123.json
    └── products/2025/01/21/14/batch-xyz789.json
```

### Verifying Output

```bash
# List files
awslocal s3 ls s3://rigatoni-test-bucket/rigatoni/examples/basic/ --recursive

# Download and view
awslocal s3 cp s3://rigatoni-test-bucket/.../batch-abc123.json - | jq .
```

### Customizing

Change bucket and region:
```rust
.bucket("my-production-bucket")
.region("eu-west-1")
```

Use different prefix:
```rust
.prefix("data-lake/raw/mongodb/cdc")
```

For LocalStack:
```rust
.endpoint("http://localhost:4566")
```

### Troubleshooting

See [Common Issues](../../examples/README.md#common-issues) for MongoDB, Redis, and LocalStack problems.

**S3-specific issues:**

**Bucket not found:**
```bash
awslocal s3 mb s3://rigatoni-test-bucket
```

**Access denied:**
- Check AWS credentials are configured
- Verify IAM permissions (s3:PutObject, s3:GetObject, s3:ListBucket)

**Next:** Try [s3_with_compression.rs](#s3_with_compressionrs) to reduce storage costs.

---

## s3_with_compression.rs

**Optimize storage and bandwidth** with compression.

### What It Demonstrates

- Gzip compression (industry standard, wide compatibility)
- Zstandard compression (better ratio, faster than gzip)
- Compression trade-offs and benchmarking
- Storage cost optimization

### Running

```bash
# With Gzip
cargo run -p rigatoni-destinations --example s3_with_compression --features s3,json,gzip

# With Zstandard
cargo run -p rigatoni-destinations --example s3_with_compression --features s3,json,zstandard
```

### Key Concepts

#### Enabling Compression

```rust
use rigatoni_destinations::s3::Compression;

let config = S3Config::builder()
    .bucket("my-bucket")
    .compression(Compression::Gzip)  // or Compression::Zstd
    .build()?;
```

**Options:**
- `Compression::None` - No compression (default)
- `Compression::Gzip` - Requires `gzip` feature
- `Compression::Zstd` - Requires `zstandard` feature

#### Compression Comparison

| Algorithm | Ratio | Speed | CPU | Best For |
|-----------|-------|-------|-----|----------|
| None | 0% | Fastest | Minimal | Small volumes, low latency |
| Gzip | 70-80% | Medium | Medium | Wide compatibility, standard |
| Zstd | 75-85% | Fast | Low-Med | Best balance, modern systems |

#### Storage Cost Savings

**Example: 100GB/month of change events**

**Without compression:**
- S3 Standard: $2.30/month
- **Total: $2.30/month**

**With Gzip (75% reduction):**
- S3 Standard: $0.58/month
- **Total: $0.58/month**
- **Savings: $1.72/month (75%)**

At 10TB/month: **Save $172/month!**

#### File Extensions

Compressed files get correct extensions automatically:

```
batch-abc123.json.gz   # Gzip
batch-def456.json.zst  # Zstandard
```

This enables auto-decompression in analytics tools.

### When to Use Compression

**Use Gzip when:**
- Analytics tools require it (Athena, Redshift)
- Maximum compatibility needed
- Storage cost is primary concern

**Use Zstandard when:**
- Performance matters
- Modern analytics stack (Presto, Spark)
- Want best ratio with good speed

**Skip compression when:**
- Very small batches (< 1KB)
- CPU constrained
- Absolute minimum latency required
- Using Parquet (has internal compression)

### Customizing

Combine with Parquet:
```rust
.format(SerializationFormat::Parquet)
.compression(Compression::Zstd)
```

Larger batches compress better:
```rust
let config = PipelineConfig::builder()
    .batch_size(10000)  // Increased from default
    .build()?;
```

### Performance Benchmarks

**10,000 events (~50MB uncompressed):**

| Format | Size | Upload Time | Ratio | Throughput |
|--------|------|-------------|-------|------------|
| JSON | 50MB | 2.1s | - | 23.8 MB/s |
| JSON + Gzip | 12MB | 1.8s | 76% | 27.8 MB/s |
| JSON + Zstd | 10MB | 1.5s | 80% | 33.3 MB/s |

**Key insight:** Compression actually improves upload speed (less network I/O)!

**Next:** Try [s3_advanced.rs](#s3_advancedrs) for analytics patterns.

---

## s3_advanced.rs

**Production data lake patterns** with Parquet and Hive partitioning.

### What It Demonstrates

- Multiple serialization formats (JSON, CSV, Parquet, Avro)
- Hive-style partitioning for analytics
- Advanced compression strategies
- Production-grade retry configuration

### Running

```bash
# With all features
cargo run -p rigatoni-destinations --example s3_advanced --all-features

# Specific format
cargo run -p rigatoni-destinations --example s3_advanced --features s3,parquet,zstandard
```

### Key Concepts

#### Hive-Style Partitioning

Organize data for efficient analytics queries:

```rust
use rigatoni_destinations::s3::KeyGenerationStrategy;

let config = S3Config::builder()
    .key_strategy(KeyGenerationStrategy::HivePartitioned)
    .build()?;
```

**File structure:**
```
s3://bucket/prefix/
  collection=users/
    year=2025/
      month=01/
        day=21/
          hour=14/
            batch-abc123.parquet
```

**Benefits:**
- **Partition pruning:** Query only relevant partitions
- **Auto-discovery:** Athena/Presto detect partitions automatically
- **Cost savings:** Scan less data
- **Performance:** 10-100x faster queries

#### Serialization Formats

```rust
use rigatoni_destinations::s3::SerializationFormat;

.format(SerializationFormat::Json)     // Human-readable
.format(SerializationFormat::Csv)      // Simple, compatible
.format(SerializationFormat::Parquet)  // Columnar, best for analytics
.format(SerializationFormat::Avro)     // Row-based, schema evolution
```

**Format Comparison:**

| Format | Type | Size | Query Speed | Best For |
|--------|------|------|-------------|----------|
| JSON | Row | 1x | Slow | Development, debugging |
| CSV | Row | 0.5x | Slow | Exports, spreadsheets |
| Parquet | Columnar | **0.1x** | **Very Fast** | **Analytics, data lakes** |
| Avro | Row | 0.3x | Medium | Streaming, schema changes |

**Recommendation:** Use **Parquet** for production data lakes!

#### Key Generation Strategies

```rust
// Hive partitioning (recommended for analytics)
.key_strategy(KeyGenerationStrategy::HivePartitioned)

// Date-based (simpler)
.key_strategy(KeyGenerationStrategy::DateBased)

// UUID (no partitioning)
.key_strategy(KeyGenerationStrategy::Uuid)
```

### Analytics Integration

#### AWS Athena

```sql
CREATE EXTERNAL TABLE mongodb_users (
  _id INT,
  name STRING,
  created_at TIMESTAMP
)
PARTITIONED BY (collection STRING, year INT, month INT, day INT)
STORED AS PARQUET
LOCATION 's3://rigatoni-test-bucket/analytics/mongodb-cdc/';

-- Discover partitions
MSCK REPAIR TABLE mongodb_users;

-- Query specific partition (fast!)
SELECT * FROM mongodb_users
WHERE collection = 'users' AND year = 2025 AND month = 1;
```

#### Apache Spark

```python
df = spark.read.parquet("s3://bucket/analytics/mongodb-cdc/")

# Automatic partition pruning
users_today = df.filter(
    (df.collection == "users") &
    (df.year == 2025) &
    (df.month == 1)
)
```

### Production Best Practices

**Combine for optimal results:**

```rust
let config = S3Config::builder()
    .bucket("production-data-lake")
    .format(SerializationFormat::Parquet)           // Best for analytics
    .compression(Compression::Zstd)                  // Best compression
    .key_strategy(KeyGenerationStrategy::HivePartitioned)  // Best partitioning
    .max_retries(5)                                  // Production retries
    .build()?;
```

**Batch size considerations:**

```rust
let config = PipelineConfig::builder()
    .batch_size(10000)  // Larger batches = larger Parquet files
    .build()?;
```

**Ideal Parquet file size:** 128MB - 1GB per file

### Production Checklist

Before deploying to production:

- [ ] Use Parquet format
- [ ] Enable Hive partitioning
- [ ] Set batch size 10,000+
- [ ] Enable Zstd compression
- [ ] Configure S3 lifecycle (archive to Glacier)
- [ ] Set up AWS Glue Crawler
- [ ] Create Athena views
- [ ] Monitor upload failures
- [ ] Test partition pruning
- [ ] Document schema

### Cost Optimization

**Savings for 1TB/month:**
- Without optimization: $23/month
- With Parquet + Zstd + lifecycle: $2/month
- **Savings: 91%!**

**Next:** Review [S3 Configuration Guide](../../docs/guides/s3-configuration.md) and [Production Deployment](../../docs/guides/production-deployment.md).

---

## Additional Resources

- **[Examples Overview](../../examples/README.md)** - Quick start and common setup
- **[Core Examples](../../rigatoni-core/examples/README.md)** - Pipeline and metrics
- **[S3 Configuration Guide](../../docs/guides/s3-configuration.md)** - Detailed S3 setup
- **[Local Development Guide](../../docs/guides/local-development.md)** - Complete local setup
- **[API Documentation](https://docs.rs/rigatoni-destinations)** - API reference

---

## Feature Flags Reference

| Feature | Description | Required For |
|---------|-------------|--------------|
| `s3` | S3 destination | All S3 examples |
| `json` | JSON serialization | JSON format |
| `csv` | CSV serialization | CSV format |
| `parquet` | Parquet serialization | Parquet format (analytics) |
| `avro` | Avro serialization | Avro format |
| `gzip` | Gzip compression | Compressed uploads |
| `zstandard` | Zstandard compression | Better compression |

---

## Need Help?

- **[GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)** - Ask questions
- **[GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)** - Report problems
