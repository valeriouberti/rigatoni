# rigatoni-stores

State store implementations for Rigatoni ETL framework - persist resume tokens for fault tolerance.

[![Crates.io](https://img.shields.io/crates/v/rigatoni-stores.svg)](https://crates.io/crates/rigatoni-stores)
[![Documentation](https://docs.rs/rigatoni-stores/badge.svg)](https://docs.rs/rigatoni-stores)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../LICENSE)

## Overview

State store implementations for persisting MongoDB change stream resume tokens, enabling fault-tolerant ETL pipelines with exactly-once or at-least-once semantics.

## Supported Stores

### Memory Store (Available)

- **Fast** - In-memory HashMap for development/testing
- **Thread-safe** - Uses `Arc<RwLock<HashMap>>`
- **No persistence** - Data lost on restart

### File Store (Available)

- **Persistent** - JSON files on disk
- **Human-readable** - Easy to inspect and debug
- **One file per collection** - Organized storage

### Redis Store (Available)

- **Distributed** - Share state across multiple pipeline instances
- **Connection pooling** - Efficient connection management with deadpool
- **Production-ready** - For multi-instance deployments
- **TTL support** - Optional token expiration
- **Retry logic** - Automatic retries with exponential backoff

## Installation

```toml
[dependencies]
rigatoni-stores = { version = "0.1", features = ["memory", "file", "redis-store"] }
```

### Available Features

- `memory` - In-memory store (enabled by default)
- `file` - File-based store (enabled by default)
- `redis-store` - Redis store with connection pooling and retry logic
- `all-stores` - All store implementations

## Quick Start

### Memory Store (Development/Testing)

```rust
use rigatoni_stores::memory::MemoryStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    // Use with Rigatoni pipeline
    // let pipeline = Pipeline::with_store(config, destination, store).await?;

    Ok(())
}
```

### File Store (Persistent)

```rust
use rigatoni_stores::file::FileStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Stores resume tokens in ./state/ directory
    let store = FileStore::new("./state").await?;

    // Use with Rigatoni pipeline
    // let pipeline = Pipeline::with_store(config, destination, store).await?;

    Ok(())
}
```

### Redis Store (Distributed)

```rust
use rigatoni_stores::redis::{RedisStore, RedisConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure Redis with connection pooling and TTL
    let config = RedisConfig::builder()
        .url("redis://localhost:6379")
        .pool_size(10)
        .ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
        .max_retries(3)
        .build()?;

    let store = RedisStore::new(config).await?;

    // Use with Rigatoni pipeline for distributed state
    // let pipeline = Pipeline::with_store(config, destination, store).await?;

    Ok(())
}
```

#### Redis Configuration Options

- `url` - Redis connection URL (supports `redis://` and `rediss://` schemes)
- `pool_size` - Connection pool size (default: 10)
- `ttl` - Optional expiration time for resume tokens (recommended: 7-30 days)
- `max_retries` - Maximum retry attempts for transient errors (default: 3)
- `connection_timeout` - Connection timeout duration (default: 5 seconds)

**Note**: Redis Cluster mode is not currently implemented. Use Redis Sentinel for high availability.

## State Store Trait

All stores implement the `StateStore` trait from `rigatoni-core`:

```rust
use rigatoni_core::store::StateStore;

#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save a resume token for a collection
    async fn save_resume_token(&self, collection: &str, token: Document)
        -> Result<(), StateError>;

    /// Load a resume token for a collection
    async fn load_resume_token(&self, collection: &str)
        -> Result<Option<Document>, StateError>;

    /// Clear a resume token for a collection
    async fn clear_resume_token(&self, collection: &str)
        -> Result<(), StateError>;
}
```

## Custom Store Implementation

Implement your own store for custom backends:

```rust
use rigatoni_core::store::{StateStore, StateError};
use async_trait::async_trait;
use bson::Document;

pub struct CustomStore {
    // Your storage backend
}

#[async_trait]
impl StateStore for CustomStore {
    async fn save_resume_token(&self, collection: &str, token: Document)
        -> Result<(), StateError>
    {
        // Your implementation
        Ok(())
    }

    async fn load_resume_token(&self, collection: &str)
        -> Result<Option<Document>, StateError>
    {
        // Your implementation
        Ok(None)
    }

    async fn clear_resume_token(&self, collection: &str)
        -> Result<(), StateError>
    {
        // Your implementation
        Ok(())
    }
}
```

## Use Cases

### Development/Testing
Use **Memory Store** for fast iteration without persistence

### Single-Instance Production
Use **File Store** for simple, reliable persistence

### Multi-Instance Production
Use **Redis Store** for distributed state across pipeline instances with:
- Shared resume tokens across multiple workers
- Connection pooling for efficient Redis usage
- Automatic retry logic for transient failures
- Optional TTL to prevent unbounded growth

## Documentation

- [Getting Started](https://valeriouberti.github.io/rigatoni/getting-started)
- [Architecture Documentation](https://valeriouberti.github.io/rigatoni/architecture)
- [API Documentation](https://docs.rs/rigatoni-stores)
- [Main Repository](https://github.com/valeriouberti/rigatoni)

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](../LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).
