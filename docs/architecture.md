---
layout: default
title: Architecture
nav_order: 3
description: "Learn about Rigatoni's system design, core concepts, and technical architecture."
permalink: /architecture
---

# Architecture
{: .no_toc }

Understand Rigatoni's system design and core concepts.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Rigatoni is built around a simple but powerful architecture that separates concerns into three main components:

```
┌────────────────┐      ┌─────────────────┐      ┌──────────────────┐
│  Source        │      │   Pipeline      │      │   Destination    │
│  (MongoDB)     │─────▶│   Orchestrator  │─────▶│   (S3, Kafka,    │
│                │      │                 │      │    BigQuery)     │
└────────────────┘      └─────────────────┘      └──────────────────┘
                              │
                              ▼
                        ┌─────────────┐
                        │ State Store │
                        │ (Resume     │
                        │  Tokens)    │
                        └─────────────┘
```

## Core Components

### 1. Sources

Sources extract data from external systems. Currently, Rigatoni supports:

- **MongoDB Change Streams** - Real-time CDC (Change Data Capture)

#### MongoDB Change Stream

The MongoDB source listens to change streams and emits `ChangeEvent` objects:

```rust
pub struct ChangeEvent {
    pub resume_token: Document,      // For exactly-once/at-least-once semantics
    pub operation: OperationType,     // Insert, Update, Delete, Replace
    pub namespace: Namespace,         // Database and collection
    pub full_document: Option<Document>,  // Complete document
    pub document_key: Option<Document>,   // Document _id
    pub update_description: Option<UpdateDescription>,  // For updates
    pub cluster_time: DateTime<Utc>,  // Event timestamp
}
```

**Resume Token Support:**
Change streams provide resume tokens that allow the pipeline to restart from where it left off, ensuring no data loss.

### 2. Pipeline Orchestrator

The pipeline coordinates data flow between sources and destinations with:

- **Multi-worker architecture** - One worker per collection for parallelism
- **Batching** - Accumulate events for efficient writes
- **Retry logic** - Exponential backoff for transient failures
- **State management** - Save resume tokens for recovery

#### Worker Architecture

```rust
pub struct Pipeline<D: Destination> {
    config: PipelineConfig,
    destination: Arc<Mutex<D>>,       // Shared, mutex-protected
    shutdown_tx: broadcast::Sender<()>,  // Broadcast shutdown signal
    workers: Vec<JoinHandle<Result<(), PipelineError>>>,
    stats: Arc<RwLock<PipelineStats>>,
}
```

**Why Multiple Workers?**

- ✅ **Parallel processing** - Handle multiple collections concurrently
- ✅ **Isolation** - Collection failures don't affect others
- ✅ **Independent resume tokens** - Each collection has its own checkpoint
- ✅ **Better CPU utilization** - Leverage multi-core systems

Each worker runs independently:

```
Worker 1 (users collection)         Worker 2 (orders collection)
        │                                    │
        ▼                                    ▼
  MongoDB Stream                       MongoDB Stream
        │                                    │
        ▼                                    ▼
    Batching                             Batching
        │                                    │
        └────────────┬──────────────────────┘
                     ▼
                Destination
                  (mutex)
```

### 3. Destinations

Destinations write data to external systems. Currently supported:

- **AWS S3** - Object storage with multiple formats and compression
- **BigQuery** (coming soon) - Google Cloud data warehouse
- **Kafka** (coming soon) - Event streaming platform

#### Destination Trait

All destinations implement the `Destination` trait:

```rust
#[async_trait]
pub trait Destination: Send + Sync {
    /// Write a batch of events
    async fn write_batch(&mut self, events: &[ChangeEvent])
        -> Result<(), DestinationError>;

    /// Flush any buffered data
    async fn flush(&mut self) -> Result<(), DestinationError>;

    /// Close and cleanup
    async fn close(&mut self) -> Result<(), DestinationError>;

    /// Get destination metadata
    fn metadata(&self) -> DestinationMetadata;

    /// Get count of buffered events
    fn buffered_count(&self) -> usize;
}
```

**S3 Destination Features:**

- Multiple serialization formats (JSON, CSV, Parquet, Avro)
- Compression (Gzip, Zstandard)
- Flexible key generation strategies
- Automatic retry with exponential backoff

### 4. State Store

State stores persist resume tokens for fault tolerance:

```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save a resume token
    async fn save_resume_token(&self, collection: &str, token: Document)
        -> Result<(), StateError>;

    /// Load a resume token
    async fn load_resume_token(&self, collection: &str)
        -> Result<Option<Document>, StateError>;

    /// Clear a resume token
    async fn clear_resume_token(&self, collection: &str)
        -> Result<(), StateError>;
}
```

**Available Implementations:**

- **In-Memory** - Fast, no persistence (testing/development)
- **File-based** - JSON files on disk
- **Redis** - Distributed state for multi-instance deployments with distributed locking

---

## Data Flow

