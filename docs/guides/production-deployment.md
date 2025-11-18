---
layout: default
title: Production Deployment
parent: Guides
nav_order: 7
description: "Best practices for deploying Rigatoni to production."
---

# Production Deployment Guide
{: .no_toc }

Best practices for deploying Rigatoni pipelines to production environments.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Pre-Deployment Checklist

Before deploying to production, ensure you have:

- ✅ **MongoDB Replica Set** - Change streams require replica sets
- ✅ **AWS Credentials** - IAM role or access keys configured
- ✅ **S3 Bucket** - Created with appropriate lifecycle policies
- ✅ **Monitoring** - Logging and metrics collection set up
- ✅ **Testing** - Integration tests passing
- ✅ **Graceful Shutdown** - Signal handling implemented

---

## Configuration Best Practices

### 1. Environment-Based Configuration

Use environment variables for deployment-specific settings:

```rust
use std::env;

let config = PipelineConfig::builder()
    .mongodb_uri(env::var("MONGODB_URI")?)
    .database(env::var("MONGODB_DATABASE")?)
    .collections(
        env::var("MONGODB_COLLECTIONS")?
            .split(',')
            .map(|s| s.to_string())
            .collect()
    )
    .batch_size(
        env::var("BATCH_SIZE")?
            .parse()
            .unwrap_or(1000)
    )
    .build()?;

let s3_config = S3Config::builder()
    .bucket(env::var("S3_BUCKET")?)
    .region(env::var("AWS_REGION")?)
    .prefix(env::var("S3_PREFIX")?)
    .build()?;
```

**Environment File:**

```bash
# .env.production
MONGODB_URI=mongodb://mongo1,mongo2,mongo3/?replicaSet=rs0
MONGODB_DATABASE=production
MONGODB_COLLECTIONS=users,orders,products
BATCH_SIZE=5000
S3_BUCKET=prod-data-lake
AWS_REGION=us-east-1
S3_PREFIX=mongodb-cdc/production
```

### 2. Optimize for Throughput

```rust
PipelineConfig::builder()
    .batch_size(5000)               // Larger batches
    .batch_timeout_ms(30000)        // 30 second timeout
    .num_workers(4)                 // More workers per collection
    .channel_buffer_size(10000)     // Larger channel buffer
    .build()?
```

### 3. Robust Retry Configuration

```rust
PipelineConfig::builder()
    .max_retries(10)                // More retries
    .retry_delay_ms(1000)           // 1 second initial delay
    .max_retry_delay_ms(60000)      // 1 minute max delay
    .build()?
```

### 4. Production Logging

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .json()  // JSON logging for structured logs
        .init();
}
```

---

## Deployment Architectures

### Single-Instance Deployment

```
┌─────────────────────────────────┐
│      Docker Container/EC2        │
│                                  │
│  ┌──────────────────────────┐   │
│  │   Rigatoni Pipeline      │   │
│  │                          │   │
│  │  MongoDB ──▶ S3          │   │
│  │                          │   │
│  │  Collections:            │   │
│  │  - users                 │   │
│  │  - orders                │   │
│  │  - products              │   │
│  └──────────────────────────┘   │
└─────────────────────────────────┘
```

**Pros:**
- Simple to deploy and manage
- Lower operational overhead

**Cons:**
- Single point of failure
- Limited horizontal scaling

**Best for:**
- Low to medium volume (< 10,000 events/sec)
- Development and staging environments

### Multi-Instance Deployment

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Instance 1  │    │  Instance 2  │    │  Instance 3  │
│              │    │              │    │              │
│  Collections:│    │  Collections:│    │  Collections:│
│  - users     │    │  - orders    │    │  - products  │
│  - comments  │    │  - payments  │    │  - inventory │
└──────────────┘    └──────────────┘    └──────────────┘
        │                   │                   │
        └───────────────────┴───────────────────┘
                            │
                      ┌─────▼─────┐
                      │    S3     │
                      └───────────┘
```

**Pros:**
- Horizontal scaling
- Fault isolation per instance
- Can dedicate resources per collection

**Cons:**
- More complex coordination
- Higher operational overhead

**Best for:**
- High volume (> 10,000 events/sec)
- Critical production workloads
- Need for high availability

---

## Docker Deployment

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.85 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/my-pipeline /usr/local/bin/pipeline

