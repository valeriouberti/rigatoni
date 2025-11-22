---
layout: default
title: Redis State Store Configuration
parent: Guides
nav_order: 2
description: "Complete guide to configuring the Redis state store."
---

# Redis State Store Configuration Guide
{: .no_toc }

Learn how to configure Redis for distributed state management across multiple pipeline instances.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

The Redis state store enables distributed state management for Rigatoni pipelines, allowing multiple pipeline instances to share resume tokens and coordinate processing across collections.

⚠️ **IMPORTANT LIMITATION: No Distributed Locking**

While Redis state store supports multiple pipeline instances, **different instances MUST watch different collections**. The current implementation does NOT include distributed locking, which means:

- ❌ **Multiple instances on the same collection will cause duplicate processing**
- ❌ **Resume token race conditions** - Last write wins (simple SET, no SETNX)
- ❌ **No coordination for concurrent writes**

**Safe Multi-Instance Pattern:**
```rust
// Instance 1 - watches users and orders
let config1 = PipelineConfig::builder()
    .collections(vec!["users".to_string(), "orders".to_string()])
    .build()?;

// Instance 2 - watches products (different collections)
let config2 = PipelineConfig::builder()
    .collections(vec!["products".to_string()])
    .build()?;
```

See [Production Deployment Guide](production-deployment.md#multi-instance-deployment) for details and [internal-docs/issues/multi-instance-same-collection-support.md](../../internal-docs/issues/multi-instance-same-collection-support.md) for planned distributed locking support.

**Key Features:**
- **Connection pooling** - Efficient connection management with deadpool-redis
- **Automatic retries** - Exponential backoff for transient failures
- **TTL support** - Optional token expiration to prevent unbounded growth
- **Production-ready** - SCAN-based listing, credential masking, retry logic

---

## Basic Configuration

The minimal Redis configuration requires only a connection URL:

```rust
use rigatoni_stores::redis::{RedisStore, RedisConfig};

let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .build()?;

let store = RedisStore::new(config).await?;
```

---

## Configuration Options

### Required Fields

#### `url(impl Into<String>)`
{: .text-purple-000 }

The Redis connection URL.

**Supported schemes:**
- `redis://` - Unencrypted connection
- `rediss://` - TLS-encrypted connection (recommended for production)

**URL formats:**
```rust
// Basic connection
.url("redis://localhost:6379")

// With password
.url("redis://:password@localhost:6379")

// With username and password
.url("redis://username:password@localhost:6379")

// Specific database
.url("redis://localhost:6379/0")

// TLS connection
.url("rediss://redis.example.com:6380")

// Redis Sentinel (for high availability)
.url("redis://sentinel1:26379,sentinel2:26379")
```

**Example:**
```rust
let config = RedisConfig::builder()
    .url("rediss://:mypassword@redis.prod.example.com:6380")
    .build()?;
```

---

### Optional Fields

#### `pool_size(usize)`
{: .text-green-100 }

**Default:** `10`

The connection pool size for concurrent operations.

**Guidelines:**
- Small deployments (1-5 pipelines): 5-10 connections
- Medium deployments (5-20 pipelines): 10-20 connections
- Large deployments (20+ pipelines): 20-50 connections

**Formula:** `pool_size = concurrent_pipelines × 2`

```rust
let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .pool_size(20)  // For ~10 concurrent pipelines
    .build()?;
```

#### `ttl(Duration)`
{: .text-green-100 }

**Default:** `None` (no expiration)

Time-to-live for resume tokens. Tokens automatically expire after this duration.

**Recommended values:**
- Development: No TTL or 1 day
- Staging: 7 days
- Production: 7-30 days

```rust
use std::time::Duration;

let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .ttl(Duration::from_secs(7 * 24 * 60 * 60))  // 7 days
    .build()?;
```

**Why use TTL?**
- ✅ Prevents unbounded Redis memory growth
- ✅ Automatic cleanup of stale tokens
- ✅ Production-safe for long-running pipelines
- ❌ Tokens expire if pipeline paused > TTL duration

#### `max_retries(u32)`
{: .text-green-100 }

**Default:** `3`

Maximum number of retry attempts for transient Redis errors.

```rust
let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .max_retries(5)  // More retries for flaky networks
    .build()?;
```

**Retry schedule:**
- Attempt 1: Immediate
- Attempt 2: 100ms delay
- Attempt 3: 200ms delay
- Attempt 4: 400ms delay
- Attempt 5: Fail

#### `connection_timeout(Duration)`
{: .text-green-100 }

**Default:** `5 seconds`

Timeout for establishing Redis connections.

```rust
use std::time::Duration;

let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .connection_timeout(Duration::from_secs(10))
    .build()?;
```

#### `cluster_mode(bool)`
{: .text-red-200 }

**Default:** `false`
**Status:** ⚠️ **Not Implemented**

Redis Cluster mode is not currently supported. Setting to `true` logs a warning but doesn't enable cluster functionality.

**For high availability, use Redis Sentinel instead.**

```rust
// ❌ Cluster mode not supported
let config = RedisConfig::builder()
    .url("redis://node1:6379,redis://node2:6379")
    .cluster_mode(true)  // This will log a warning
    .build()?;

// ✅ Use Redis Sentinel instead
let config = RedisConfig::builder()
    .url("redis://sentinel1:26379,sentinel2:26379")
    .build()?;
```

---

## Complete Example

```rust
use rigatoni_stores::redis::{RedisStore, RedisConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production-ready configuration
    let config = RedisConfig::builder()
        .url("rediss://:password@redis.prod.example.com:6380")
        .pool_size(20)
        .ttl(Duration::from_secs(14 * 24 * 60 * 60))  // 14 days
        .max_retries(5)
        .connection_timeout(Duration::from_secs(10))
        .build()?;

    let store = RedisStore::new(config).await?;

    // Use with pipeline
    let pipeline = Pipeline::with_store(
        pipeline_config,
        destination,
        store
    ).await?;

    pipeline.run().await?;

    Ok(())
}
```

---

## Production Deployment

### Security Best Practices

1. **Use TLS encryption**
   ```rust
   .url("rediss://redis.example.com:6380")  // Note: rediss:// not redis://
   ```

2. **Enable Redis AUTH**
   ```rust
   .url("rediss://:strong_password@redis.example.com:6380")
   ```

3. **Use private network**
   - Deploy Redis in private VPC
   - No public internet access
   - Firewall rules for pipeline IPs only

4. **Credential management**
   ```rust
   use std::env;

   let redis_url = env::var("REDIS_URL")?;
   let config = RedisConfig::builder()
       .url(redis_url)  // Load from environment
       .build()?;
   ```

### High Availability with Redis Sentinel

For production deployments, use Redis Sentinel for automatic failover:

```rust
// Multiple Sentinel nodes
let config = RedisConfig::builder()
    .url("redis://sentinel1:26379,sentinel2:26379,sentinel3:26379")
    .pool_size(20)
    .ttl(Duration::from_secs(14 * 24 * 60 * 60))
    .build()?;
```

**Sentinel benefits:**
- Automatic master failover
- Health monitoring
- Client automatic reconnection

### Monitoring

Track these metrics in production:

**Redis Metrics:**
- Connection pool utilization
- Command latency (GET, SET, SCAN)
- Memory usage
- Eviction count (should be 0 with TTL)
- Hit rate (should be >90%)

**Application Metrics:**
- Save token latency (P50, P95, P99)
- Get token latency
- Retry count per operation
- Error rate by type

---

## Common Configurations

### Development/Testing

```rust
let config = RedisConfig::builder()
    .url("redis://localhost:6379")
    .pool_size(5)
    .build()?;
```

### Staging

```rust
let config = RedisConfig::builder()
    .url("redis://:password@redis-staging:6379")
    .pool_size(10)
    .ttl(Duration::from_secs(7 * 24 * 60 * 60))  // 7 days
    .build()?;
```

### Production

```rust
let config = RedisConfig::builder()
    .url("rediss://:password@redis-prod:6380")
    .pool_size(50)
    .ttl(Duration::from_secs(14 * 24 * 60 * 60))  // 14 days
    .max_retries(5)
    .connection_timeout(Duration::from_secs(10))
    .build()?;
```

### Multi-Region (Cross-Region Latency)

```rust
let config = RedisConfig::builder()
    .url("rediss://:password@redis.us-east-1.example.com:6380")
    .pool_size(30)
    .ttl(Duration::from_secs(30 * 24 * 60 * 60))  // 30 days (longer for less churn)
    .max_retries(8)  // More retries for network issues
    .connection_timeout(Duration::from_secs(15))  // Higher timeout
    .build()?;
```

---

## Troubleshooting

### Connection Failures

**Symptom:** `Failed to connect to Redis`

**Solutions:**
1. Verify Redis is running: `redis-cli PING`
2. Check firewall rules allow connection
3. Verify URL format and credentials
4. Check Redis logs for errors

### High Retry Rate

**Symptom:** Many retry warnings in logs

**Solutions:**
1. Check Redis CPU usage (should be <80%)
2. Measure network latency to Redis
3. Increase connection pool size
4. Check for Redis memory pressure

### Timeout Errors

**Symptom:** `Connection timeout` or `Operation timed out`

**Solutions:**
1. Increase `connection_timeout` duration
2. Check Redis `maxclients` limit
3. Verify network connectivity
4. Check Redis slow log: `redis-cli SLOWLOG GET 10`

### Memory Growth

**Symptom:** Redis memory continuously growing

**Solutions:**
1. Verify TTL is set: `redis-cli TTL rigatoni:resume_token:users`
2. Check for orphaned keys: `redis-cli KEYS rigatoni:resume_token:*`
3. Monitor `used_memory` in `redis-cli INFO memory`
4. Set appropriate TTL or implement cleanup

---

## Key Pattern

Resume tokens are stored with this key pattern:

```
rigatoni:resume_token:{collection_name}
```

**Examples:**
```
rigatoni:resume_token:users
rigatoni:resume_token:orders
rigatoni:resume_token:products
```

### Listing All Tokens

```bash
# List all Rigatoni resume token keys
redis-cli KEYS "rigatoni:resume_token:*"

# Get a specific token
redis-cli GET "rigatoni:resume_token:users"

# Check TTL
redis-cli TTL "rigatoni:resume_token:users"

# Delete a token
redis-cli DEL "rigatoni:resume_token:users"
```

---

## Advanced Topics

### TTL Semantics

**Important:** Reading a token does NOT refresh its TTL. Only writing refreshes the TTL.

```rust
// Save token - sets 7-day TTL
store.save_resume_token("users", token).await?;
// Redis: SETEX rigatoni:resume_token:users 604800 <bytes>
// TTL: 7 days

// Read token - TTL unchanged
store.get_resume_token("users").await?;
// Redis: GET rigatoni:resume_token:users
// TTL: Still 7 days (not refreshed)

// Save again - resets TTL to 7 days
store.save_resume_token("users", new_token).await?;
// Redis: SETEX rigatoni:resume_token:users 604800 <bytes>
// TTL: 7 days (reset)
```

### Connection Pooling

The store uses `deadpool-redis` for connection pooling:

- Connections are reused across operations
- Pool automatically recycles stale connections
- Timeouts prevent connection leaks
- Thread-safe for concurrent access

### Retry Logic

Automatic retries for transient errors:

**Retryable errors:**
- `IoError` - Network failures
- `ResponseError` - Generic Redis errors

**Non-retryable errors:**
- Serialization failures
- Invalid URL
- Authentication failures

### Security: Credential Masking

Credentials are automatically masked in debug output:

```rust
let config = RedisConfig::builder()
    .url("redis://user:password@localhost:6379")
    .build()?;

println!("{:?}", config);
// Output: RedisConfig { url: "redis://***:***@localhost:6379", ... }
```

This prevents password leaks in logs and error messages.

---

## See Also

- [Production Deployment Guide](production-deployment.md)
- [Architecture Documentation](../architecture.md)
- [API Documentation](https://docs.rs/rigatoni-stores)
- [Redis Store Technical Explanation](../../internal-docs/stores/redis_tech_explain.md)
