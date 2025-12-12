# Multi-Instance Deployment Guide

This guide explains how to deploy multiple Rigatoni instances for horizontal scaling and high availability using distributed locking.

## Overview

Rigatoni supports running multiple instances watching the same MongoDB collections without duplicate event processing. This is achieved through **distributed locking** using Redis.

### How It Works

1. Each Rigatoni instance generates a unique owner ID (`{hostname}-{uuid}`)
2. Before processing a collection, an instance acquires an exclusive lock in Redis
3. Only the lock holder processes events for that collection
4. Locks are automatically refreshed (heartbeat) to prevent expiry
5. If an instance crashes, its locks expire after TTL, and other instances take over

```
Instance 1                    Instance 2                    Instance 3
    |                             |                             |
    v                             v                             v
Try acquire lock          Try acquire lock          Try acquire lock
"mydb:users"              "mydb:users"              "mydb:users"
    |                             |                             |
    v                             v                             v
SUCCESS                      FAIL (locked)            FAIL (locked)
    |                             |                             |
    v                             v                             v
Process events           Try "mydb:orders"        Try "mydb:products"
from "users"                   SUCCESS                  SUCCESS
```

## Prerequisites

- Redis server (standalone, Sentinel, or Cluster)
- MongoDB replica set (required for change streams)
- Multiple compute instances (VMs, containers, pods)

## Configuration

### Basic Multi-Instance Setup

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig, DistributedLockConfig};
use rigatoni_stores::redis::{RedisStore, RedisConfig};
use std::time::Duration;

// Configure Redis store
let redis_config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .pool_size(10)
    .ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days for resume tokens
    .build()?;

let store = RedisStore::new(redis_config).await?;

// Configure pipeline with distributed locking
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0")
    .database("mydb")
    .watch_collections(vec![
        "users".to_string(),
        "orders".to_string(),
        "products".to_string(),
    ])
    .distributed_lock(DistributedLockConfig {
        enabled: true,
        ttl: Duration::from_secs(30),
        refresh_interval: Duration::from_secs(10),
        retry_interval: Duration::from_secs(5),
    })
    .build()?;

let mut pipeline = Pipeline::new(config, store, destination).await?;
pipeline.start().await?;
```

### Lock Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `enabled` | `true` | Enable/disable distributed locking |
| `ttl` | `30s` | Lock expiry time (failover delay) |
| `refresh_interval` | `10s` | How often to refresh the lock (heartbeat) |
| `retry_interval` | `5s` | How often to retry acquiring unowned locks |

### Tuning Guidelines

**Lock TTL (`ttl`)**
- Short (10s): Fast failover, requires more frequent refreshes
- Long (60s): Slow failover, less Redis traffic
- **Recommended**: 30 seconds

**Refresh Interval (`refresh_interval`)**
- Must be less than `ttl / 2` to prevent accidental expiry
- **Rule of thumb**: `refresh_interval < ttl / 3`
- **Recommended**: `ttl / 3` (e.g., 10s for 30s TTL)

**Retry Interval (`retry_interval`)**
- How quickly instances claim available collections
- Short (1s): Fast claiming, more Redis traffic
- Long (30s): Less overhead, slower distribution
- **Recommended**: 5-10 seconds

## Kubernetes Deployment

### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rigatoni
spec:
  replicas: 3  # Run 3 instances
  selector:
    matchLabels:
      app: rigatoni
  template:
    metadata:
      labels:
        app: rigatoni
    spec:
      containers:
      - name: rigatoni
        image: your-registry/rigatoni:latest
        env:
        - name: MONGODB_URI
          value: "mongodb://mongo:27017/?replicaSet=rs0"
        - name: REDIS_URL
          value: "redis://redis:6379"
        - name: RUST_LOG
          value: "info,rigatoni_core=info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rigatoni-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rigatoni
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

### Pod Disruption Budget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: rigatoni-pdb
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app: rigatoni
```

## Docker Compose

```yaml
version: '3.8'

services:
  rigatoni-1:
    image: your-registry/rigatoni:latest
    environment:
      - MONGODB_URI=mongodb://mongo:27017/?replicaSet=rs0
      - REDIS_URL=redis://redis:6379
      - INSTANCE_NAME=rigatoni-1
    depends_on:
      - mongo
      - redis

  rigatoni-2:
    image: your-registry/rigatoni:latest
    environment:
      - MONGODB_URI=mongodb://mongo:27017/?replicaSet=rs0
      - REDIS_URL=redis://redis:6379
      - INSTANCE_NAME=rigatoni-2
    depends_on:
      - mongo
      - redis

  rigatoni-3:
    image: your-registry/rigatoni:latest
    environment:
      - MONGODB_URI=mongodb://mongo:27017/?replicaSet=rs0
      - REDIS_URL=redis://redis:6379
      - INSTANCE_NAME=rigatoni-3
    depends_on:
      - mongo
      - redis

  mongo:
    image: mongo:7.0
    command: --replSet rs0
    ports:
      - "27017:27017"

  redis:
    image: redis:7.0
    ports:
      - "6379:6379"
```

