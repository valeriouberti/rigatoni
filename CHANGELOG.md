# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Improved S3 configuration error handling with structured `S3ConfigError` enum
- Optimized S3 compression API to accept `&[u8]` instead of `Vec<u8>` to avoid cloning

## [0.1.0] - 2025-11-16

### Added

#### Core Pipeline (`rigatoni-core`)

**Pipeline Orchestration:**

- Multi-worker architecture for concurrent processing of multiple collections
- Configurable batching based on size and timeout (dual-trigger strategy)
- Exponential backoff retry logic with configurable limits and jitter
- Graceful shutdown with proper cleanup and resource management
- Resume token persistence for exactly-once/at-least-once semantics
- Pipeline statistics tracking (events processed, retries, errors, etc.)
- Structured error handling with `PipelineError` enum

**Configuration:**

- Builder pattern for type-safe configuration
- Compile-time validation of required fields
- Smart defaults for batch size, timeouts, and retry settings
- Structured `ConfigError` enum for better error handling
- Environment variable support

**MongoDB Source:**

- MongoDB change stream integration with replica set support
- Real-time CDC (Change Data Capture) capabilities
- Resume token management for fault tolerance
- Support for watching specific collections or all collections
- Automatic reconnection on transient failures
- Operation type filtering (Insert, Update, Delete, Replace)

**Event Model:**

- `ChangeEvent` struct representing MongoDB change stream events
- `Namespace` for database and collection tracking
- `OperationType` enum for event classification
- Full document and update description support
- Cluster time tracking for ordering

**Destination Trait:**

- Generic `Destination` trait for pluggable outputs
- Async-first API with `write_batch`, `flush`, and `close` methods
- Metadata support for destination capabilities
- Buffered event tracking
- Error handling with `DestinationError` enum

**State Store Trait:**

- `StateStore` trait for resume token persistence
- In-memory store implementation
- File-based store implementation
- Support for custom store implementations

#### S3 Destination (`rigatoni-destinations`)

**Core Features:**

- AWS S3 integration with official AWS SDK
- Buffered batch writing with automatic flushing
- Resume token checkpoint support
- Comprehensive error handling and retry logic
- S3-compatible storage support (LocalStack, MinIO)

**Serialization Formats:**

- **JSON (JSONL)** - Newline-delimited JSON (default)
- **CSV** - Comma-separated values with header row
- **Parquet** - Apache Parquet columnar format (with `parquet` feature)
- **Avro** - Apache Avro binary format (with `avro` feature)

**Compression:**

- **None** - No compression (default)
- **Gzip** - RFC 1952 compression (with `gzip` feature)
- **Zstandard** - Modern compression with better ratio (with `zstandard` feature)

**Key Generation Strategies:**

- `HivePartitioned` - Hive-style partitioning for analytics platforms
  - Pattern: `collection=name/year=YYYY/month=MM/day=DD/hour=HH/timestamp.ext`
- `DateHourPartitioned` - Time-based partitioning with hour granularity (default)
  - Pattern: `collection/YYYY/MM/DD/HH/timestamp.ext`
- `DatePartitioned` - Daily partitioning without hour
  - Pattern: `collection/YYYY/MM/DD/timestamp.ext`
- `CollectionBased` - Simple collection grouping
  - Pattern: `collection/timestamp.ext`
- `Flat` - Flat structure with timestamp
  - Pattern: `collection_timestamp.ext`

**Configuration:**

- Type-safe builder pattern with validation
- Structured `S3ConfigError` enum for better error handling
- Bucket name validation (3-63 characters, lowercase only)
- Prefix validation (no path traversal, no leading slash)
- Configurable retry attempts (default: 3)
- Custom endpoint URL for S3-compatible storage
- Force path-style addressing option

**AWS Integration:**

- Automatic credential discovery (environment, profile, IAM role)
- Regional endpoint support
- Path-style and virtual-hosted style addressing
- Retry logic for throttling and transient errors

#### State Stores (`rigatoni-stores`)

**Memory Store:**

- Fast in-memory resume token storage
- Thread-safe with `Arc<RwLock<HashMap>>`
- Ideal for testing and development
- No persistence (data lost on restart)

**File Store:**

- JSON-based file persistence
- Automatic directory creation
- One file per collection
- Human-readable format for debugging

#### Feature Flags

**Workspace:**

- Modular crate structure (core, destinations, stores)
- Workspace-level dependency management
- Shared metadata and versioning
- Resolver v2 for better feature handling

**Destinations Features:**

- `s3` - AWS S3 destination (enabled by default)
- `bigquery` - Google BigQuery destination (planned)
- `kafka` - Apache Kafka destination (planned)
- `json` - JSON/JSONL format support (enabled by default)
- `csv` - CSV format support
- `parquet` - Apache Parquet format support
- `avro` - Apache Avro format support
- `gzip` - Gzip compression support
- `zstandard` - Zstandard compression support
- `compression` - All compression formats
- `all-destinations` - All destination implementations
- `all-formats` - All serialization formats
- `all` - Everything (destinations, formats, compression)

**Stores Features:**

- `memory` - In-memory store (enabled by default)
- `file` - File-based store (enabled by default)
- `redis-store` - Redis-based distributed store (planned)
- `all-stores` - All store implementations

### Development Tools

**Testing:**

- Comprehensive unit tests for core logic
- Integration tests with LocalStack for S3
- MockDestination for testing pipelines
- Test utilities and fixtures
- Example programs for each feature

**Code Quality:**