# Create non-root user
RUN useradd -m -u 1000 pipeline
USER pipeline

ENTRYPOINT ["pipeline"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  pipeline:
    build: .
    restart: unless-stopped
    environment:
      - MONGODB_URI=${MONGODB_URI}
      - MONGODB_DATABASE=${MONGODB_DATABASE}
      - MONGODB_COLLECTIONS=${MONGODB_COLLECTIONS}
      - S3_BUCKET=${S3_BUCKET}
      - AWS_REGION=${AWS_REGION}
      - S3_PREFIX=${S3_PREFIX}
      - RUST_LOG=info
    env_file:
      - .env.production
    # Mount AWS credentials (or use IAM role)
    volumes:
      - ~/.aws:/home/pipeline/.aws:ro
```

### Build and Run

```bash
# Build image
docker build -t my-pipeline:latest .

# Run container
docker-compose up -d

# View logs
docker-compose logs -f pipeline

# Stop
docker-compose down
```

---

## Kubernetes Deployment

### deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rigatoni-pipeline
  labels:
    app: rigatoni
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rigatoni
  template:
    metadata:
      labels:
        app: rigatoni
    spec:
      serviceAccountName: rigatoni-sa
      containers:
      - name: pipeline
        image: my-pipeline:latest
        env:
        - name: MONGODB_URI
          valueFrom:
            secretKeyRef:
              name: mongodb-secret
              key: uri
        - name: MONGODB_DATABASE
          value: "production"
        - name: MONGODB_COLLECTIONS
          value: "users,orders"
        - name: S3_BUCKET
          value: "prod-data-lake"
        - name: AWS_REGION
          value: "us-east-1"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
```

### Service Account (for IAM Roles)

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rigatoni-sa
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/rigatoni-s3-role
```

---

## Monitoring and Observability

### 1. Structured Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self), fields(collection = %collection))]
async fn process_batch(&self, collection: String, events: Vec<ChangeEvent>) {
    info!(
        event_count = events.len(),
        "Processing batch"
    );

    match self.write_to_destination(&events).await {
        Ok(_) => {
            info!(
                event_count = events.len(),
                "Batch written successfully"
            );
        }
        Err(e) => {
            error!(
                event_count = events.len(),
                error = %e,
                "Failed to write batch"
            );
        }
    }
}
```

### 2. Metrics Collection

```rust
use metrics::{counter, gauge, histogram};

// Track events processed
counter!("events_processed_total", "collection" => collection).increment(events.len() as u64);

// Track batch size
histogram!("batch_size", "collection" => collection).record(events.len() as f64);

// Track active workers
gauge!("active_workers").set(worker_count as f64);

// Track errors
counter!("write_errors_total", "collection" => collection).increment(1);
```

### 3. Health Checks

```rust
use axum::{Router, routing::get, Json};
use serde_json::json;

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// In main()
let app = Router::new()
    .route("/health", get(health_check));

tokio::spawn(async {
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
});
```

---

## Graceful Shutdown

### Signal Handling

```rust
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut pipeline = Pipeline::new(config, destination).await?;

    // Handle graceful shutdown
    tokio::select! {
        result = pipeline.run() => {
            if let Err(e) = result {
                error!("Pipeline error: {}", e);
                return Err(e.into());
            }
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal, shutting down gracefully...");
            pipeline.shutdown().await?;
            info!("Pipeline shut down successfully");
        }
    }

    Ok(())
}
```

### Docker Shutdown

```dockerfile
# Use SIGTERM for graceful shutdown
STOPSIGNAL SIGTERM

# Allow time for graceful shutdown (30 seconds)
# docker stop will wait this long before SIGKILL
```

### Kubernetes Shutdown

```yaml
spec:
  containers:
  - name: pipeline
    # ...
    lifecycle:
      preStop:
        exec:
          command: ["/bin/sh", "-c", "sleep 15"]
  terminationGracePeriodSeconds: 30
```

---

## Security Best Practices

### 1. Use IAM Roles

**AWS ECS/EKS:**

```yaml
# IAM role for task
taskRoleArn: arn:aws:iam::123456789012:role/rigatoni-task-role
```

**IAM Policy:**

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
      "Resource": "arn:aws:s3:::prod-data-lake/*"
    }
  ]
}
```

### 2. Encrypt Secrets

Use AWS Secrets Manager or Kubernetes Secrets:

```rust
use aws_sdk_secretsmanager::Client;