## Watch Level Considerations

### Collection-Level (`watch_collections`)

- Each collection gets its own lock
- Collections distributed across instances
- **Best for**: Many collections, parallel processing

```rust
.watch_collections(vec!["users", "orders", "products", "logs"])
```

### Database-Level (`watch_database`)

- Single lock for entire database
- Only one instance processes all events
- **Best for**: Few collections, low throughput, or when order matters

```rust
.watch_database()
```

### Deployment-Level (`watch_deployment`)

- Single lock for entire MongoDB deployment
- Only one instance processes all events
- **Best for**: Multi-tenant with database-per-tenant

```rust
.watch_deployment()
```

## Failure Scenarios

### Instance Crash

1. Instance crashes without releasing locks
2. Locks expire after TTL (default: 30 seconds)
3. Other instances acquire expired locks
4. Processing resumes from last checkpoint

**Data loss**: None (at-least-once semantics preserved)
**Downtime**: Maximum TTL duration per collection

### Redis Failure

1. Lock acquisition fails
2. Instance cannot start workers
3. **Better to stop than process duplicates**

**Recommendation**: Use Redis Sentinel or Cluster for HA

### Network Partition

1. Instance loses connection to Redis
2. Cannot refresh locks
3. Locks expire, other instances take over
4. **Duplicate window**: Up to TTL duration

**Mitigation**: Use shorter TTL in partition-prone environments

## Monitoring

### Key Metrics

| Metric | Description |
|--------|-------------|
| `rigatoni_locks_held_total` | Number of locks held by this instance |
| `rigatoni_lock_acquisitions_total` | Successful lock acquisitions |
| `rigatoni_lock_acquisition_failures_total` | Failed lock attempts (by reason) |
| `rigatoni_locks_lost_total` | Locks lost (expired or stolen) |
| `rigatoni_lock_refreshes_total` | Successful lock refreshes |
| `rigatoni_locks_released_total` | Gracefully released locks |

### Prometheus Alerts

```yaml
groups:
- name: rigatoni-locking
  rules:
  - alert: RigatoniLockLost
    expr: increase(rigatoni_locks_lost_total[5m]) > 0
    for: 1m
    labels:
      severity: warning
    annotations:
      summary: "Rigatoni instance lost locks"
      description: "Instance {{ $labels.instance }} lost {{ $value }} locks"

  - alert: RigatoniNoLocksHeld
    expr: rigatoni_locks_held_total == 0
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Rigatoni instance holds no locks"
      description: "Instance {{ $labels.instance }} is not processing any collections"
```

## Best Practices

### 1. Use Graceful Shutdown

Always stop the pipeline gracefully to release locks immediately:

```rust
// Handle shutdown signals
tokio::signal::ctrl_c().await?;
pipeline.stop().await?;  // Releases locks
```

### 2. Monitor Lock Distribution

Ensure locks are evenly distributed across instances. If one instance holds all locks, others are idle.

### 3. Size Instances Appropriately

- More instances = more parallelism (up to number of collections)
- Instances beyond collection count provide only failover benefit

### 4. Use Health Checks

```rust
// In your health check endpoint
if pipeline.is_running().await {
    let stats = pipeline.stats().await;
    // Return healthy with stats
} else {
    // Return unhealthy
}
```

### 5. Set Resource Limits

Prevent runaway resource usage in Kubernetes:

```yaml
resources:
  limits:
    memory: "512Mi"
    cpu: "500m"
```

## Troubleshooting

### No Instance Acquiring Locks

1. Check Redis connectivity
2. Verify Redis URL configuration
3. Check Redis logs for errors

### Uneven Lock Distribution

1. Restart instances with fewer locks
2. Check if some collections have longer processing times
3. Consider splitting large collections

### Frequent Lock Loss

1. Check network stability
2. Consider increasing TTL
3. Check Redis latency
4. Monitor instance CPU/memory

### Duplicate Events

1. Verify locking is enabled (`distributed_lock.enabled = true`)
2. Check that all instances use Redis (not MemoryStore)
3. Verify all instances connect to the same Redis

## Example: Running Locally

```bash
# Terminal 1 - Start infrastructure
docker-compose up -d mongo redis

# Terminal 2 - Instance 1
INSTANCE_NAME=instance-1 cargo run --example multi_instance_redis

# Terminal 3 - Instance 2
INSTANCE_NAME=instance-2 cargo run --example multi_instance_redis

# Terminal 4 - Generate events
docker exec mongodb mongosh testdb --eval '
  db.users.insertOne({name: "Alice"});
  db.orders.insertOne({product: "Widget"});
'
```

Watch how events are distributed between instances!