- Strict Clippy linting with pedantic mode
- Rustfmt for consistent formatting
- Security auditing with `cargo-audit`
- License compliance checking with `cargo-deny`
- Pre-push script for local validation

**CI/CD:**

- GitHub Actions workflow with multiple jobs
- Parallel test execution
- Cross-platform testing (planned)
- Automated dependency updates (Dependabot)
- Security scanning on every commit
- Automated release to crates.io on version tags

**Documentation:**

- Comprehensive API documentation with rustdoc
- GitHub Pages documentation site
- Getting Started guide
- Architecture deep-dive
- Production deployment guide
- S3 configuration guide
- Contributing guidelines
- Examples for common use cases

### Performance Characteristics

**Throughput:**

- Single worker: ~5,000-10,000 events/second
- Multi-worker (4 collections): ~20,000-40,000 events/second
- Batching optimizations for high-volume streams

**Latency:**

- Best case: 100-500ms (immediate batch flush)
- Typical: 1-5 seconds (batch timeout)
- Configurable based on use case (throughput vs latency)

**Memory Usage:**

- Base overhead: ~10 MB
- Per worker: ~5 MB
- Per batch: ~0.5 MB (1000 events × 500 bytes)
- Example: 32 MB for 4 workers with 1000 batch size

**Scalability:**

- Horizontal: Multiple pipeline instances with different collections
- Vertical: More workers per collection for parallelism
- Efficient resource utilization on multi-core systems

### Examples Included

**Core Examples:**

- `pipeline_basic` - Simple MongoDB to destination pipeline
- `mock_destination` - Testing with mock destination
- `custom_destination` - Implementing custom destinations

**S3 Examples:**

- `s3_basic` - Basic S3 usage with default settings
- `s3_advanced` - Advanced features (formats, compression, partitioning)
- `s3_with_compression` - Compression examples (gzip, zstandard)

### Integration Tests

**LocalStack Integration:**

- Docker Compose setup for local S3 testing
- Integration tests for all S3 features
- Format-specific tests (JSON, CSV, Parquet, Avro)
- Compression tests (Gzip, Zstandard)
- Partitioning strategy tests
- Metadata validation tests

### Security

**Vulnerability Scanning:**

- Automated security audits with `cargo-audit`
- Daily dependency vulnerability checks
- License compliance enforcement
- No known vulnerabilities in dependencies

**Best Practices:**

- No hardcoded credentials
- AWS SDK credential chain support
- IAM role support for cloud deployments
- Secrets management via environment variables
- Path traversal prevention in S3 prefixes

### Known Limitations

- MongoDB change streams require replica set mode
- "Watch all collections" mode not yet implemented (early validation added)
- S3 uploads are sequential per batch (parallel uploads planned)
- No exactly-once semantics for destinations without transaction support
- No built-in metrics export (Prometheus integration planned)

### Dependencies

**Core:**

- `tokio` (1.40) - Async runtime
- `mongodb` (3.3) - MongoDB driver
- `serde`/`serde_json` (1.0) - Serialization
- `tracing` (0.1) - Structured logging
- `async-trait` (0.1) - Async trait support
- `thiserror` (1.0) - Error handling
- `chrono` (0.4) - Time handling

**S3 Destination:**

- `aws-config` (1.8) - AWS SDK configuration
- `aws-sdk-s3` (1.112) - S3 client
- `parquet` (57.0) - Parquet format (optional)
- `apache-avro` (0.20) - Avro format (optional)
- `csv` (1.3) - CSV format (optional)
- `flate2` (1.0) - Gzip compression (optional)
- `zstd` (0.13) - Zstandard compression (optional)

**Development:**

- `mockall` (0.13) - Mocking framework
- `tokio-test` (0.4) - Async testing utilities
- `tracing-subscriber` (0.3) - Logging backend

### License

Dual-licensed under MIT OR Apache-2.0.

---

## Release Notes

### v0.1.0 - Initial Release

This is the first public release of Rigatoni, a high-performance ETL framework for Rust focused on real-time data pipelines.

**Highlights:**

- ✅ Production-ready MongoDB change stream source
- ✅ AWS S3 destination with multiple formats and compression
- ✅ Type-safe pipeline orchestration with retry logic
- ✅ Modular architecture with feature flags
- ✅ Comprehensive documentation and examples
- ✅ Extensive test coverage

**Use Cases:**

- Real-time CDC from MongoDB to S3 data lake
- Continuous backup and archival
- Event sourcing and audit logging
- Analytics data pipeline

**Getting Started:**

```toml
[dependencies]
rigatoni-core = "0.1"
rigatoni-destinations = { version = "0.1", features = ["s3"] }
```

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_destinations::s3::{S3Config, S3Destination};

let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017")
    .database("mydb")
    .collections(vec!["users"])
    .build()?;

let s3_config = S3Config::builder()
    .bucket("my-bucket")
    .region("us-east-1")
    .build()?;

let destination = S3Destination::new(s3_config).await?;
let mut pipeline = Pipeline::new(config, destination).await?;
pipeline.run().await?;
```

**Documentation:**

- Getting Started: https://valeriouberti.github.io/rigatoni/getting-started
- API Docs: https://docs.rs/rigatoni
- Examples: https://github.com/valeriouberti/rigatoni/tree/main/examples

**What's Next:**

- BigQuery destination
- Kafka destination
- Metrics export (Prometheus/OpenTelemetry)
- Filtering and transformations
- Schema evolution support
- Exactly-once semantics for supported destinations

---

[Unreleased]: https://github.com/valeriouberti/rigatoni/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/valeriouberti/rigatoni/releases/tag/v0.1.0
