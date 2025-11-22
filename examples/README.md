# Rigatoni Examples

Welcome to Rigatoni examples! This guide helps you find the right example for your needs.

## Quick Start

### 🚀 Simplest Example (5 minutes)

**[simple_pipeline_memory.rs](../rigatoni-core/examples/simple_pipeline_memory.rs)** - MongoDB only, no external dependencies

```bash
# Start MongoDB
docker run -d --name mongodb -p 27017:27017 mongo:7.0 --replSet rs0
docker exec mongodb mongosh --eval "rs.initiate()"

# Run example
cargo run -p rigatoni-core --example simple_pipeline_memory
```

**Perfect for:** First-time users, learning basics, quick experiments

---

### 📊 Full Stack Example (15 minutes)

**[metrics_prometheus.rs](../rigatoni-core/examples/metrics_prometheus.rs)** - Production-like setup

```bash
# Start all services
cd docker && docker compose up -d

# Run example
cargo run -p rigatoni-core --example metrics_prometheus --features metrics-export

# Generate test data
./docker/scripts/generate-test-data.sh
```

**Perfect for:** Production prep, observability, full feature exploration

---

## All Examples

### Core Examples
**Location:** [`rigatoni-core/examples/`](../rigatoni-core/examples/)

| Example | Difficulty | Dependencies | Description |
|---------|-----------|--------------|-------------|
| **simple_pipeline_memory.rs** | ⭐ Beginner | MongoDB | In-memory state, console output |
| **metrics_prometheus.rs** | ⭐⭐ Intermediate | Full stack | Redis, S3, Prometheus, Grafana |
| **change_stream_listener.rs** | ⭐⭐ Intermediate | MongoDB | Low-level change stream API |

**[View detailed core examples documentation →](../rigatoni-core/examples/README.md)**

---

### Destination Examples
**Location:** [`rigatoni-destinations/examples/`](../rigatoni-destinations/examples/)

| Example | Difficulty | Key Features | Description |
|---------|-----------|--------------|-------------|
| **s3_basic.rs** | ⭐ Beginner | JSON, LocalStack | Simple S3 upload |
| **s3_with_compression.rs** | ⭐⭐ Intermediate | Gzip, Zstd | Compression strategies |
| **s3_advanced.rs** | ⭐⭐⭐ Advanced | Parquet, partitioning | Analytics data lake |

**[View detailed destination examples documentation →](../rigatoni-destinations/examples/README.md)**

---

## Common Setup

### Prerequisites

- **Rust 1.85+**
- **Docker** and Docker Compose
- **awslocal** (optional, for S3 examples)

### MongoDB Setup (Required for Core Examples)

```bash
# Start MongoDB replica set
docker run -d --name mongodb -p 27017:27017 mongo:7.0 --replSet rs0

# Initialize replica set
docker exec mongodb mongosh --eval "rs.initiate()"

# Verify
docker exec mongodb mongosh --eval "rs.status()"
```

### Full Stack Setup (For metrics_prometheus Example)

```bash
# Start all services
cd docker
docker compose up -d

# Verify all services are running
docker compose ps
```

Services started:
- MongoDB (port 27017)
- Redis (port 6379)
- LocalStack S3 (port 4566)
- Prometheus (port 9090)
- Grafana (port 3000)

### LocalStack Setup (For S3 Examples)

```bash
# Start LocalStack
docker compose -f docker/docker-compose.localstack.yml up -d

# Create test bucket
awslocal s3 mb s3://rigatoni-test-bucket

# Verify
awslocal s3 ls
```

**Environment variables:**
```bash
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
```

---

## Running Examples

### Core Examples

```bash
# Simple pipeline
cargo run -p rigatoni-core --example simple_pipeline_memory

# Full stack with metrics
cargo run -p rigatoni-core --example metrics_prometheus --features metrics-export

# Change stream listener
cargo run -p rigatoni-core --example change_stream_listener
```

### Destination Examples

```bash
# Basic S3
cargo run -p rigatoni-destinations --example s3_basic --features s3,json

# With compression
cargo run -p rigatoni-destinations --example s3_with_compression --features s3,json,gzip

# Advanced (all features)
cargo run -p rigatoni-destinations --example s3_advanced --all-features
```

---

## Example Matrix

| Example | MongoDB | Redis | S3 | Metrics | Time | Difficulty |
|---------|---------|-------|----|---------| -----|------------|
| simple_pipeline_memory | ✅ | ❌ | ❌ | ❌ | < 5 min | ⭐ |
| change_stream_listener | ✅ | ❌ | ❌ | ❌ | < 5 min | ⭐⭐ |
| s3_basic | ❌ | ❌ | ✅ | ❌ | < 5 min | ⭐ |
| s3_with_compression | ❌ | ❌ | ✅ | ❌ | < 5 min | ⭐⭐ |
| s3_advanced | ❌ | ❌ | ✅ | ❌ | < 5 min | ⭐⭐⭐ |
| metrics_prometheus | ✅ | ✅ | ✅ | ✅ | 10-15 min | ⭐⭐ |

---

## Learning Path

**Recommended progression:**

1. **Start Simple** → [simple_pipeline_memory.rs](../rigatoni-core/examples/simple_pipeline_memory.rs)
   - Learn pipeline basics without infrastructure complexity

2. **Add S3** → [s3_basic.rs](../rigatoni-destinations/examples/s3_basic.rs)
   - Understand destinations and data persistence

3. **Add Compression** → [s3_with_compression.rs](../rigatoni-destinations/examples/s3_with_compression.rs)
   - Optimize storage costs

4. **Full Observability** → [metrics_prometheus.rs](../rigatoni-core/examples/metrics_prometheus.rs)
   - Production monitoring and distributed state

5. **Advanced** → [s3_advanced.rs](../rigatoni-destinations/examples/s3_advanced.rs) & [change_stream_listener.rs](../rigatoni-core/examples/change_stream_listener.rs)
   - Custom partitioning and low-level control

---

## Common Issues

### MongoDB: "change stream is only supported on replica sets"

**Solution:**
```bash
docker exec mongodb mongosh --eval "rs.initiate()"
```

### LocalStack: Bucket not found

**Solution:**
```bash
awslocal s3 mb s3://rigatoni-test-bucket
```

### Redis: Connection refused

**Solution:**
```bash
docker compose -f docker/docker-compose.yml up -d redis
```

### Example not found

**Solution:** Check you're using the correct package name:
```bash
cargo run -p rigatoni-core --example <name>        # For core examples
cargo run -p rigatoni-destinations --example <name> # For destination examples
```

---

## Detailed Documentation

- **[Core Examples Guide](../rigatoni-core/examples/README.md)** - Detailed docs for pipeline, metrics, and change stream examples
- **[Destination Examples Guide](../rigatoni-destinations/examples/README.md)** - Detailed docs for S3 examples with compression and partitioning
- **[Local Development Guide](../docs/guides/local-development.md)** - Complete local setup with docker-compose
- **[S3 Configuration Guide](../docs/guides/s3-configuration.md)** - Production S3 patterns
- **[Observability Guide](../docs/OBSERVABILITY.md)** - Metrics and monitoring

---

## Need Help?

- **[GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)** - Ask questions
- **[GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)** - Report problems
- **[Documentation](https://valeriouberti.github.io/rigatoni)** - Full docs

---

**Quick Reference:**

```bash
# List all examples
cargo run --example

# Run with features
cargo run -p <crate> --example <name> --features <features>

# See example source
cat rigatoni-core/examples/<name>.rs
```
