# Rigatoni Core Examples

This directory contains examples demonstrating the core functionality of Rigatoni, including pipeline orchestration, change stream integration, and observability.

## Quick Reference

| Example | Difficulty | Setup Time | Dependencies | Best For |
|---------|-----------|------------|--------------|----------|
| [simple_pipeline_memory.rs](#simple_pipeline_memoryrs) | ⭐ Beginner | < 5 min | MongoDB only | First-time users, quick experiments |
| [metrics_prometheus.rs](#metrics_prometheusrs) | ⭐⭐ Intermediate | 10-15 min | Full stack | Production preparation, observability |
| [change_stream_listener.rs](#change_stream_listenerrs) | ⭐⭐ Intermediate | < 5 min | MongoDB only | Understanding internals, custom integrations |

---

## simple_pipeline_memory.rs

**The simplest possible Rigatoni setup** - Perfect starting point for learning the framework.

### What It Does

This example demonstrates:
- Creating a pipeline with in-memory state store (no Redis required)
- Watching MongoDB change streams for real-time events
- Processing events with a simple console destination
- Graceful shutdown handling

### Prerequisites

**Minimal setup** - Just MongoDB in replica set mode:

```bash
# Start MongoDB
docker run -d --name mongodb -p 27017:27017 \
  mongo:7.0 --replSet rs0

# Initialize replica set (required for change streams)
docker exec mongodb mongosh --eval "rs.initiate()"
```

### Running the Example

```bash
cargo run --example simple_pipeline_memory
```

The pipeline will start and display:
```
🚀 Starting Rigatoni with In-Memory State Store
✅ In-memory state store created (no external dependencies)
✅ Console destination created
📊 Configuration:
   MongoDB: mongodb://localhost:27017/?replicaSet=rs0&directConnection=true
   Database: testdb
   Collections: ["users"]
   Batch size: 10
   Batch timeout: 5s
   State store: In-memory (ephemeral)
▶️  Starting pipeline...
✅ Pipeline running!
```

### Generating Test Data

In another terminal, insert some data to see events:

```bash
docker exec mongodb mongosh testdb --eval '
  db.users.insertOne({
    name: "Alice",
    email: "alice@example.com",
    age: 30
  })
'
```

You'll see output like:
```
📦 Event received count=1 collection=users operation=Insert
   Document { _id: ObjectId("..."), name: "Alice", email: "alice@example.com", age: 30 }
```

### Key Concepts

#### In-Memory State Store

The example uses `MemoryStore` from `rigatoni-stores`:

```rust
let store = MemoryStore::new();
```

**Advantages:**
- No external dependencies (no Redis)
- Zero configuration
- Fast local development
- Perfect for learning and testing

**Limitations:**
- Resume tokens not persisted across restarts
- Single process only (no distributed state)
- Events replay from beginning on restart

For production, use `RedisStore` instead.

#### Console Destination

A simple destination that prints events to the terminal:

```rust
#[derive(Debug)]
struct ConsoleDestination {
    event_count: usize,
}

#[async_trait::async_trait]
impl Destination for ConsoleDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        for event in events {
            // Print event details
        }
        Ok(())
    }
}
```

This demonstrates the minimal `Destination` trait implementation.

#### Pipeline Configuration

The pipeline is configured with:

```rust
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0&directConnection=true")
    .database("testdb")
    .collections(vec!["users".to_string()])
    .batch_size(10)
    .batch_timeout(Duration::from_secs(5))
    .build()?;
```

**Key parameters:**
- `mongodb_uri` - Connection string (must be replica set)
- `database` - Database to watch
- `collections` - Collections to monitor
- `batch_size` - Max events per batch
- `batch_timeout` - Max wait time before flushing batch

### Modifying the Example

#### Watch Multiple Collections

```rust
.collections(vec![
    "users".to_string(),
    "orders".to_string(),
    "products".to_string(),
])
```

#### Change Batch Behavior

```rust
.batch_size(100)                         // Larger batches
.batch_timeout(Duration::from_secs(1))   // More frequent flushes
```

#### Use Different Database

```rust
.database("production")
.collections(vec!["events".to_string()])
```

### Common Issues

#### Error: "The $changeStream stage is only supported on replica sets"

**Solution:** Initialize MongoDB as replica set:
```bash
docker exec mongodb mongosh --eval "rs.initiate()"
```

#### No events showing up

**Checklist:**
1. Is MongoDB running? `docker ps`
2. Is it a replica set? `docker exec mongodb mongosh --eval "rs.status()"`
3. Are you inserting to the correct database/collection? (default: `testdb.users`)
4. Is the pipeline running? Look for "Pipeline running!" message

#### Pipeline restarts from beginning

This is expected with `MemoryStore`. Resume tokens are not persisted. To fix:
1. Use `RedisStore` for persistent state
2. See [metrics_prometheus.rs](#metrics_prometheusrs) example

### Next Steps

After mastering this example:
1. Try [S3 Basic Example](../../rigatoni-destinations/examples/README.md#s3_basicrs) to learn about destinations
2. Try [metrics_prometheus.rs](#metrics_prometheusrs) for production-like setup with Redis and observability

---

## metrics_prometheus.rs

**Complete production-like example** with full observability stack.

### What It Does

This example demonstrates:
- Redis state store for distributed, persistent resume tokens
- S3 destination (via LocalStack) for data lake storage
- Prometheus metrics export
- Grafana dashboards for visualization
- Production-ready error handling and graceful shutdown

### Prerequisites

**Full local development stack** using Docker Compose:

```bash
# Start all services
cd docker
docker compose up -d

# Verify services are running
docker compose ps
```

Services started:
- MongoDB (replica set on port 27017)
- Redis (port 6379)
- LocalStack (S3 on port 4566)
- Prometheus (port 9090)
- Grafana (port 3000)

### Running the Example

```bash
# From repo root
cargo run --example metrics_prometheus --features metrics-export
```

Required features:
- `metrics-export` - Enables Prometheus exporter

### Generating Test Data

Use the provided script:

```bash
./docker/scripts/generate-test-data.sh
```

Or manually:

```bash
docker exec mongodb mongosh testdb --eval '
  for (let i = 0; i < 100; i++) {
    db.users.insertOne({
      name: "User " + i,
      email: "user" + i + "@example.com",
      timestamp: new Date()
    });
  }
'
```

### Accessing Services

**Prometheus** - http://localhost:9090
- Query metrics: `rigatoni_events_processed_total`
- View targets: Status > Targets

**Grafana** - http://localhost:3000
- Default credentials: admin/admin
- Import dashboard: `docs/grafana-dashboard.json`
- Pre-configured Prometheus datasource

**LocalStack S3** - AWS CLI:
```bash
awslocal s3 ls s3://rigatoni-test-bucket/
```

**MongoDB** - mongosh:
```bash
docker exec -it mongodb mongosh testdb
```

**Redis** - redis-cli:
```bash
docker exec -it redis redis-cli
KEYS rigatoni:*
```

### Key Concepts

#### Redis State Store

Persistent, distributed resume token storage:

```rust
use rigatoni_stores::redis::RedisStore;

let redis_config = RedisConfig::builder()
    .host("localhost")
    .port(6379)
    .key_prefix("rigatoni")
    .build()?;

let store = RedisStore::new(redis_config).await?;
```

**Advantages over MemoryStore:**
- Resume tokens survive process restarts
- Can be shared across multiple pipeline instances
- Durable storage with Redis persistence

#### S3 Destination

Write events to S3 (LocalStack):

```rust
use rigatoni_destinations::s3::{S3Destination, S3Config};

let s3_config = S3Config::builder()
    .bucket("rigatoni-test-bucket")
    .endpoint("http://localhost:4566")  // LocalStack
    .region("us-east-1")
    .build()?;

let destination = S3Destination::new(s3_config).await?;
```

Supports:
- Multiple serialization formats (JSON, Parquet)
- Compression (Gzip, Zstandard)
- Partitioning strategies (Hive-style, date-based)

#### Metrics Export

Prometheus metrics exposed on port 9091:

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

let builder = PrometheusBuilder::new();
builder
    .with_http_listener(([0, 0, 0, 0], 9091))
    .install()?;
```

Available metrics:
- `rigatoni_events_processed_total` - Counter of processed events
- `rigatoni_events_failed_total` - Counter of failed events
- `rigatoni_batch_size` - Histogram of batch sizes
- `rigatoni_batch_duration_seconds` - Histogram of batch processing time
- `rigatoni_destination_write_duration_seconds` - Histogram of destination write latency
- `rigatoni_active_collections` - Gauge of active collections
- `rigatoni_pipeline_status` - Gauge of pipeline health (1=running, 0=stopped)

All metrics include labels:
- `collection` - MongoDB collection name
- `destination_type` - Type of destination (s3, console, etc.)
- `error_type` - Type of error (for failure metrics)

### Grafana Dashboard

Import the pre-built dashboard:

1. Open Grafana: http://localhost:3000
2. Navigate to Dashboards > Import
3. Upload `docs/grafana-dashboard.json`
4. Select Prometheus datasource

Dashboard panels:
- Event throughput (events/sec)
- Error rate
- Batch size distribution (p50, p95, p99)
- Processing latency
- Destination write latency
- Active collections

### Modifying the Example

#### Use Real AWS S3

Replace LocalStack endpoint with real AWS:

```rust
let s3_config = S3Config::builder()
    .bucket("your-production-bucket")
    .region("us-west-2")
    // Remove .endpoint() to use real AWS
    .build()?;
```

Ensure AWS credentials are configured:
```bash
export AWS_ACCESS_KEY_ID="your-key"
export AWS_SECRET_ACCESS_KEY="your-secret"
```

#### Add Compression

```rust
use rigatoni_destinations::s3::Compression;

let s3_config = S3Config::builder()
    .bucket("rigatoni-test-bucket")
    .compression(Compression::Gzip)
    .build()?;
```

#### Enable Partitioning

```rust
use rigatoni_destinations::s3::PartitionStrategy;

let s3_config = S3Config::builder()
    .bucket("rigatoni-test-bucket")
    .partition_strategy(PartitionStrategy::DateBased {
        format: "year=%Y/month=%m/day=%d".to_string(),
    })
    .build()?;
```

### Common Issues

#### Redis connection refused

**Check Redis is running:**
```bash
docker compose ps redis
```

**Restart if needed:**
```bash
docker compose restart redis
```

#### LocalStack S3 bucket not found

**Create bucket:**
```bash
awslocal s3 mb s3://rigatoni-test-bucket
```

**Verify:**
```bash
awslocal s3 ls
```

#### Prometheus not scraping metrics

**Check metrics endpoint is accessible:**
```bash
curl http://localhost:9091/metrics
```

**Check Prometheus targets:**
- Open http://localhost:9090/targets
- Verify `rigatoni` target is UP

#### No data in Grafana

**Checklist:**
1. Is the pipeline running and processing events?
2. Is Prometheus scraping metrics? (check /targets)
3. Is Grafana datasource configured? (Settings > Data Sources)
4. Generate test data: `./docker/scripts/generate-test-data.sh`

### Performance Tuning

#### Increase Batch Size

For higher throughput:

```rust
.batch_size(1000)
.batch_timeout(Duration::from_secs(10))
```

**Trade-offs:**
- Larger batches = higher throughput
- Longer timeout = more latency
- More memory usage

#### Adjust Worker Threads

In Cargo.toml:

```toml
[profile.release]
opt-level = 3
lto = true
```

Or use `TOKIO_WORKER_THREADS`:

```bash
TOKIO_WORKER_THREADS=4 cargo run --example metrics_prometheus --release
```

### Production Checklist

Before deploying to production:

- [ ] Replace LocalStack with real AWS S3
- [ ] Configure Redis persistence (RDB or AOF)
- [ ] Set up Redis high availability (Sentinel or Cluster)
- [ ] Enable compression for S3 destination
- [ ] Configure appropriate batch size/timeout
- [ ] Set up Prometheus alerting rules
- [ ] Configure log aggregation
- [ ] Test graceful shutdown (SIGTERM handling)
- [ ] Set resource limits (memory, CPU)
- [ ] Enable TLS for Redis and MongoDB connections

### Next Steps

After mastering this example:
1. Review [S3 Advanced Example](../../rigatoni-destinations/examples/README.md#s3_advancedrs) for partitioning strategies
2. Read [Observability Guide](../../docs/OBSERVABILITY.md) for metric details
3. Read [Production Deployment Guide](../../docs/guides/production-deployment.md)

---

## change_stream_listener.rs

**Low-level change stream API** for advanced use cases and custom integrations.

### What It Does

This example demonstrates:
- Direct usage of `ChangeStreamListener` without full pipeline
- Filtering change events by operation type
- Manual resume token handling
- Watching multiple collections
- Custom event processing logic

### Prerequisites

Just MongoDB in replica set mode:

```bash
docker run -d --name mongodb -p 27017:27017 \
  mongo:7.0 --replSet rs0

docker exec mongodb mongosh --eval "rs.initiate()"
```

### Running the Example

```bash
cargo run --example change_stream_listener
```

### What You'll See

The example demonstrates several change stream patterns:

1. **Basic listener** - Watch single collection
2. **Filtered listener** - Only insert and update operations
3. **Multi-collection** - Watch multiple collections
4. **Resume token handling** - Save and resume from specific point

### Key Concepts

#### Direct ChangeStreamListener Usage

Without the full pipeline abstraction:

```rust
use rigatoni_core::stream::ChangeStreamListener;
use mongodb::Client;

let client = Client::with_uri_str(&uri).await?;
let db = client.database("testdb");

let listener = ChangeStreamListener::new(
    db.clone(),
    vec!["users".to_string()],
    None, // No resume token
).await?;
```

#### Event Filtering

Filter by operation type:

```rust
use rigatoni_core::event::OperationType;

while let Some(event) = listener.next().await {
    match event.operation {
        OperationType::Insert => {
            // Handle inserts
        }
        OperationType::Update => {
            // Handle updates
        }
        OperationType::Delete => {
            // Handle deletes
        }
        _ => continue, // Ignore other operations
    }
}
```

#### Resume Token Management

Save and restore position in change stream:

```rust
// Get resume token from event
if let Some(resume_token) = &event.resume_token {
    // Save token (to Redis, file, database, etc.)
    save_resume_token("users", resume_token).await?;
}

// Later, resume from saved token
let token = load_resume_token("users").await?;
let listener = ChangeStreamListener::new(
    db.clone(),
    vec!["users".to_string()],
    token, // Resume from this point
).await?;
```

This allows processing to continue from where it left off after restart.

#### Multi-Collection Watching

Watch multiple collections with one listener:

```rust
let listener = ChangeStreamListener::new(
    db.clone(),
    vec![
        "users".to_string(),
        "orders".to_string(),
        "products".to_string(),
    ],
    None,
).await?;

while let Some(event) = listener.next().await {
    println!("Collection: {}", event.namespace.collection);
    // Handle event based on collection
}
```

### Use Cases

#### Custom Aggregation Pipeline

Add MongoDB aggregation to change stream:

```rust
use mongodb::bson::doc;

let pipeline = vec![
    doc! {
        "$match": {
            "operationType": { "$in": ["insert", "update"] },
            "fullDocument.status": "active"
        }
    }
];

// Apply pipeline to change stream
// (Note: This requires modifying ChangeStreamListener internals)
```

#### Event Routing

Route events to different destinations based on content:

```rust
while let Some(event) = listener.next().await {
    if let Some(doc) = &event.full_document {
        match doc.get_str("type") {
            Ok("order") => send_to_orders_queue(event).await?,
            Ok("user") => send_to_users_queue(event).await?,
            _ => send_to_default_queue(event).await?,
        }
    }
}
```

#### Change Stream Analytics

Compute real-time statistics:

```rust
use std::collections::HashMap;

let mut stats: HashMap<String, u64> = HashMap::new();

while let Some(event) = listener.next().await {
    let key = format!("{}:{:?}", event.namespace.collection, event.operation);
    *stats.entry(key).or_insert(0) += 1;

    if stats.values().sum::<u64>() % 1000 == 0 {
        println!("Stats: {:?}", stats);
    }
}
```

### Modifying the Example

#### Add Full Document Lookup

For update events, fetch the full document:

```rust
use mongodb::options::ChangeStreamOptions;

let options = ChangeStreamOptions::builder()
    .full_document(Some(mongodb::options::FullDocumentType::UpdateLookup))
    .build();
```

#### Filter by Namespace

Only watch specific collections:

```rust
let pipeline = vec![
    doc! {
        "$match": {
            "ns.coll": { "$in": ["users", "premium_users"] }
        }
    }
];
```

### Common Issues

#### Error: "cannot open change stream"

**Possible causes:**
1. MongoDB not in replica set mode
2. Insufficient permissions
3. MongoDB version too old (need 3.6+)

**Solution:**
```bash
docker exec mongodb mongosh --eval "rs.status()"
```

#### Missing full document on updates

Update events don't include full document by default. Enable it:

```rust
.full_document(Some(FullDocumentType::UpdateLookup))
```

#### Events missed during downtime

Without resume token, events during downtime are lost. Solution:
1. Always save resume tokens
2. Use oplog retention to allow catching up
3. Consider using the full Pipeline for built-in token management

### When to Use This Example

Use `ChangeStreamListener` directly when:
- Need custom event filtering beyond what Pipeline provides
- Building custom integration (not using standard destinations)
- Need fine-grained control over change stream options
- Implementing custom aggregation pipelines
- Building real-time analytics on change events

Use full `Pipeline` when:
- Standard ETL/CDC use case
- Want built-in batching and retry logic
- Need metrics and observability
- Using standard destinations (S3, BigQuery, Kafka)

### Next Steps

After understanding the low-level API:
1. Review [Pipeline source code](../src/pipeline.rs) to see how it uses `ChangeStreamListener`
2. Read [MongoDB Change Streams documentation](https://www.mongodb.com/docs/manual/changeStreams/)
3. Explore [Architecture Guide](../../docs/architecture.md) for system design

---

## Additional Resources

- [Main Examples README](../../examples/README.md) - Complete examples guide
- [Destination Examples](../../rigatoni-destinations/examples/README.md) - S3 destination examples
- [Getting Started Guide](../../docs/getting-started.md) - Step-by-step tutorial
- [Local Development Guide](../../docs/guides/local-development.md) - Complete local setup
- [API Documentation](https://docs.rs/rigatoni-core) - API reference

---

## Contributing Examples

When adding new core examples:

1. **Follow naming convention** - `{feature}_{variation}.rs`
2. **Include documentation** - Comprehensive doc comments at top
3. **Keep minimal** - Only dependencies absolutely needed
4. **Add README entry** - Document here with full details
5. **Test from clean state** - Verify setup instructions work
6. **Show graceful shutdown** - Always handle SIGTERM/Ctrl+C

Example template:

```rust
//! Brief description
//!
//! # Prerequisites
//!
//! ```bash
//! # Setup commands
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo run --example my_example
//! ```

use rigatoni_core::pipeline::Pipeline;
// ... rest of implementation
```

---

## Need Help?

- **Issues:** [GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)
- **Discussions:** [GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)
- **Guides:** [Documentation](https://valeriouberti.github.io/rigatoni/)