async fn get_mongodb_uri() -> Result<String, Box<dyn Error>> {
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let secret = client
        .get_secret_value()
        .secret_id("mongodb-uri")
        .send()
        .await?;

    Ok(secret.secret_string().unwrap().to_string())
}
```

### 3. Network Security

- Use VPC endpoints for S3 (no internet gateway required)
- Restrict MongoDB access to VPC
- Use TLS for MongoDB connections

```rust
.mongodb_uri("mongodb://mongo1,mongo2,mongo3/?tls=true&replicaSet=rs0")
```

---

## Performance Tuning

### 1. Resource Allocation

**CPU:**
- Minimum: 1 vCPU
- Recommended: 2-4 vCPUs for multi-worker pipelines

**Memory:**
- Minimum: 512 MB
- Recommended: 1-2 GB
- Formula: `base (256 MB) + (workers × 128 MB) + (batch_size × 1 KB)`

**Example:**
```
4 workers × 128 MB = 512 MB
batch_size=5000 × 1 KB = 5 MB
Total: 256 MB + 512 MB + 5 MB = ~1 GB
```

### 2. Batching Optimization

```rust
// High throughput, higher latency
.batch_size(10000)
.batch_timeout_ms(60000)

// Low latency, lower throughput
.batch_size(100)
.batch_timeout_ms(1000)

// Balanced (recommended)
.batch_size(5000)
.batch_timeout_ms(30000)
```

### 3. S3 Upload Optimization

```rust
use rigatoni_destinations::s3::{Compression, SerializationFormat};

S3Config::builder()
    // Use Parquet for best compression
    .format(SerializationFormat::Parquet)

    // Use Zstd for best compression ratio and speed
    .compression(Compression::Zstd)

    // Use Hive partitioning for analytics
    .key_strategy(KeyGenerationStrategy::HivePartitioned)

    .build()?
```

---

## Disaster Recovery

### 1. Resume Token Persistence

Use Redis for distributed state:

```rust
use rigatoni_stores::redis::{RedisStore, RedisConfig};
use std::time::Duration;

// Configure Redis with connection pooling and TTL
let redis_config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .pool_size(10)
    .ttl(Duration::from_secs(14 * 24 * 60 * 60))  // 14 days
    .max_retries(3)
    .build()?;

let store = RedisStore::new(redis_config).await?;
let pipeline = Pipeline::with_store(config, destination, store).await?;
```

**Redis Configuration for Production:**

- Use **TLS** for encryption: `rediss://` scheme
- Set **TTL** to prevent unbounded growth (7-30 days recommended)
- Configure **pool size** based on concurrent pipelines (2× pipeline count)
- Enable **Redis AUTH** for authentication
- Use **Redis Sentinel** for high availability

### 2. Backup Resume Tokens

```bash
# Redis backup
redis-cli SAVE

# Copy backup
scp /var/lib/redis/dump.rdb backup-server:/backups/
```

### 3. Recovery Procedure

1. Stop pipeline
2. Restore Redis from backup
3. Start pipeline (resumes from last checkpoint)

---

## Troubleshooting

### High Memory Usage

**Symptoms:**
- Container OOM kills
- Slow performance

**Solutions:**
1. Reduce `batch_size`
2. Reduce `channel_buffer_size`
3. Increase container memory limits

### Pipeline Lag

**Symptoms:**
- Events not processed in real-time
- Growing backlog

**Solutions:**
1. Increase `num_workers`
2. Increase `batch_size`
3. Scale horizontally (more instances)

### S3 Throttling

**Symptoms:**
- Frequent 503 errors
- Slow uploads

**Solutions:**
1. Use S3 Transfer Acceleration
2. Increase retry delays
3. Use prefix sharding

---

## Monitoring Checklist

- [ ] CPU and memory metrics
- [ ] Events processed per second
- [ ] Batch write latency
- [ ] Error rate
- [ ] Retry count
- [ ] Resume token age
- [ ] S3 upload latency
- [ ] MongoDB connection health

---

## Next Steps

- **[Monitoring and Observability](monitoring)** - Set up comprehensive monitoring
- **[Error Handling](error-handling)** - Handle failures gracefully
- **[Testing Strategies](testing)** - Test your pipelines thoroughly
