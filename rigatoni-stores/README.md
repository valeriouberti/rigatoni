# rigatoni-stores

State store implementations for Rigatoni ETL framework - persist resume tokens for fault tolerance.

[![Crates.io](https://img.shields.io/crates/v/rigatoni-stores.svg)](https://crates.io/crates/rigatoni-stores)
[![Documentation](https://docs.rs/rigatoni-stores/badge.svg)](https://docs.rs/rigatoni-stores)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE)

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

### Redis Store (Coming Soon)

- **Distributed** - Share state across multiple pipeline instances
- **Highly available** - Redis clustering support
- **Production-ready** - For multi-instance deployments

## Installation

```toml
[dependencies]
rigatoni-stores = { version = "0.1", features = ["memory", "file"] }
```

### Available Features

- `memory` - In-memory store (enabled by default)
- `file` - File-based store (enabled by default)
- `redis-store` - Redis store (coming soon)
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

### Redis Store (Coming Soon)

```rust
use rigatoni_stores::redis::RedisStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = RedisStore::new("redis://localhost:6379").await?;

    // Use with Rigatoni pipeline for distributed state
    // let pipeline = Pipeline::with_store(config, destination, store).await?;

    Ok(())
}
```

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
Use **Redis Store** (coming soon) for distributed state across instances

## Documentation

- [Getting Started](https://valeriouberti.github.io/rigatoni/getting-started)
- [Architecture Documentation](https://valeriouberti.github.io/rigatoni/architecture)
- [API Documentation](https://docs.rs/rigatoni-stores)
- [Main Repository](https://github.com/valeriouberti/rigatoni)

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
