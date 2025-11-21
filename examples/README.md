# Rigatoni Examples

This directory serves as a guide to all examples across the Rigatoni workspace. Examples are organized by crate to follow Rust best practices and maintain clear dependency boundaries.

## Quick Links

- [Core Examples](#core-examples) - Pipeline orchestration, change streams, metrics
- [Destination Examples](#destination-examples) - S3 configurations and features
- [Getting Started](#getting-started) - Recommended examples for beginners

---

## Getting Started

### 🚀 Start Here: Simplest Example

**[simple_pipeline_memory.rs](../rigatoni-core/examples/simple_pipeline_memory.rs)** - Absolute minimum setup

```bash
# Just MongoDB required (no Redis, S3, Prometheus)
docker run -d --name mongodb -p 27017:27017 mongo:7.0 --replSet rs0
docker exec mongodb mongosh --eval "rs.initiate()"

# Run the example
cargo run -p rigatoni-core --example simple_pipeline_memory
```

**What you'll learn:**
- Basic pipeline setup
- In-memory state store (no external dependencies)
- Console output destination
- MongoDB change stream integration

**Perfect for:** First-time users, quick experiments, understanding core concepts

---

### 📊 Full Stack Example

**[metrics_prometheus.rs](../rigatoni-core/examples/metrics_prometheus.rs)** - Complete production-like setup

```bash
# Start all services
cd docker && docker compose up -d

# Run the example
cargo run -p rigatoni-core --example metrics_prometheus --features metrics-export

# Generate test data
./docker/scripts/generate-test-data.sh
```

**What you'll learn:**
- Redis state store for distributed deployments
- S3 destination with LocalStack
- Prometheus metrics integration
- Grafana dashboards
- Complete observability setup

**Perfect for:** Production preparation, learning best practices, full feature exploration

---

## Core Examples

Located in [`rigatoni-core/examples/`](../rigatoni-core/examples/)

### 1. simple_pipeline_memory.rs

**Difficulty:** ⭐ Beginner
**Dependencies:** MongoDB only
**Run time:** `cargo run -p rigatoni-core --example simple_pipeline_memory`

The simplest possible Rigatoni setup. Perfect starting point for beginners.

**Features:**
- In-memory state store (no Redis)
- Console destination (prints to terminal)
- Minimal configuration
- Real-time event visualization

**Use when:**
- Learning Rigatoni for the first time
- Quick local experiments
- Testing pipeline logic
- No infrastructure setup desired

**Related docs:**
- [Local Development Guide - Minimal Setup](../docs/guides/local-development.md#minimal-setup-mongodb-only)
- [In-Memory StateStore API](https://docs.rs/rigatoni-stores/latest/rigatoni_stores/memory/)

---

### 2. metrics_prometheus.rs

**Difficulty:** ⭐⭐ Intermediate
**Dependencies:** MongoDB, Redis, LocalStack, Prometheus, Grafana
**Run time:** `cargo run -p rigatoni-core --example metrics_prometheus --features metrics-export`

Complete production-like example with full observability stack.

**Features:**
- Redis state store for fault tolerance
- S3 destination with LocalStack
- Prometheus metrics export
- Grafana dashboard integration
- Graceful shutdown handling

**Use when:**
- Preparing for production deployment
- Learning observability patterns
- Testing distributed state management
- Building monitoring dashboards

**Related docs:**
- [Local Development Guide](../docs/guides/local-development.md)
- [Observability Guide](../docs/OBSERVABILITY.md)
- [Redis Configuration](../docs/guides/redis-configuration.md)

---

### 3. change_stream_listener.rs

**Difficulty:** ⭐⭐ Intermediate
**Dependencies:** MongoDB
**Run time:** `cargo run -p rigatoni-core --example change_stream_listener`

Low-level change stream API examples demonstrating direct usage without the full pipeline.

**Features:**
- Basic change stream listener
- Filtering change events
- Resume token handling
- Multi-collection watching

**Use when:**
- Understanding MongoDB change streams
- Building custom integrations
- Need lower-level control
- Learning the underlying mechanisms

**Related docs:**
- [Architecture - Change Streams](../docs/architecture.md)
- [ChangeStreamListener API](https://docs.rs/rigatoni-core/latest/rigatoni_core/stream/)

---

## Destination Examples

Located in [`rigatoni-destinations/examples/`](../rigatoni-destinations/examples/)

### 1. s3_basic.rs

**Difficulty:** ⭐ Beginner
**Dependencies:** AWS credentials or LocalStack
**Run time:** `cargo run -p rigatoni-destinations --example s3_basic --features s3,json`

Simplest S3 destination usage with JSON serialization.

**Features:**
- Basic S3 configuration
- JSON serialization
- No compression
- Default partitioning

**Use when:**
- First time using S3 destination
- Simple data lake setup
- Learning S3 destination basics

**Environment setup:**
```bash
export S3_BUCKET="your-bucket-name"
export AWS_REGION="us-east-1"
# AWS credentials via ~/.aws/credentials or environment
```

---

### 2. s3_with_compression.rs

**Difficulty:** ⭐⭐ Intermediate
**Dependencies:** AWS credentials or LocalStack
**Run time:** `cargo run -p rigatoni-destinations --example s3_with_compression --features s3,json,gzip`

Demonstrates compression options for reducing storage costs and improving performance.

**Features:**
- Gzip compression
- Zstandard compression
- Compression trade-offs
- Storage optimization

**Use when:**
- Optimizing storage costs
- Improving upload performance
- Large data volumes
- Bandwidth constraints

---

### 3. s3_advanced.rs

**Difficulty:** ⭐⭐⭐ Advanced
**Dependencies:** AWS credentials or LocalStack
**Run time:** `cargo run -p rigatoni-destinations --example s3_advanced --features s3,json,parquet,gzip,zstandard`

Advanced S3 features including multiple formats, compression, and partitioning strategies.

**Features:**
- Multiple serialization formats (JSON, Parquet)
- Different compression algorithms
- Hive-style partitioning
- Date-based partitioning
- Custom key generation strategies

**Use when:**
- Building production data lakes
- Analytics workloads
- Need partitioning for performance
- Working with data warehouses

**Related docs:**
- [S3 Configuration Guide](../docs/guides/s3-configuration.md)

---

## Running Examples by Use Case

### Local Development / Learning

```bash
# Simplest setup - just MongoDB
cargo run -p rigatoni-core --example simple_pipeline_memory
```

### Testing with S3

```bash
# Start LocalStack
docker compose -f docker/docker-compose.localstack.yml up -d

# Run S3 examples
cargo run -p rigatoni-destinations --example s3_basic --features s3,json
```

### Full Production-Like Setup

```bash
# Start all services
docker compose -f docker/docker-compose.yml up -d

# Run full stack example
cargo run -p rigatoni-core --example metrics_prometheus --features metrics-export
```

### Understanding Change Streams

```bash
# Just MongoDB needed
cargo run -p rigatoni-core --example change_stream_listener
```

---

## Example Matrix

| Example | MongoDB | Redis | S3 | Metrics | Difficulty | Time to Run |
|---------|---------|-------|----|---------| ----------|-------------|
| simple_pipeline_memory | ✅ | ❌ | ❌ | ❌ | ⭐ | < 5 min |
| change_stream_listener | ✅ | ❌ | ❌ | ❌ | ⭐⭐ | < 5 min |
| s3_basic | ❌ | ❌ | ✅ | ❌ | ⭐ | < 5 min |
| s3_with_compression | ❌ | ❌ | ✅ | ❌ | ⭐⭐ | < 5 min |
| s3_advanced | ❌ | ❌ | ✅ | ❌ | ⭐⭐⭐ | < 5 min |
| metrics_prometheus | ✅ | ✅ | ✅ | ✅ | ⭐⭐ | 10-15 min |

---

## Prerequisites by Example

### Minimal (simple_pipeline_memory)
- Docker
- Rust 1.85+

### Standard (most examples)
- Docker
- Rust 1.85+
- AWS credentials (or LocalStack)

### Full Stack (metrics_prometheus)
- Docker & Docker Compose
- Rust 1.85+
- awslocal (optional but recommended)

---

## Common Issues

### MongoDB Not in Replica Set Mode

Many examples require MongoDB to be in replica set mode for change streams:

```bash
# Start MongoDB with replica set
docker run -d --name mongodb -p 27017:27017 mongo:7.0 --replSet rs0

# Initialize replica set
docker exec mongodb mongosh --eval "rs.initiate()"
```

### LocalStack S3 Bucket Not Found

Create buckets before running S3 examples:

```bash
awslocal s3 mb s3://rigatoni-test-bucket
```

### Redis Connection Failed

Make sure Redis is running:

```bash
docker compose -f docker/docker-compose.yml up -d redis
```

---

## Learning Path

**Recommended progression for learning Rigatoni:**

1. **Start Simple** → `simple_pipeline_memory.rs`
   - Get comfortable with basic pipeline concepts
   - See real-time change stream events
   - No infrastructure complexity

2. **Add S3** → `s3_basic.rs`
   - Learn destination patterns
   - Understand serialization
   - See data persistence

3. **Add Compression** → `s3_with_compression.rs`
   - Optimize storage
   - Compare compression algorithms
   - Production considerations

4. **Full Observability** → `metrics_prometheus.rs`
   - Production monitoring
   - Grafana dashboards
   - Distributed state with Redis

5. **Advanced Features** → `s3_advanced.rs` & `change_stream_listener.rs`
   - Custom partitioning
   - Low-level control
   - Performance optimization

---

## Contributing Examples

When adding new examples:

1. **Place in appropriate crate** - Keep examples with the features they demonstrate
2. **Include comprehensive documentation** - Explain what, why, and when
3. **Minimize dependencies** - Only require what's necessary
4. **Add error handling** - Show proper error handling patterns
5. **Update this README** - Add entry with metadata
6. **Test thoroughly** - Ensure example works from clean state

---

## Additional Resources

- **[Documentation](https://valeriouberti.github.io/rigatoni/)** - Full documentation site
- **[API Reference](https://docs.rs/rigatoni)** - Complete API documentation
- **[Architecture Guide](../docs/architecture.md)** - System design and concepts
- **[Getting Started](../docs/getting-started.md)** - Step-by-step tutorial
- **[Local Development](../docs/guides/local-development.md)** - Complete local setup guide

---

## Need Help?

- **Issues:** [GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)
- **Discussions:** [GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)
- **Documentation:** [guides/](../docs/guides/)

---

## Quick Reference Commands

```bash
# List all examples
cargo run --example

# Run specific example
cargo run -p <crate-name> --example <example-name>

# With features
cargo run -p <crate-name> --example <example-name> --features <features>

# See example help
cargo run -p <crate-name> --example <example-name> -- --help
```