### End-to-End Flow

1. **Listen** - MongoDB change stream emits events
2. **Batch** - Accumulate events in memory
3. **Write** - Send batch to destination
4. **Checkpoint** - Save resume token to state store
5. **Repeat** - Continue from step 1

### Batching Strategy

Events are batched based on **size** OR **timeout** (whichever comes first):

```rust
loop {
    tokio::select! {
        // Timeout: flush partial batch
        _ = batch_timer.tick() => {
            if !batch.is_empty() {
                flush_batch(&batch).await?;
                batch.clear();
            }
        }

        // Event: add to batch
        Some(Ok(event)) = stream.next() => {
            batch.push(event);

            // Size: flush full batch
            if batch.len() >= batch_size {
                flush_batch(&batch).await?;
                batch.clear();
            }
        }
    }
}
```

**Trade-offs:**

| Batch Size | Throughput | Latency | Memory |
|------------|------------|---------|--------|
| Small (10) | Lower      | Lower   | Lower  |
| Medium (100) | Good     | Medium  | Medium |
| Large (5000) | Highest  | Higher  | Higher |

**Timeout Benefits:**

- Guarantees maximum latency for low-volume streams
- Ensures resume tokens are saved regularly
- Prevents unbounded memory growth

---

## Retry Logic

The pipeline implements exponential backoff with jitter:

```rust
pub async fn write_with_retry(
    destination: &mut D,
    events: &[ChangeEvent],
    config: &RetryConfig,
) -> Result<(), PipelineError> {
    let mut attempt = 0;
    let mut delay = config.retry_delay;

    loop {
        match destination.write_batch(events).await {
            Ok(_) => return Ok(()),
            Err(e) if !is_retryable(&e) => return Err(e),
            Err(e) => {
                attempt += 1;
                if attempt > config.max_retries {
                    return Err(e);
                }

                // Exponential backoff with jitter
                let jitter = rand::thread_rng().gen_range(0..100);
                tokio::time::sleep(delay + Duration::from_millis(jitter)).await;

                delay = std::cmp::min(delay * 2, config.max_retry_delay);
            }
        }
    }
}
```

**Retry Schedule Example:**

```
Attempt 1: Immediate
Attempt 2: 1s + jitter
Attempt 3: 2s + jitter  (exponential)
Attempt 4: 4s + jitter
Attempt 5: 8s + jitter
Attempt 6: 16s + jitter
Attempt 7: 30s + jitter (capped at max_retry_delay)
```

**Retryable vs Non-Retryable Errors:**

- ✅ **Retryable**: Network errors, throttling, temporary unavailability
- ❌ **Non-Retryable**: Authentication failures, invalid data, programming errors

---

## Graceful Shutdown

The pipeline supports graceful shutdown with proper cleanup:

```rust
pub async fn shutdown(&mut self) -> Result<(), PipelineError> {
    // 1. Set running flag to false
    *self.running.write().await = false;

    // 2. Broadcast shutdown to all workers
    if let Some(shutdown_tx) = self.shutdown_tx.take() {
        let _ = shutdown_tx.send(());
    }

    // 3. Wait for workers to finish (with timeout)
    let mut workers = self.workers.write().await;
    for worker in workers.drain(..) {
        tokio::time::timeout(
            Duration::from_secs(30),
            worker
        ).await??;
    }

    // 4. Flush destination
    let mut destination = self.destination.lock().await;
    destination.flush().await?;
    destination.close().await?;

    Ok(())
}
```

**Shutdown Sequence:**

1. **Signal workers** - Broadcast shutdown via channel
2. **Wait for completion** - Workers finish current batches
3. **Flush destination** - Write remaining buffered data
4. **Close connections** - Clean up resources

**Timeout Protection:**

If workers don't finish within 30 seconds, the shutdown force-completes to prevent hanging.

---

## Concurrency Model

### Async/Await with Tokio

Rigatoni uses Tokio's async runtime for non-blocking I/O:

```rust
#[tokio::main]
async fn main() {
    let pipeline = Pipeline::new(config, destination).await;
    pipeline.run().await;  // Async execution
}
```

**Benefits:**

- ✅ Efficient I/O multiplexing
- ✅ Low memory overhead (vs threads)
- ✅ Composable async operations
- ✅ Cancellation-safe with `select!`

### Synchronization Primitives

**Mutex** - Exclusive access to destination:

```rust
Arc<Mutex<D>>  // Multiple workers, one at a time can write
```

**RwLock** - Shared read, exclusive write:

```rust
Arc<RwLock<PipelineStats>>  // Many readers, one writer
```

**Broadcast Channel** - One-to-many signaling:

```rust
broadcast::Sender<()>  // Shutdown signal to all workers
```

---

## Performance Characteristics

### Throughput

- **Single worker**: ~5,000-10,000 events/second
- **Multi-worker** (4 collections): ~20,000-40,000 events/second
- **Bottleneck**: Usually destination write speed

