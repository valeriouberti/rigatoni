---
layout: default
title: API Reference
nav_order: 6
permalink: /api
description: "Complete API documentation for Rigatoni."
---

# API Reference
{: .no_toc }

Complete API documentation for all Rigatoni crates.
{: .fs-6 .fw-300 }

---

## Rust API Documentation

The complete Rust API documentation is hosted on **docs.rs**:

### Core Crates

- **[rigatoni-core](https://docs.rs/rigatoni-core)** - Core traits, pipeline orchestration, and MongoDB integration
- **[rigatoni-destinations](https://docs.rs/rigatoni-destinations)** - Destination implementations (S3)
- **[rigatoni-stores](https://docs.rs/rigatoni-stores)** - State store implementations

---

## Quick Reference

### Pipeline

The main orchestrator for ETL workflows.

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};

// Create pipeline
let config = PipelineConfig::builder()
    .mongodb_uri("mongodb://localhost:27017")
    .database("mydb")
    .collections(vec!["users"])
    .build()?;

let pipeline = Pipeline::new(config, destination).await?;

// Run pipeline
pipeline.run().await?;
```

[Full API →](https://docs.rs/rigatoni-core/latest/rigatoni_core/pipeline/struct.Pipeline.html)

---

### Destination Trait

All destinations implement this trait.

```rust
use rigatoni_core::destination::Destination;

#[async_trait]
pub trait Destination: Send + Sync {
    async fn write_batch(&mut self, events: &[ChangeEvent])
        -> Result<(), DestinationError>;

    async fn flush(&mut self) -> Result<(), DestinationError>;

    async fn close(&mut self) -> Result<(), DestinationError>;

    fn metadata(&self) -> DestinationMetadata;

    fn buffered_count(&self) -> usize;
}
```

[Full API →](https://docs.rs/rigatoni-core/latest/rigatoni_core/destination/trait.Destination.html)

---

### S3 Destination

AWS S3 destination with multiple formats and compression.

```rust
use rigatoni_destinations::s3::{S3Config, S3Destination};

let config = S3Config::builder()
    .bucket("my-bucket")
    .region("us-east-1")
    .prefix("data")
    .build()?;

let destination = S3Destination::new(config).await?;
```

[Full API →](https://docs.rs/rigatoni-destinations/latest/rigatoni_destinations/s3/index.html)

---

### Change Event

Represents a MongoDB change stream event.

```rust
use rigatoni_core::event::ChangeEvent;

pub struct ChangeEvent {
    pub resume_token: Document,
    pub operation: OperationType,
    pub namespace: Namespace,
    pub full_document: Option<Document>,
    pub document_key: Option<Document>,
    pub update_description: Option<UpdateDescription>,
    pub cluster_time: DateTime<Utc>,
}
```

[Full API →](https://docs.rs/rigatoni-core/latest/rigatoni_core/event/struct.ChangeEvent.html)

---

### State Store Trait

For persisting resume tokens.

```rust
use rigatoni_core::store::StateStore;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_resume_token(&self, collection: &str, token: Document)
        -> Result<(), StateError>;

    async fn load_resume_token(&self, collection: &str)
        -> Result<Option<Document>, StateError>;

    async fn clear_resume_token(&self, collection: &str)
        -> Result<(), StateError>;
}
```

[Full API →](https://docs.rs/rigatoni-core/latest/rigatoni_core/store/trait.StateStore.html)

---

## Generating Local Documentation

You can generate and browse the API documentation locally:

```bash
# Generate docs for all workspace members
cargo doc --no-deps --all-features --open

# Generate docs for specific crate
cargo doc -p rigatoni-core --open

# Include private items (for development)
cargo doc --document-private-items --open
```

---

## Module Organization

### rigatoni-core

```
rigatoni_core
├── pipeline        - Pipeline orchestration
├── destination     - Destination trait
├── event           - Change event types
├── store           - State store trait
└── error           - Error types
```

### rigatoni-destinations

```
rigatoni_destinations
└── s3              - S3 destination
    ├── destination - S3Destination impl
    ├── config      - S3Config and builder
    └── key_gen     - Key generation strategies
```

### rigatoni-stores

```
rigatoni_stores
├── memory          - In-memory store
├── file            - File-based store
└── redis           - Redis store
```

---

## Examples

Each crate includes examples in the `examples/` directory:

### Core Examples

```bash
# View available examples
ls rigatoni-core/examples/

# Run example
cargo run --example example_name -p rigatoni-core
```

### S3 Examples

```bash
# Basic S3 usage
cargo run --example s3_basic --features s3,json

# Advanced S3 features
cargo run --example s3_advanced --all-features

# With compression
cargo run --example s3_with_compression --features s3,gzip
```

---

## Changelog

See [CHANGELOG.md](https://github.com/valeriouberti/rigatoni/blob/main/CHANGELOG.md) for version history and breaking changes.

---

## Need Help?

- **User Guides** - [Browse guides](../guides/) for task-specific help
- **Getting Started** - [Quick start guide](../getting-started)
- **Architecture** - [System design docs](../architecture)
- **Issues** - [Report bugs](https://github.com/valeriouberti/rigatoni/issues)
