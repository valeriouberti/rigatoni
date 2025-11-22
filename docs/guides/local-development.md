---
layout: default
title: Local Development with Docker Compose
parent: Guides
nav_order: 6
description: "Complete guide to running Rigatoni locally with MongoDB, Redis, LocalStack, Prometheus, and Grafana."
permalink: /guides/local-development
---

# Local Development with Docker Compose
{: .no_toc }

Run a complete Rigatoni pipeline locally with MongoDB, Redis, LocalStack S3, Prometheus, and Grafana.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

This guide shows you how to run a complete, production-like Rigatoni environment on your local machine using Docker Compose.

**What you'll get:** MongoDB (replica set), Redis, LocalStack S3, Prometheus, Grafana, and optional web UIs for MongoDB and Redis.

**Perfect for:** Development, testing, learning Rigatoni, and experimenting with configurations.

> **📖 Technical Reference**
>
> For detailed information about Docker Compose files, service configurations, and advanced options, see **[docker/README.md](../../docker/README.md)**.
>
> This guide focuses on the step-by-step workflow. The docker README provides technical details about each service, ports, configuration options, and troubleshooting.

> **💡 Want an even simpler setup?**
>
> You can skip Redis entirely by using the in-memory state store. Just run:
> ```bash
> cargo run --example simple_pipeline_memory
> ```
>
> This requires only MongoDB (no Redis, LocalStack, Prometheus, or Grafana).
> Perfect for quick experiments! See the [Minimal Setup](#minimal-setup-mongodb-only) section below.

---

## Prerequisites

Before you begin, ensure you have:

- **Docker** (20.10+) - [Install Docker](https://docs.docker.com/get-docker/)
- **Docker Compose** (v2.0+) - Included with Docker Desktop
- **Rust** (1.85+) - [Install Rust](https://rustup.rs/)
- **awslocal** (optional but recommended) - `pip install awscli-local`

### Verify Installation

```bash
docker --version
# Docker version 24.0.0 or later

docker compose version
# Docker Compose version v2.20.0 or later

rustc --version
# rustc 1.85.0 or later
```

---

## Quick Start

The fastest way to get started:

### 1. Clone the Repository (if you haven't already)

```bash
git clone https://github.com/valeriouberti/rigatoni.git
cd rigatoni
```

### 2. Run the Quick Start Script

```bash
./tools/local-development/scripts/quick-start-local.sh
```

This script will:
1. Check prerequisites
2. Start all Docker services
3. Wait for services to be healthy
4. Display service URLs and next steps

### 3. Run the Example Pipeline

```bash
cargo run --example metrics_prometheus --features metrics-export
```

### 4. Generate Test Data

In a new terminal:

```bash
./tools/local-development/scripts/generate-test-data.sh
```

### 5. View the Results

Open your browser to:
- **Grafana Dashboard**: [http://localhost:3000](http://localhost:3000) (login: admin/admin)
- **Prometheus**: [http://localhost:9090](http://localhost:9090)
- **MongoDB Express**: [http://localhost:8081](http://localhost:8081)
- **Metrics Endpoint**: [http://localhost:9000/metrics](http://localhost:9000/metrics)

That's it! You now have a fully functional Rigatoni pipeline with observability.

---

## Manual Setup (Step by Step)

If you prefer to understand each step or customize the setup:

> **📖 Note:** This section focuses on the workflow. For technical details about individual services, configuration options, and advanced customization, refer to [docker/README.md](../../docker/README.md).

### Step 1: Start Docker Services

```bash
cd docker
docker compose up -d
```

This starts all services (MongoDB, Redis, LocalStack, Prometheus, Grafana, and web UIs).

> **📖 Service Details:** For ports, credentials, and configuration of each service, see [docker/README.md](../../docker/README.md)

### Step 2: Verify Services are Running

```bash
docker compose -f docker/docker-compose.yml ps
```

All services should show status as "healthy" or "running":

```
NAME                       STATUS
rigatoni-mongodb           Up (healthy)
rigatoni-redis             Up (healthy)
rigatoni-localstack        Up (healthy)
rigatoni-prometheus        Up (healthy)
rigatoni-grafana           Up (healthy)
rigatoni-mongo-express     Up
rigatoni-redis-commander   Up
```

### Step 3: Wait for MongoDB Replica Set

The MongoDB container initializes a replica set automatically. Wait about 30 seconds, then verify:

```bash
docker exec rigatoni-mongodb mongosh --quiet --eval "rs.status()"
```

You should see replica set status with PRIMARY node.

### Step 4: Create Test Database and Collection

```bash
docker exec -it rigatoni-mongodb mongosh
```

In the MongoDB shell:

```javascript
use testdb

// Create collections
db.createCollection("users")
db.createCollection("orders")
db.createCollection("products")

// Insert sample data
db.users.insertMany([
  { name: "Alice Smith", email: "alice@example.com", age: 30, city: "New York" },
  { name: "Bob Johnson", email: "bob@example.com", age: 25, city: "San Francisco" },
  { name: "Carol Williams", email: "carol@example.com", age: 35, city: "Austin" }
])

db.orders.insertMany([
  { userId: 1, product: "Widget", amount: 29.99, status: "completed" },
  { userId: 2, product: "Gadget", amount: 49.99, status: "pending" }
])

db.products.insertMany([
  { name: "Widget", price: 29.99, category: "Electronics" },
  { name: "Gadget", price: 49.99, category: "Electronics" }
])

exit
```

### Step 5: Verify LocalStack S3 Bucket

The initialization script creates a test bucket automatically:

```bash
awslocal s3 ls
# Should show: rigatoni-test-bucket
```

If the bucket doesn't exist, create it:

```bash
awslocal s3 mb s3://rigatoni-test-bucket
```

### Step 6: Set Environment Variables

The example uses these defaults, but you can customize:

```bash
export MONGODB_URI="mongodb://localhost:27017/?replicaSet=rs0&directConnection=true"
export REDIS_URL="redis://:redispassword@localhost:6379"
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_REGION="us-east-1"
```

Or use the setup script:

```bash
source ./tools/local-development/scripts/setup-local-env.sh
```

### Step 7: Build the Example

```bash
cargo build --example metrics_prometheus --features metrics-export
```

### Step 8: Run the Pipeline

```bash
cargo run --example metrics_prometheus --features metrics-export
```

You should see output like:

```
🚀 Starting Rigatoni with Prometheus Metrics Exporter
📊 Starting Prometheus exporter on http://0.0.0.0:9000
✅ Prometheus metrics available at http://localhost:9000/metrics
🔧 Configuring Redis state store...
✅ Redis connection established
🔧 Configuring S3 destination...
✅ S3 destination configured
🔧 Configuring Rigatoni pipeline...
✅ Pipeline created successfully

📊 Metrics Information:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The pipeline is now running and watching for changes!

### Step 9: Generate Activity

In a new terminal, run the test data generator:

```bash
./tools/local-development/scripts/generate-test-data.sh
```

This script continuously inserts documents into MongoDB. You'll see the pipeline process them in real-time.

Alternatively, manually insert data:

```bash
docker exec -it rigatoni-mongodb mongosh testdb
```

```javascript
// Insert users
db.users.insertOne({
  name: "David Brown",
  email: "david@example.com",
  age: 28,
  city: "Seattle"
})

// Insert orders
db.orders.insertOne({
  userId: 4,
  product: "Doohickey",
  amount: 19.99,
  status: "pending"
})

// Update existing
db.users.updateOne(
  { name: "Alice Smith" },
  { $set: { age: 31 } }
)

// Delete
db.products.deleteOne({ name: "Widget" })
```

### Step 10: View Metrics

While the pipeline is running, check the metrics:

```bash
curl http://localhost:9000/metrics | grep rigatoni_
```

You'll see metrics like:

```prometheus
# HELP rigatoni_events_processed_total Total number of events successfully processed
# TYPE rigatoni_events_processed_total counter
rigatoni_events_processed_total{collection="users",operation="insert"} 15

# HELP rigatoni_batch_duration_seconds Time taken to process a batch
# TYPE rigatoni_batch_duration_seconds histogram
rigatoni_batch_duration_seconds_sum{collection="users"} 0.234
rigatoni_batch_duration_seconds_count{collection="users"} 3

# HELP rigatoni_pipeline_status Current pipeline status (0=stopped, 1=running, 2=error)
# TYPE rigatoni_pipeline_status gauge
rigatoni_pipeline_status 1
```

---

## Accessing Services

> **📖 Complete Service Details**
>
> For connection strings, ports, and configuration options for all services, see **[docker/README.md - Service Details](../../docker/README.md#service-details)**.

**Quick Access URLs:**
- **Grafana Dashboard**: [http://localhost:3000](http://localhost:3000) (admin/admin)
- **Prometheus**: [http://localhost:9090](http://localhost:9090)
- **MongoDB Web UI**: [http://localhost:8081](http://localhost:8081)
- **Redis Web UI**: [http://localhost:8082](http://localhost:8082)

**Common Commands:**

```bash
# MongoDB CLI
docker exec -it rigatoni-mongodb mongosh

# Redis CLI
docker exec -it rigatoni-redis redis-cli -a redispassword

# List S3 buckets
awslocal s3 ls

# View uploaded files
awslocal s3 ls s3://rigatoni-test-bucket/mongodb-cdc/ --recursive
```

**Example Prometheus Queries:**

```promql
# Events per second
rate(rigatoni_events_processed_total[5m])

# Events by collection
sum by (collection) (rate(rigatoni_events_processed_total[5m]))

# 95th percentile write latency
histogram_quantile(0.95, rate(rigatoni_destination_write_duration_seconds_bucket[5m]))
```

---

## Understanding the Pipeline Flow

Here's what happens when you insert a document:

1. **MongoDB Change Stream**: Document inserted into MongoDB
   ```javascript
   db.users.insertOne({ name: "Eve", email: "eve@example.com" })
   ```

2. **Rigatoni Listener**: Change stream event captured
   ```
   INFO rigatoni_core::change_stream: Received event: insert operation for collection users
   ```

3. **Batching**: Event added to batch queue
   - Metrics: `rigatoni_batch_queue_size` increases

4. **Batch Processing**: When batch size reached or timeout occurs
   ```
   INFO rigatoni_core::pipeline: Processing batch of 50 events for collection users
   ```
   - Metrics: `rigatoni_batch_size` recorded
   - Metrics: `rigatoni_batch_duration_seconds` recorded

5. **State Store**: Resume token saved to Redis
   ```
   INFO rigatoni_stores::redis: Saved resume token for users
   ```

6. **Destination Write**: Batch written to LocalStack S3
   ```
   INFO rigatoni_destinations::s3: Writing batch to s3://rigatoni-test-bucket/mongodb-cdc/metrics-demo/users/2025/01/21/1737451200000.json.gz
   ```
   - Metrics: `rigatoni_destination_write_duration_seconds` recorded
   - Metrics: `rigatoni_destination_write_bytes` recorded
   - Metrics: `rigatoni_events_processed_total` incremented
   - Metrics: `rigatoni_batches_written_total` incremented

7. **Prometheus Scrape**: Metrics scraped every 15 seconds

8. **Grafana Display**: Dashboard updates with latest metrics

---

## Configuration Details

> **📖 Service Configuration Reference**
>
> For detailed configuration options for all services (MongoDB, Redis, LocalStack, Prometheus, Grafana), see **[docker/README.md - Service Details](../../docker/README.md#service-details)** and **[Environment Variables](../../docker/README.md#environment-variables)**.

### Pipeline Configuration

The example uses these settings (see `rigatoni-core/examples/metrics_prometheus.rs:136`):

```rust
PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0&directConnection=true")
    .database("testdb")
    .collections(vec!["users", "orders", "products"])
    .batch_size(50)              // Batch up to 50 events
    .batch_timeout(10 seconds)   // Or flush after 10 seconds
    .max_retries(3)              // Retry failed writes 3 times
    .build()?
```

### State Store Options

**Option 1: Redis** (Full stack setup - what this guide uses)
```rust
use rigatoni_stores::redis::{RedisStore, RedisConfig};
let config = RedisConfig::builder()
    .url("redis://:redispassword@localhost:6379")
    .build()?;
let store = RedisStore::new(config).await?;
```

**Option 2: In-Memory** (Simplest setup - MongoDB only)
```rust
use rigatoni_stores::memory::MemoryStore;
let store = MemoryStore::new();
```

See [Minimal Setup](#minimal-setup-mongodb-only) section for the in-memory approach.

---

## Monitoring with Grafana

### Accessing the Dashboard

1. Open [http://localhost:3000](http://localhost:3000)
2. Login with admin/admin
3. Go to Dashboards → Rigatoni Pipeline Dashboard

### Key Panels

**Pipeline Health**:
- Pipeline status (should be 1.0 = running)
- Active collections (should be 3: users, orders, products)

**Throughput**:
- Events processed per second
- Breakdown by collection
- Breakdown by operation (insert/update/delete)

**Latency**:
- p50, p95, p99 write latencies
- Batch processing duration
- Histogram of write latencies

**Errors & Retries**:
- Failed events over time
- Retry attempts
- Error types

**Data Volume**:
- Bytes written per second
- Cumulative data written
- Average batch size

**Queue Health**:
- Current queue depth
- Queue growth rate

### Setting Up Alerts

Grafana can alert you when things go wrong. Example alert:

1. Edit a panel
2. Go to Alert tab
3. Create alert rule:

```
WHEN avg() OF query(A, 5m, now) IS ABOVE 0.05
```

This alerts when error rate exceeds 5%.

---

## Testing Different Scenarios

### Scenario 1: High Volume Inserts

```bash
./tools/local-development/scripts/generate-test-data.sh
```

Watch metrics:
- `rigatoni_events_processed_total` should increase rapidly
- `rigatoni_batch_size` should approach configured max (50)
- `rigatoni_batch_duration_seconds` shows processing time

### Scenario 2: Updates and Deletes

```bash
docker exec -it rigatoni-mongodb mongosh testdb
```

```javascript
// Bulk updates
for (let i = 0; i < 100; i++) {
  db.users.updateOne(
    { name: "Alice Smith" },
    { $inc: { age: 1 } }
  )
}

// Bulk deletes
db.orders.deleteMany({ status: "pending" })
```

Watch metrics labeled with `operation="update"` and `operation="delete"`.

### Scenario 3: Pipeline Restart (Resume Token)

1. Stop the pipeline (Ctrl+C)
2. Insert data while pipeline is down
3. Restart the pipeline

The pipeline should catch up, processing all missed events. Resume tokens in Redis ensure no data loss.

Verify:
```bash
docker exec -it rigatoni-redis redis-cli -a redispassword
> KEYS rigatoni:resume_token:*
> GET rigatoni:resume_token:testdb:users
```

### Scenario 4: Error Simulation

Temporarily break S3 connectivity:

```bash
docker stop rigatoni-localstack
```

Insert data - the pipeline will retry and fail. Watch:
- `rigatoni_retries_total` increases
- `rigatoni_events_failed_total` increases
- `rigatoni_pipeline_status` may change

Restore:
```bash
docker start rigatoni-localstack
```

The pipeline should recover and successfully write queued events.

### Scenario 5: Large Documents

```bash
docker exec -it rigatoni-mongodb mongosh testdb
```

```javascript
db.users.insertOne({
  name: "Large Doc User",
  email: "large@example.com",
  metadata: "x".repeat(100000)  // 100KB document
})
```

Watch `rigatoni_destination_write_bytes` to see the impact.

---

## Customizing the Setup

### Change MongoDB Database/Collections

Edit the example code or set environment variables:

```bash
export MONGODB_DATABASE="mydb"
export MONGODB_COLLECTIONS="collection1,collection2"
```

### Change Batch Size

Modify `rigatoni-core/examples/metrics_prometheus.rs:144`:

```rust
.batch_size(100)              // Larger batches
.batch_timeout(Duration::from_secs(5))  // Flush sooner
```

### Use Different S3 Bucket

Edit `rigatoni-core/examples/metrics_prometheus.rs:111`:

```rust
.bucket("my-custom-bucket")
```

Then create the bucket in LocalStack:

```bash
awslocal s3 mb s3://my-custom-bucket
```

### Use Parquet Instead of JSON

Edit S3 config:

```rust
.format(SerializationFormat::Parquet)
.compression(Compression::Zstd)  // Better for Parquet
```

And update dependencies in `rigatoni-destinations/Cargo.toml` to enable Parquet feature.

### Change Prometheus Scrape Interval

Edit `tools/local-development/config/prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'rigatoni'
    scrape_interval: 5s  # Scrape every 5 seconds
```

Restart Prometheus:
```bash
docker restart rigatoni-prometheus
```

---

## Minimal Setup (MongoDB Only)

Want the absolute simplest setup for quick experiments? Use the in-memory state store to skip Redis entirely.

### Step 1: Start Only MongoDB

```bash
# Start MongoDB with replica set
docker run -d --name mongodb -p 27017:27017 \
  mongo:7.0 --replSet rs0 --bind_ip_all

# Initialize replica set (wait a few seconds first)
docker exec mongodb mongosh --eval "rs.initiate()"
```

### Step 2: Run the Simple Example

```bash
cargo run -p rigatoni-core --example simple_pipeline_memory
```

This example uses:
- ✅ MongoDB (change streams)
- ✅ In-memory state store (no Redis!)
- ✅ Console destination (prints to terminal)
- ❌ No S3, Prometheus, Grafana, or Redis

### Step 3: Insert Test Data

In another terminal:

```bash
docker exec mongodb mongosh testdb --eval '
  db.users.insertOne({
    name: "Alice",
    email: "alice@example.com",
    age: 30
  })
'
```

Watch the events appear in the first terminal!

### What You Get

**Pros:**
- Fastest possible setup (just MongoDB)
- No configuration files needed
- Perfect for learning and quick tests
- See events in real-time in your terminal

**Cons:**
- No persistence (resume tokens lost on restart)
- No observability (metrics, dashboards)
- No real destination (just console output)

**Perfect for:**
- First time trying Rigatoni
- Understanding change streams
- Quick experiments
- Testing pipeline logic

### Cleanup

```bash
docker stop mongodb && docker rm mongodb
```

---

## Troubleshooting

> **📖 Complete Troubleshooting Guide**
>
> For detailed troubleshooting of all services (MongoDB, Redis, LocalStack, Prometheus, Grafana), see **[docker/README.md - Troubleshooting](../../docker/README.md#troubleshooting)**.

### Common Issues

**Pipeline Not Processing Events**

If metrics show `rigatoni_events_processed_total` is 0:

1. Insert a test document:
   ```bash
   docker exec rigatoni-mongodb mongosh testdb --eval 'db.users.insertOne({name:"test"})'
   ```

2. Check pipeline logs:
   ```bash
   RUST_LOG=debug cargo run --example metrics_prometheus --features metrics-export
   ```

**Data Not in S3**

If pipeline runs but no files appear in LocalStack:

1. Check S3:
   ```bash
   awslocal s3 ls s3://rigatoni-test-bucket/mongodb-cdc/ --recursive
   ```

2. Verify batch is being flushed (wait for timeout or insert enough documents)

**Prometheus Not Scraping**

1. Verify metrics endpoint:
   ```bash
   curl http://localhost:9000/metrics
   ```

2. Check Prometheus targets at [http://localhost:9090/targets](http://localhost:9090/targets)

For service-specific issues (MongoDB replica set, Redis connection, LocalStack health, port conflicts, etc.), see the [docker/README.md troubleshooting section](../../docker/README.md#troubleshooting).

---

## Stopping and Cleaning Up

**Stop all services (keep data):**
```bash
cd docker && docker compose down
```

**Stop and remove all data:**
```bash
cd docker && docker compose down -v
```

**View logs:**
```bash
docker logs rigatoni-mongodb -f  # Specific service
cd docker && docker compose logs -f  # All services
```

> **📖 Docker Management**
>
> For detailed Docker Compose commands, volume management, and individual service control, see **[docker/README.md - Common Commands](../../docker/README.md#common-commands)**.

---

## Next Steps

Now that you have a working local environment:

1. **Experiment with Configuration**: Try different batch sizes, formats, compression
2. **Add Custom Metrics**: Instrument your own code
3. **Build Custom Dashboards**: Create Grafana dashboards for your use case
4. **Test Failure Scenarios**: Simulate errors, restarts, network issues
5. **Load Testing**: Use the test data generator with high volume
6. **Deploy to Production**: Adapt this setup for AWS/GCP/Azure

### Additional Resources

- [Getting Started Guide](../getting-started) - Basic pipeline setup
- [Observability Guide](../OBSERVABILITY) - Comprehensive metrics guide
- [S3 Configuration](s3-configuration) - S3 destination options
- [Redis Configuration](redis-configuration) - State store setup
- [Production Deployment](production-deployment) - Production best practices
- [Example Code](https://github.com/valeriouberti/rigatoni/tree/main/rigatoni-core/examples) - More examples

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Your Local Machine                       │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  Docker Compose                       │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │  │
│  │  │ MongoDB  │  │  Redis   │  │   LocalStack S3  │   │  │
│  │  │  :27017  │  │  :6379   │  │      :4566       │   │  │
│  │  └─────┬────┘  └─────┬────┘  └────────┬─────────┘   │  │
│  │        │             │                 │             │  │
│  │  ┌─────┴─────────────┴─────────────────┴─────────┐   │  │
│  │  │                                                 │   │  │
│  │  │  ┌───────────┐  ┌────────────┐  ┌──────────┐  │   │  │
│  │  │  │Prometheus │  │  Grafana   │  │  Mongo   │  │   │  │
│  │  │  │   :9090   │  │   :3000    │  │ Express  │  │   │  │
│  │  │  └─────┬─────┘  └──────┬─────┘  │  :8081   │  │   │  │
│  │  │        │                │        └──────────┘  │   │  │
│  │  └────────┼────────────────┼─────────────────────┘   │  │
│  └───────────┼────────────────┼──────────────────────────┘  │
│              │                │                              │
│  ┌───────────┴────────────────┴──────────────────────────┐  │
│  │        Rigatoni Pipeline (Rust Application)           │  │
│  │                    :9000 (metrics)                     │  │
│  │                                                        │  │
│  │  ┌──────────────┐  ┌──────────┐  ┌────────────────┐  │  │
│  │  │  Change      │→ │ Batcher  │→ │  S3 Writer     │  │  │
│  │  │  Stream      │  │          │  │                │  │  │
│  │  │  Listener    │  │ (Redis)  │  │ (LocalStack)   │  │  │
│  │  └──────────────┘  └──────────┘  └────────────────┘  │  │
│  │         ↓                                             │  │
│  │  ┌──────────────────────────────────────────────────┐ │  │
│  │  │         Metrics Exporter (Prometheus)            │ │  │
│  │  └──────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘

Data Flow:
1. MongoDB Change Stream → Rigatoni Listener
2. Events → Batcher (with Redis state)
3. Batches → S3 Writer (LocalStack)
4. Metrics → Prometheus → Grafana Dashboards
```

---

## Summary

You now have a complete local development environment for Rigatoni with:

- Real-time change data capture from MongoDB
- Distributed state management with Redis
- Local S3 storage with LocalStack
- Comprehensive metrics with Prometheus
- Beautiful dashboards with Grafana
- Web UIs for easy data inspection

This setup gives you production-like experience locally, making it easy to develop, test, and learn Rigatoni.

Happy building!