### Latency

- **Best case**: 100-500ms (immediate batch)
- **Typical**: 1-5 seconds (batch timeout)
- **Worst case**: batch_timeout + retry_delay

### Memory Usage

```
Base overhead: ~10 MB
Per worker: ~5 MB
Per batch (1000 events × 500 bytes): ~0.5 MB

Example (4 workers, 1000 batch size):
10 MB + (4 × 5 MB) + (4 × 0.5 MB) = 32 MB
```

### Scaling Characteristics

**Horizontal Scaling with Distributed Locking:**

Run multiple pipeline instances watching the same collections with Redis-based distributed locking:

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig, DistributedLockConfig};
use rigatoni_stores::redis::{RedisStore, RedisConfig};

let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0")
    .database("mydb")
    .watch_collections(vec!["users".to_string(), "orders".to_string()])
    .distributed_lock(DistributedLockConfig {
        enabled: true,
        ttl: Duration::from_secs(30),           // Lock expires if holder crashes
        refresh_interval: Duration::from_secs(10), // Heartbeat interval
        retry_interval: Duration::from_secs(5),    // Retry claiming locks
    })
    .build()?;
```

**How it works:**
- Each collection is protected by a distributed lock (stored in Redis)
- Only one instance processes a collection at a time
- If an instance crashes, its locks expire after TTL (default 30s)
- Other instances automatically take over orphaned collections
- No duplicate event processing

```
Instance 1          Instance 2          Instance 3
    |                   |                   |
    v                   v                   v
Acquires locks     Acquires locks     Acquires locks
"users"            "orders"           "products"
    |                   |                   |
    v                   v                   v
Process events     Process events     Process events
(no duplicates!)   (no duplicates!)   (no duplicates!)
```

See [Multi-Instance Deployment Guide](guides/multi-instance-deployment) for Kubernetes examples and configuration.

**Vertical Scaling:**

- Increase `num_workers` for more parallelism
- Increase `batch_size` for higher throughput
- Add more CPU cores for compute-bound workloads

---

## Error Handling

### Error Hierarchy

```rust
pub enum PipelineError {
    MongoDB(MongoError),          // MongoDB connection/stream errors
    Destination(DestinationError), // Write failures
    Configuration(ConfigError),     // Invalid config
    StateStore(StateError),        // Resume token failures
    Timeout(String),               // Operation timeouts
    Shutdown(String),              // Shutdown errors
}
```

### Error Recovery Strategies

**1. Transient Errors** - Retry with backoff
```rust
Network timeout → Retry up to max_retries
Throttling (429) → Backoff and retry
```

**2. Permanent Errors** - Fail fast
```rust
Authentication failure → Shutdown immediately
Invalid data format → Log and skip
```

**3. State Errors** - Attempt recovery
```rust
Can't save resume token → Continue but warn
Can't load resume token → Start from beginning
```

---

## Design Principles

### 1. Type Safety

Leverage Rust's type system for correctness:

```rust
// ❌ Runtime error
let events: Vec<Box<dyn Any>> = ...;

// ✅ Compile-time safety
let events: Vec<ChangeEvent> = ...;
```

### 2. Composability

Build complex pipelines from simple components:

```rust
let pipeline = Pipeline::new(config, destination);
// Pipeline is generic over Destination trait
```

### 3. Testability

Mock destinations for testing:

```rust
#[derive(Default)]
struct MockDestination {
    events: Vec<ChangeEvent>,
}

#[async_trait]
impl Destination for MockDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        self.events.extend_from_slice(events);
        Ok(())
    }
    // ...
}
```

### 4. Observability

Comprehensive tracing and metrics:

```rust
#[instrument(skip(self), fields(collection = %collection))]
async fn process_batch(&self, collection: String, events: Vec<ChangeEvent>) {
    info!("Processing batch of {} events", events.len());
    // ...
}
```

---

## Future Enhancements

### Planned Features

- **BigQuery Destination** - Google Cloud data warehouse integration
- **Kafka Destination** - Event streaming platform integration
- **Dead Letter Queue** - Failed events for manual review
- **Schema Evolution** - Automatic schema migration
- **Exactly-Once Semantics** - Transactional destinations
- **Filtering** - Event filtering before batching
- **Transformations** - Custom data transformations

### Performance Improvements

- **Parallel S3 Uploads** - Upload multiple collections concurrently
- **Compression Optimization** - Streaming compression
- **Buffer Pooling** - Reuse allocations between batches

---

## Next Steps

- **[Getting Started](getting-started)** - Build your first pipeline
- **[User Guides](guides/)** - Task-specific guides
- **[API Reference](https://docs.rs/rigatoni)** - Complete API documentation

---

## Further Reading

- [Tokio Documentation](https://tokio.rs/tokio/tutorial)
- [MongoDB Change Streams](https://www.mongodb.com/docs/manual/changeStreams/)
- [AWS S3 Best Practices](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html)
