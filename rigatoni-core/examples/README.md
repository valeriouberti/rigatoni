# Rigatoni Core Examples

Detailed documentation for core Rigatoni examples demonstrating pipeline orchestration, change streams, and observability.

> **💡 New to Rigatoni examples?** Start with the [Examples Overview](../../examples/README.md) for quick start guides and common setup instructions.

## Examples in This Directory

| Example | Difficulty | Dependencies | Description |
|---------|-----------|--------------|-------------|
| [simple_pipeline_memory.rs](#simple_pipeline_memoryrs) | ⭐ Beginner | MongoDB | In-memory state, console output, minimal setup |
| [metrics_prometheus.rs](#metrics_prometheusrs) | ⭐⭐ Intermediate | Full stack | Redis, S3, Prometheus, Grafana, observability |
| [change_stream_listener.rs](#change_stream_listenerrs) | ⭐⭐ Intermediate | MongoDB | Low-level change stream API, custom integrations |

**Prerequisites:** See [Common Setup](../../examples/README.md#common-setup) for MongoDB, Redis, and LocalStack setup instructions.

---

## simple_pipeline_memory.rs

**The simplest possible Rigatoni setup** - Perfect starting point for learning the framework.

### What It Demonstrates

- Creating a pipeline with in-memory state store (no Redis required)
- Watching MongoDB change streams for real-time events  
- Processing events with a simple console destination
- Graceful shutdown handling

### Running

```bash
# See Examples Overview for MongoDB setup
cargo run --example simple_pipeline_memory
```

### Key Concepts

#### In-Memory State Store

Uses `MemoryStore` from `rigatoni-stores` - no external dependencies:

```rust
let store = MemoryStore::new();
```

**Advantages:**
- Zero configuration
- Fast local development
- No Redis needed

**Limitations:**
- Resume tokens not persisted (lost on restart)
- Single process only
- For production, use `RedisStore`

#### Console Destination

Minimal `Destination` trait implementation that prints events:

```rust
#[async_trait::async_trait]
impl Destination for ConsoleDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        for event in events {
            println!("Event: {:?}", event);
        }
        Ok(())
    }
}
```

#### Pipeline Configuration

```rust
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0")
    .database("testdb")
    .collections(vec!["users".to_string()])
    .batch_size(10)
    .batch_timeout(Duration::from_secs(5))
    .build()?;
```

### Customizing

Watch multiple collections:
```rust
.collections(vec!["users".to_string(), "orders".to_string()])
```

Adjust batching:
```rust
.batch_size(100)
.batch_timeout(Duration::from_secs(1))
```

### Troubleshooting

See [Common Issues](../../examples/README.md#common-issues) in the Examples Overview.

**Next:** Try [s3_basic.rs](../../rigatoni-destinations/examples/README.md#s3_basicrs) to learn about destinations.

---

## metrics_prometheus.rs

**Production-ready observability** with full monitoring stack.

### What It Demonstrates

- Redis state store for distributed, persistent resume tokens
- S3 destination (via LocalStack) for data lake storage
- Prometheus metrics export
- Grafana dashboards for visualization
- Complete observability setup

### Running

```bash
# Start all services (see Examples Overview for details)
cd docker && docker compose up -d

# Run example
cargo run --example metrics_prometheus --features metrics-export

# Generate test data
./docker/scripts/generate-test-data.sh
```

**Required feature:** `metrics-export`

### Accessing Services

- **Prometheus:** http://localhost:9090
- **Grafana:** http://localhost:3000 (admin/admin)
- **Metrics endpoint:** http://localhost:9000/metrics

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

Benefits over MemoryStore:
- Survives process restarts
- Shared across multiple instances
- Durable with Redis persistence

#### S3 Destination

Write events to S3 (LocalStack for local testing):

```rust
use rigatoni_destinations::s3::{S3Destination, S3Config};

let s3_config = S3Config::builder()
    .bucket("rigatoni-test-bucket")
    .endpoint("http://localhost:4566")
    .region("us-east-1")
    .build()?;

let destination = S3Destination::new(s3_config).await?;
```

#### Metrics Export

Prometheus exporter on port 9000:

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

PrometheusBuilder::new()
    .with_http_listener(([0, 0, 0, 0], 9000))
    .install()?;
```

**Available metrics:**
- `rigatoni_events_processed_total` - Events successfully processed
- `rigatoni_events_failed_total` - Failed events
- `rigatoni_batch_size` - Batch size distribution
- `rigatoni_batch_duration_seconds` - Batch processing time
- `rigatoni_destination_write_duration_seconds` - S3 write latency
- `rigatoni_active_collections` - Active collections being monitored
- `rigatoni_pipeline_status` - Pipeline health (1=running, 0=stopped)

All with labels: `collection`, `destination_type`, `error_type`

### Grafana Dashboard

Import the pre-built dashboard:

1. Open http://localhost:3000
2. Dashboards > Import
3. Upload `docs/grafana-dashboard.json`

**Panels include:**
- Event throughput (events/sec)
- Error rates
- Batch size percentiles (p50, p95, p99)
- Processing latency
- Destination write latency

### Production Preparation

**Checklist before deploying:**

- [ ] Replace LocalStack with real AWS S3
- [ ] Configure Redis persistence (RDB or AOF)
- [ ] Set up Redis HA (Sentinel or Cluster)
- [ ] Enable S3 compression
- [ ] Configure appropriate batch size
- [ ] Set up Prometheus alerting
- [ ] Enable TLS for Redis/MongoDB
- [ ] Set resource limits

**Next:** Review [Production Deployment Guide](../../docs/guides/production-deployment.md).

---

## change_stream_listener.rs

**Low-level change stream API** for advanced use cases and custom integrations.

### What It Demonstrates

- Direct usage of `ChangeStreamListener` without full pipeline
- Filtering change events by operation type
- Manual resume token handling
- Watching multiple collections
- Custom event processing logic

### Running

```bash
cargo run --example change_stream_listener
```

### Key Concepts

#### Direct ChangeStreamListener Usage

Without pipeline abstraction:

```rust
use rigatoni_core::stream::ChangeStreamListener;

let listener = ChangeStreamListener::new(
    db.clone(),
    vec!["users".to_string()],
    None, // No resume token
).await?;

while let Some(event) = listener.next().await {
    // Process event directly
}
```

#### Event Filtering

Filter by operation type:

```rust
match event.operation {
    OperationType::Insert => { /* handle inserts */ }
    OperationType::Update => { /* handle updates */ }
    OperationType::Delete => { /* handle deletes */ }
    _ => continue,
}
```

#### Resume Token Management

Manual checkpoint control:

```rust
// Save resume token
if let Some(token) = &event.resume_token {
    save_to_storage("users", token).await?;
}

// Later, resume from token
let token = load_from_storage("users").await?;
let listener = ChangeStreamListener::new(db, vec!["users".to_string()], token).await?;
```

### Use Cases

**When to use ChangeStreamListener directly:**
- Custom event routing logic
- Real-time analytics
- Integration with non-standard destinations
- Fine-grained control over change stream options

**When to use Pipeline:**
- Standard CDC/data replication workflows
- Need batching and retry logic
- Using standard destinations (S3, BigQuery)
- Want built-in metrics

### Customizing

Multi-collection watching:

```rust
ChangeStreamListener::new(
    db,
    vec!["users".to_string(), "orders".to_string(), "products".to_string()],
    None
).await?
```

Custom event routing:

```rust
while let Some(event) = listener.next().await {
    match event.namespace.collection.as_str() {
        "users" => send_to_users_queue(event).await?,
        "orders" => send_to_orders_queue(event).await?,
        _ => log_event(event).await?,
    }
}
```

**Next:** Review [Pipeline source code](../src/pipeline.rs) to see how it uses `ChangeStreamListener`.

---

## Additional Resources

- **[Examples Overview](../../examples/README.md)** - Quick start and common setup
- **[Destination Examples](../../rigatoni-destinations/examples/README.md)** - S3 examples
- **[Local Development Guide](../../docs/guides/local-development.md)** - Complete local setup
- **[Observability Guide](../../docs/OBSERVABILITY.md)** - Metrics deep-dive
- **[API Documentation](https://docs.rs/rigatoni-core)** - API reference

---

## Need Help?

- **[GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)** - Ask questions
- **[GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)** - Report problems
