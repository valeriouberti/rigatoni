# Rigatoni Examples

This directory contains comprehensive examples demonstrating how to use Rigatoni for real-world data replication scenarios.

> **Note**: This is an unpublished workspace member (`publish = false`) that serves as a container for examples requiring multiple Rigatoni crates. This avoids circular dependency issues in the published crates.

## Available Examples

### 1. Simple Pipeline with Memory Store

**File**: `simple_pipeline_memory.rs`

The simplest possible Rigatoni setup using an in-memory state store.

```bash
cargo run --example simple_pipeline_memory
```

### 2. Change Stream Listener

**File**: `change_stream_listener.rs`

Direct usage of the MongoDB change stream listener.

```bash
cargo run --example change_stream_listener
```

### 3. Prometheus Metrics

**File**: `metrics_prometheus.rs`

Pipeline with Prometheus metrics export.

```bash
cargo run --example metrics_prometheus --features metrics-export
```

### 4. S3 Basic

**File**: `s3_basic.rs`

Basic S3 destination with JSON serialization and no compression.

```bash
cargo run --example s3_basic -p rigatoni-examples
```

### 5. S3 with Compression

**File**: `s3_with_compression.rs`

S3 destination with Zstandard compression for reduced storage costs.

```bash
cargo run --example s3_with_compression -p rigatoni-examples
```

### 6. S3 Advanced

**File**: `s3_advanced.rs`

Advanced S3 destination with Parquet format and Hive-style partitioning for analytics.

```bash
cargo run --example s3_advanced -p rigatoni-examples
```

### 7. Multi-Instance with Redis Distributed Locking

**File**: `multi_instance_redis.rs`

Demonstrates horizontal scaling with multiple Rigatoni instances using Redis-based distributed locking to prevent duplicate event processing.

```bash
# Terminal 1
INSTANCE_NAME=instance-1 cargo run --example multi_instance_redis

# Terminal 2
INSTANCE_NAME=instance-2 cargo run --example multi_instance_redis

# Terminal 3
INSTANCE_NAME=instance-3 cargo run --example multi_instance_redis
```

Each instance automatically acquires locks for available collections. If an instance crashes, its locks expire after TTL (30s) and other instances take over.

## Prerequisites

All examples require:

- **Rust 1.88+**
- **MongoDB replica set** (change streams require replica set mode)

S3 examples additionally require:
- **AWS credentials** configured (or LocalStack for local testing)
- **S3 bucket** (or LocalStack S3 service)

Multi-instance examples additionally require:
- **Redis** (for distributed locking)

### Start MongoDB

```bash
docker run -d --name mongodb -p 27017:27017 \
  mongo:7.0 --replSet rs0

# Initialize replica set
docker exec mongodb mongosh --eval "rs.initiate()"
```

### Start LocalStack (for S3 examples)

```bash
# Using Docker
docker run -d --name localstack -p 4566:4566 localstack/localstack

# Create test bucket
awslocal s3 mb s3://rigatoni-test-bucket
```

### Start Redis (for multi-instance examples)

```bash
docker run -d --name redis -p 6379:6379 redis:7.0
```

## Running Examples

From the workspace root:

```bash
# Run any example
cargo run --example <example-name>

# With features
cargo run --example metrics_prometheus --features metrics-export

# With specific log level
RUST_LOG=debug cargo run --example simple_pipeline_memory
```
